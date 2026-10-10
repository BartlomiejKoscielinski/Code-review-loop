//! Local changes: the working tree (staged, unstaged and untracked files)
//! compared with a commit (ADR 0006).
//!
//! Git itself turns the working tree into a tree object: a copy of the index
//! in Lupa's cache gets `git add -A`, then `git write-tree`. Objects go to an
//! object directory in the cache and the repository's own objects are read
//! through alternates, so the developer's index and object store stay
//! untouched. Ignore rules, attributes, `core.autocrlf`, clean filters,
//! symlinks, submodules and conflicts all behave exactly as in Git, and
//! `git diff-tree -M` compares the snapshot with the base.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{ChangeKind, ChangedFile, EntryKind, FileVersion, GitError, ObjectId};

/// Variables that would point Git at another repository, index or object
/// store, or inject configuration from a parent `git -c` call.
const REDIRECTING_VARIABLES: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

/// Working tree of a repository, snapshotted into `cache_dir`.
pub struct WorkTree {
    /// Top-level directory of the working tree.
    top: PathBuf,
    /// The repository's index (copied, never written).
    index: PathBuf,
    /// The repository's object directory (read through alternates).
    repo_objects: PathBuf,
    /// Lupa's object directory for snapshot objects.
    objects: PathBuf,
    /// Lupa's copy of the index.
    snapshot_index: PathBuf,
}

impl WorkTree {
    /// Opens the working tree that contains `path`. `cache_dir` must be
    /// outside the working tree; it is created if missing.
    pub fn new(path: &Path, cache_dir: &Path) -> Result<Self, GitError> {
        let top = PathBuf::from(output(
            plain_git(path).args(["rev-parse", "--show-toplevel"]),
            "rev-parse",
        )?);
        let git_path = |name: &str| -> Result<PathBuf, GitError> {
            Ok(PathBuf::from(output(
                plain_git(&top).args(["rev-parse", "--path-format=absolute", "--git-path", name]),
                "rev-parse",
            )?))
        };
        let index = git_path("index")?;
        let repo_objects = git_path("objects")?;
        if is_inside(cache_dir, &top) {
            return Err(GitError::Command {
                command: "snapshot".to_owned(),
                detail: format!(
                    "cache directory {} is inside the working tree {}",
                    cache_dir.display(),
                    top.display()
                ),
            });
        }
        let objects = cache_dir.join("objects");
        std::fs::create_dir_all(&objects).map_err(|error| io_failed("create cache", &error))?;
        Ok(Self {
            snapshot_index: cache_dir.join("index"),
            top,
            index,
            repo_objects,
            objects,
        })
    }

    /// Files whose working tree content differs from `base` (`None` = the
    /// empty tree), with renames as `git diff -M`. Sorted by path.
    ///
    /// The snapshot reflects content, not staging: a file removed from the
    /// index but still on disk with the same content is unchanged.
    pub fn changed_files(&self, base: Option<ObjectId>) -> Result<Vec<ChangedFile>, GitError> {
        let tree = self.snapshot()?;
        let base = match base {
            Some(commit) => commit.to_string(),
            None => output(
                self.git()
                    .args(["hash-object", "-t", "tree", "--stdin"])
                    .stdin(Stdio::null()),
                "hash-object",
            )?,
        };
        let raw = output_raw(
            self.git().args([
                "diff-tree",
                "-r",
                "-z",
                "-M",
                "--raw",
                "--no-abbrev",
                "--no-ext-diff",
                &base,
                &tree.to_string(),
            ]),
            "diff-tree",
        )?;
        let mut files = parse_raw(&raw)?;
        files.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(files)
    }

    /// Writes the working tree as a tree object in Lupa's store.
    pub fn snapshot(&self) -> Result<ObjectId, GitError> {
        match std::fs::copy(&self.index, &self.snapshot_index) {
            Ok(_) => {}
            // No index yet: a repository where nothing was ever staged.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::remove_file(&self.snapshot_index) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(io_failed("reset snapshot index", &error)),
                }
            }
            Err(error) => return Err(io_failed("copy index", &error)),
        }
        output(self.git().args(["add", "-A"]), "add")?;
        let tree = output(self.git().arg("write-tree"), "write-tree")?;
        tree.parse().map_err(|_| GitError::Command {
            command: "write-tree".to_owned(),
            detail: format!("unexpected output {tree}"),
        })
    }

    /// Content of a blob from the repository or from a snapshot.
    pub fn read_blob(&self, id: ObjectId) -> Result<Vec<u8>, GitError> {
        let out = self
            .git()
            .args(["cat-file", "blob", &id.to_string()])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| io_failed("cat-file", &error))?;
        if !out.status.success() {
            return Err(failed("cat-file", &out.stderr));
        }
        Ok(out.stdout)
    }

    /// `git` on the snapshot: Lupa's index and object store, the repository's
    /// objects as alternates.
    fn git(&self) -> Command {
        let mut command = plain_git(&self.top);
        command
            .env("GIT_INDEX_FILE", &self.snapshot_index)
            .env("GIT_OBJECT_DIRECTORY", &self.objects)
            .env(
                "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                quote_alternate(&self.repo_objects),
            );
        command
    }
}

/// `git -C dir` without inherited redirections.
fn plain_git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    for variable in REDIRECTING_VARIABLES {
        command.env_remove(variable);
    }
    command.env("GIT_TERMINAL_PROMPT", "0").arg("-C").arg(dir);
    command
}

/// Git splits `GIT_ALTERNATE_OBJECT_DIRECTORIES` on `;` (Windows) or `:`
/// (elsewhere) unless the entry is quoted C-style.
fn quote_alternate(path: &Path) -> OsString {
    let text = path.to_string_lossy();
    let separator = if cfg!(windows) { ';' } else { ':' };
    if !text.contains(separator) && !text.starts_with('"') {
        return path.as_os_str().to_owned();
    }
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    OsString::from(format!("\"{escaped}\""))
}

/// True when `path` is `root` or below it, ignoring case on Windows.
fn is_inside(path: &Path, root: &Path) -> bool {
    let normalize = |p: &Path| {
        let absolute = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let text = absolute.to_string_lossy().replace('\\', "/");
        if cfg!(windows) {
            text.to_lowercase()
        } else {
            text
        }
    };
    let (path, root) = (normalize(path), normalize(root));
    path == root || path.starts_with(&format!("{}/", root.trim_end_matches('/')))
}

/// Parses `git diff-tree --raw -z --no-abbrev` records. Unknown statuses or
/// modes are errors, never guesses.
fn parse_raw(raw: &[u8]) -> Result<Vec<ChangedFile>, GitError> {
    let mut fields = raw
        .split(|&byte| byte == 0)
        .filter(|field| !field.is_empty());
    let mut files = Vec::new();
    while let Some(meta) = fields.next() {
        let meta = String::from_utf8_lossy(meta);
        let bad = || failed("diff-tree", format!("unexpected record {meta}").as_bytes());
        let parts: Vec<&str> = meta.trim_start_matches(':').split(' ').collect();
        let [old_mode, new_mode, old_id, new_id, status] = parts[..] else {
            return Err(bad());
        };
        let mut path = || -> Result<String, GitError> {
            fields
                .next()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .ok_or_else(bad)
        };
        let first = path()?;
        let (kind, second) = match status.chars().next() {
            Some('A') => (ChangeKind::Added, first.clone()),
            Some('D') => (ChangeKind::Deleted, first.clone()),
            Some('M') => (ChangeKind::Modified, first.clone()),
            Some('T') => (ChangeKind::TypeChanged, first.clone()),
            Some('R') => (ChangeKind::Renamed, path()?),
            _ => return Err(bad()),
        };
        let side = |mode: &str, id: &str, path: String| -> Result<FileVersion, GitError> {
            Ok(FileVersion {
                path,
                id: id.parse().map_err(|_| bad())?,
                kind: kind_from_mode(mode).ok_or_else(bad)?,
            })
        };
        files.push(ChangedFile {
            kind,
            old: match kind {
                ChangeKind::Added => None,
                _ => Some(side(old_mode, old_id, first)?),
            },
            new: match kind {
                ChangeKind::Deleted => None,
                _ => Some(side(new_mode, new_id, second)?),
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

fn output_raw(command: &mut Command, what: &str) -> Result<Vec<u8>, GitError> {
    let out = command
        .stdin(Stdio::null())
        .output()
        .map_err(|error| io_failed(what, &error))?;
    if !out.status.success() {
        return Err(failed(what, &out.stderr));
    }
    Ok(out.stdout)
}

fn output(command: &mut Command, what: &str) -> Result<String, GitError> {
    let out = output_raw(command, what)?;
    Ok(String::from_utf8_lossy(&out).trim_end().to_owned())
}

fn io_failed(what: &str, error: &std::io::Error) -> GitError {
    GitError::Command {
        command: what.to_owned(),
        detail: error.to_string(),
    }
}

fn failed(what: &str, stderr: &[u8]) -> GitError {
    GitError::Command {
        command: what.to_owned(),
        detail: String::from_utf8_lossy(stderr).trim().to_owned(),
    }
}
