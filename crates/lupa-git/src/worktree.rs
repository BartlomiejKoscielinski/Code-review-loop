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
//!
//! # Security
//!
//! `git add` runs the clean filters configured for the repository, which is
//! acceptable for the developer's own repository only (ADR 0006, ADR 0007).
//! Never use `WorkTree` on code from a merge request: compare MR commits
//! with [`crate::Repo::changed_files`], which reads objects only. Hooks,
//! fsmonitor, split index and lazy fetching are switched off for every call.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{ChangeKind, ChangedFile, EntryKind, FileVersion, GitError, ObjectId};

/// Inherited `GIT_*` variables are removed, except these, which only choose
/// which configuration files apply ("Git as the developer runs it").
const KEPT_GIT_VARIABLES: &[&str] = &[
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_NOSYSTEM",
];

/// Settings forced on every snapshot command: no hooks (they could come from
/// a checked-out branch through a relative `core.hooksPath`), no fsmonitor
/// hook or daemon, and no split index (it writes `sharedindex.*` into `.git`).
const FORCED_CONFIG: &[&str] = &["core.fsmonitor=false", "core.splitIndex=false"];

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
    /// Cache directory of this working tree.
    cache: PathBuf,
    /// Empty directory used as `core.hooksPath`.
    no_hooks: PathBuf,
    /// One snapshot at a time; each uses its own index file.
    snapshot_lock: Mutex<()>,
    next_index: AtomicU64,
}

impl WorkTree {
    /// Opens the working tree that contains `path`. `cache_dir` is created if
    /// missing and must be outside the working tree and the Git directory.
    pub fn new(path: &Path, cache_dir: &Path) -> Result<Self, GitError> {
        let top = PathBuf::from(output(
            plain_git(path).args(["rev-parse", "--show-toplevel"]),
            "rev-parse",
        )?);
        let git_path = |args: &[&str]| -> Result<PathBuf, GitError> {
            Ok(PathBuf::from(output(
                plain_git(&top)
                    .args(["rev-parse", "--path-format=absolute"])
                    .args(args),
                "rev-parse",
            )?))
        };
        let index = git_path(&["--git-path", "index"])?;
        let repo_objects = git_path(&["--git-path", "objects"])?;
        let common_dir = git_path(&["--git-common-dir"])?;

        let existed = cache_dir.exists();
        std::fs::create_dir_all(cache_dir).map_err(|error| io_failed("create cache", &error))?;
        // Canonical paths exist only now that the cache directory exists.
        let canonical =
            |p: &Path| std::fs::canonicalize(p).map_err(|e| io_failed("resolve path", &e));
        let cache = canonical(cache_dir)?;
        for forbidden in [canonical(&top)?, canonical(&common_dir)?] {
            if cache.starts_with(&forbidden) {
                if !existed {
                    // Undo the directory we just created; ignore failure (not empty).
                    let _ = std::fs::remove_dir(cache_dir);
                }
                return Err(GitError::Command {
                    command: "snapshot".to_owned(),
                    detail: format!(
                        "cache directory {} is inside the working tree or Git directory {}",
                        cache_dir.display(),
                        forbidden.display()
                    ),
                });
            }
        }
        let objects = cache.join("objects");
        let no_hooks = cache.join("no-hooks");
        for dir in [&objects, &no_hooks] {
            std::fs::create_dir_all(dir).map_err(|error| io_failed("create cache", &error))?;
        }
        Ok(Self {
            top,
            index,
            repo_objects,
            objects,
            cache,
            no_hooks,
            snapshot_lock: Mutex::new(()),
            next_index: AtomicU64::new(0),
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
        // A poisoned lock only means an earlier snapshot panicked; each
        // snapshot uses a fresh index file, so continuing is safe.
        let _guard = self
            .snapshot_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let number = self.next_index.fetch_add(1, Ordering::Relaxed);
        let index = self
            .cache
            .join(format!("index-{}-{number}", std::process::id()));
        let result = self.snapshot_with(&index);
        for leftover in [index.clone(), index.with_extension("lock")] {
            let _ = std::fs::remove_file(leftover);
        }
        result
    }

    fn snapshot_with(&self, index: &Path) -> Result<ObjectId, GitError> {
        match std::fs::copy(&self.index, index) {
            Ok(_) => keep_modified_time(&self.index, index)?,
            // No index yet: nothing was ever staged, `git add` starts empty.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_failed("copy index", &error)),
        }
        output(
            self.git().env("GIT_INDEX_FILE", index).args(["add", "-A"]),
            "add",
        )?;
        let tree = output(
            self.git().env("GIT_INDEX_FILE", index).arg("write-tree"),
            "write-tree",
        )?;
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

    /// `git` on the snapshot store: Lupa's object directory, the repository's
    /// objects as alternates, hooks and fsmonitor off.
    fn git(&self) -> Command {
        let mut command = plain_git(&self.top);
        let mut hooks = OsString::from("core.hooksPath=");
        hooks.push(&self.no_hooks);
        command.arg("-c").arg(hooks);
        for setting in FORCED_CONFIG {
            command.args(["-c", setting]);
        }
        command.env("GIT_OBJECT_DIRECTORY", &self.objects).env(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            quote_alternate(&self.repo_objects),
        );
        command
    }
}

/// Gives the copied index the original's modification time. Git compares
/// entry timestamps with the index file's own to spot "racily clean" entries
/// (changed in the same second the index was written); a copy with a newer
/// time would make Git trust stale stat data and miss same-size changes.
fn keep_modified_time(original: &Path, copy: &Path) -> Result<(), GitError> {
    let modified = std::fs::metadata(original)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| io_failed("read index time", &error))?;
    std::fs::File::options()
        .write(true)
        .open(copy)
        .and_then(|file| file.set_modified(modified))
        .map_err(|error| io_failed("set index time", &error))
}

/// `git -C dir` with inherited `GIT_*` variables removed, no prompts and no
/// network fetches of missing objects (partial clones).
fn plain_git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    for variable in git_variables_to_remove(std::env::vars_os().map(|(key, _)| key)) {
        command.env_remove(variable);
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .arg("-C")
        .arg(dir);
    command
}

fn git_variables_to_remove(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    names
        .filter(|name| {
            let text = name.to_string_lossy();
            text.to_ascii_uppercase().starts_with("GIT_")
                && !KEPT_GIT_VARIABLES
                    .iter()
                    .any(|kept| OsStr::new(kept) == name)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_every_inherited_git_variable_except_config_file_choice() {
        let names = [
            "GIT_DIR",
            "GIT_INDEX_FILE",
            "GIT_TRACE2_EVENT",
            "git_redirect_stdout",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_NOSYSTEM",
            "PATH",
            "HOME",
        ]
        .map(OsString::from);
        let removed = git_variables_to_remove(names.into_iter());
        assert_eq!(
            removed,
            [
                "GIT_DIR",
                "GIT_INDEX_FILE",
                "GIT_TRACE2_EVENT",
                "git_redirect_stdout"
            ]
            .map(OsString::from)
        );
    }
}
