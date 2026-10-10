//! Local changes: the working tree (staged, unstaged and untracked files)
//! compared with a commit.
//!
//! Everything goes through the `git` CLI (ADR 0005), so `.gitignore`,
//! `.gitattributes`, `core.autocrlf` and clean filters such as LFS behave
//! exactly as in Git. Working tree files are stored as blobs *after* Git's
//! clean filters, in an object directory inside Lupa's cache: the diff then
//! compares like with like (a CRLF checkout of an LF blob is unchanged), and
//! nothing is written to the developer's repository.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{ChangeKind, ChangedFile, EntryKind, FileVersion, GitError, ObjectId};

/// Object id of the empty tree, used as the base of a repository without commits.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// Working tree of a repository, with blobs stored in `cache_dir`.
pub struct WorkTree {
    repo: PathBuf,
    /// Lupa's object directory for working tree blobs.
    objects: PathBuf,
    /// The repository's own object directory, read through Git's alternates.
    repo_objects: PathBuf,
}

impl WorkTree {
    /// `cache_dir` is created if missing; it must not be inside the repository.
    pub fn new(repo: &Path, cache_dir: &Path) -> Result<Self, GitError> {
        let objects = cache_dir.join("objects");
        std::fs::create_dir_all(&objects).map_err(|source| git_error("create cache", source))?;
        let repo_objects = output(
            Command::new("git").arg("-C").arg(repo).args([
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "objects",
            ]),
            "rev-parse",
        )?;
        Ok(Self {
            repo: repo.to_owned(),
            objects,
            repo_objects: PathBuf::from(repo_objects.trim_end()),
        })
    }

    /// Files in the working tree that differ from `base` (`None` = empty tree):
    /// tracked changes with renames as `git diff -M <base>`, plus untracked
    /// files that are not ignored. Sorted by path.
    pub fn changed_files(&self, base: Option<ObjectId>) -> Result<Vec<ChangedFile>, GitError> {
        let base = base.map_or_else(|| EMPTY_TREE.to_owned(), |id| id.to_string());
        let raw = output(
            &mut self.git(&[
                "diff",
                "--raw",
                "-z",
                "-M",
                "--no-ext-diff",
                "--no-textconv",
                "--abbrev=40",
                &base,
            ]),
            "diff",
        )?;
        let mut files = parse_raw(&raw)?;
        let untracked = output(
            &mut self.git(&["ls-files", "--others", "--exclude-standard", "-z"]),
            "ls-files",
        )?;
        for path in untracked.split('\0').filter(|path| !path.is_empty()) {
            files.push(ChangedFile {
                kind: ChangeKind::Added,
                old: None,
                new: Some(FileVersion {
                    path: path.to_owned(),
                    id: ObjectId::null(gix::hash::Kind::Sha1),
                    kind: EntryKind::File,
                }),
            });
        }
        self.store_new_sides(&mut files)?;
        // A file that only differs in stat information hashes to its old blob.
        files.retain(|file| match (&file.old, &file.new) {
            (Some(old), Some(new)) => {
                old.id != new.id || old.kind != new.kind || old.path != new.path
            }
            _ => true,
        });
        files.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(files)
    }

    /// Content of a blob from the repository or from the working tree store.
    pub fn read_blob(&self, id: ObjectId) -> Result<Vec<u8>, GitError> {
        let out = self
            .git(&["cat-file", "blob", &id.to_string()])
            .output()
            .map_err(|source| git_error("cat-file", source))?;
        if !out.status.success() {
            return Err(GitError::NotABlob { id });
        }
        Ok(out.stdout)
    }

    /// Hashes regular files on the new side through Git's clean filters and
    /// stores them in Lupa's object directory.
    fn store_new_sides(&self, files: &mut [ChangedFile]) -> Result<(), GitError> {
        let mut regular: Vec<&mut FileVersion> = files
            .iter_mut()
            .filter_map(|file| file.new.as_mut())
            .filter(|new| matches!(new.kind, EntryKind::File | EntryKind::Executable))
            .collect();
        if regular.is_empty() {
            return Ok(());
        }
        let paths: String = regular
            .iter()
            .map(|new| format!("{}\n", new.path))
            .collect();
        let mut child = self
            .git(&["hash-object", "-w", "--stdin-paths"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| git_error("hash-object", source))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(paths.as_bytes())
                .map_err(|source| git_error("hash-object", source))?;
        }
        let out = child
            .wait_with_output()
            .map_err(|source| git_error("hash-object", source))?;
        if !out.status.success() {
            return Err(command_failed("hash-object", &out.stderr));
        }
        let ids = String::from_utf8_lossy(&out.stdout);
        let mut ids = ids.lines();
        for new in &mut regular {
            let id = ids
                .next()
                .and_then(|line| line.parse().ok())
                .ok_or_else(|| command_failed("hash-object", b"fewer ids than paths"))?;
            new.id = id;
        }
        Ok(())
    }

    /// `git` in the repository, writing objects to Lupa's store and reading the
    /// repository's objects through alternates.
    fn git(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.repo)
            .args(args)
            .env("GIT_OBJECT_DIRECTORY", &self.objects)
            .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", &self.repo_objects)
            .env("GIT_TERMINAL_PROMPT", "0");
        command
    }
}

/// Parses `git diff --raw -z --abbrev=40` records.
fn parse_raw(raw: &str) -> Result<Vec<ChangedFile>, GitError> {
    let mut fields = raw.split('\0').filter(|field| !field.is_empty());
    let mut files = Vec::new();
    while let Some(meta) = fields.next() {
        let parts: Vec<&str> = meta.trim_start_matches(':').split(' ').collect();
        let [old_mode, new_mode, old_id, new_id, status] = parts[..] else {
            return Err(command_failed("diff", meta.as_bytes()));
        };
        let first = fields.next().unwrap_or_default().to_owned();
        let letter = status.chars().next().unwrap_or('M');
        let second = if letter == 'R' || letter == 'C' {
            fields.next().unwrap_or_default().to_owned()
        } else {
            first.clone()
        };
        let side = |mode: &str, id: &str, path: &str| -> Option<FileVersion> {
            Some(FileVersion {
                path: path.to_owned(),
                id: id.parse().ok()?,
                kind: kind_from_mode(mode)?,
            })
        };
        let old = side(old_mode, old_id, &first);
        let new = side(new_mode, new_id, &second);
        let kind = match letter {
            'A' => ChangeKind::Added,
            'D' => ChangeKind::Deleted,
            'R' => ChangeKind::Renamed,
            'T' => ChangeKind::TypeChanged,
            _ => ChangeKind::Modified,
        };
        files.push(ChangedFile {
            kind,
            old: if kind == ChangeKind::Added { None } else { old },
            new: if kind == ChangeKind::Deleted {
                None
            } else {
                new
            },
        });
    }
    Ok(files)
}

fn kind_from_mode(mode: &str) -> Option<EntryKind> {
    match mode {
        "100644" => Some(EntryKind::File),
        "100755" => Some(EntryKind::Executable),
        "120000" => Some(EntryKind::Symlink),
        "160000" => Some(EntryKind::Submodule),
        _ => None,
    }
}

fn output(command: &mut Command, what: &str) -> Result<String, GitError> {
    let out = command
        .stdin(Stdio::null())
        .output()
        .map_err(|source| git_error(what, source))?;
    if !out.status.success() {
        return Err(command_failed(what, &out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git_error(what: &str, source: std::io::Error) -> GitError {
    GitError::Command {
        command: what.to_owned(),
        detail: source.to_string(),
    }
}

fn command_failed(what: &str, stderr: &[u8]) -> GitError {
    GitError::Command {
        command: what.to_owned(),
        detail: String::from_utf8_lossy(stderr).trim().to_owned(),
    }
}
