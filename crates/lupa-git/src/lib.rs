//! Read-only access to a Git repository: resolving revisions, merge bases,
//! the list of changed files between two commits and blob contents.
//!
//! Object reading goes through `gix`. Local changes ([`WorkTree`]) and network
//! operations use the `git` CLI (ADR 0005).

use std::path::Path;

mod worktree;

pub use gix::ObjectId;
use gix::object::tree::diff::ChangeDetached;
use gix::objs::tree::EntryMode;
pub use worktree::WorkTree;

/// Errors from reading a repository. Messages name the revision or object,
/// never credentials (this crate does not handle any).
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("{path} is not a Git repository")]
    NotARepository {
        path: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("cannot resolve revision {revision}")]
    UnknownRevision {
        revision: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("{revision} is not a commit")]
    NotACommit { revision: String },
    #[error("commits {one} and {two} have no common ancestor")]
    NoMergeBase { one: ObjectId, two: ObjectId },
    #[error("cannot compute the merge base of {one} and {two}")]
    MergeBase {
        one: ObjectId,
        two: ObjectId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("object {id} is not a file (blob)")]
    NotABlob { id: ObjectId },
    /// A `git` command failed; `detail` is its error output (local commands
    /// only, which never see credentials).
    #[error("git {command} failed: {detail}")]
    Command { command: String, detail: String },
    #[error("cannot read object {id}")]
    Object {
        id: ObjectId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("cannot compare trees of {} and {head}", base.map_or("the empty tree".to_owned(), |id| id.to_string()))]
    TreeDiff {
        base: Option<ObjectId>,
        head: ObjectId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// How a file changed between two commits, as in `git diff --name-status -M`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Added,
    Deleted,
    /// Content, executable bit or submodule pointer changed.
    Modified,
    Renamed,
    /// The entry became a different kind (file, symlink or submodule): `T` in
    /// Git. A line diff between the two sides is meaningless.
    TypeChanged,
}

/// Kind of tree entry on one side of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    File,
    Executable,
    Symlink,
    /// Submodule pointer (gitlink); its id is a commit in another repository.
    Submodule,
}

impl EntryKind {
    /// Git's type classes: a regular and an executable file are the same type.
    fn same_type(self, other: Self) -> bool {
        let class = |kind| match kind {
            EntryKind::File | EntryKind::Executable => 0,
            EntryKind::Symlink => 1,
            EntryKind::Submodule => 2,
        };
        class(self) == class(other)
    }
}

/// One side of a change: path, object id and entry kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVersion {
    /// Path relative to the repository root, `/`-separated as Git stores it.
    /// Non-UTF-8 bytes (rare, only in repositories created on Linux) are
    /// replaced, so such a path cannot be mapped back to a working tree file.
    pub path: String,
    pub id: ObjectId,
    pub kind: EntryKind,
}

/// A file that differs between two commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub kind: ChangeKind,
    /// Absent for added files.
    pub old: Option<FileVersion>,
    /// Absent for deleted files.
    pub new: Option<FileVersion>,
}

impl ChangedFile {
    /// Path to show for the change: the new path, or the old one for deletions.
    pub fn path(&self) -> &str {
        self.new
            .as_ref()
            .or(self.old.as_ref())
            .map_or("", |version| version.path.as_str())
    }
}

/// An open repository. Cheap to keep around for the lifetime of a review.
pub struct Repo {
    inner: gix::Repository,
}

impl Repo {
    /// Opens the repository that contains `path`.
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let mut inner = gix::discover(path).map_err(|source| GitError::NotARepository {
            path: path.display().to_string(),
            source: Box::new(source),
        })?;
        inner.object_cache_size_if_unset(16 * 1024 * 1024);
        Ok(Self { inner })
    }

    /// Resolves a revision (`main`, `HEAD~2`, a SHA prefix, ...) to a commit id.
    pub fn resolve_commit(&self, revision: &str) -> Result<ObjectId, GitError> {
        let object = self
            .inner
            .rev_parse_single(revision)
            .map_err(|source| GitError::UnknownRevision {
                revision: revision.to_owned(),
                source: Box::new(source),
            })?
            .object()
            .map_err(|source| GitError::UnknownRevision {
                revision: revision.to_owned(),
                source: Box::new(source),
            })?;
        let commit = object.peel_to_commit().map_err(|_| GitError::NotACommit {
            revision: revision.to_owned(),
        })?;
        Ok(commit.id)
    }

    /// Best common ancestor of two commits, as `git merge-base`.
    pub fn merge_base(&self, one: ObjectId, two: ObjectId) -> Result<ObjectId, GitError> {
        match self.inner.merge_base(one, two) {
            Ok(Some(id)) => Ok(id.detach()),
            Ok(None) => Err(GitError::NoMergeBase { one, two }),
            Err(source) => Err(GitError::MergeBase {
                one,
                two,
                source: Box::new(source),
            }),
        }
    }

    /// Parent commits of `commit`, first parent first (none for a root commit).
    pub fn parents(&self, commit: ObjectId) -> Result<Vec<ObjectId>, GitError> {
        let object_error = |source: Box<dyn std::error::Error + Send + Sync>| GitError::Object {
            id: commit,
            source,
        };
        let commit = self
            .inner
            .find_commit(commit)
            .map_err(|source| object_error(Box::new(source)))?;
        Ok(commit.parent_ids().map(|id| id.detach()).collect())
    }

    /// Files that differ between the trees of `base` and `head`, with rename
    /// detection at 50% similarity like `git diff -M`. A `None` base is the
    /// empty tree (everything in `head` is added). Sorted by path.
    pub fn changed_files(
        &self,
        base: Option<ObjectId>,
        head: ObjectId,
    ) -> Result<Vec<ChangedFile>, GitError> {
        let tree_error = |source: Box<dyn std::error::Error + Send + Sync>| GitError::TreeDiff {
            base,
            head,
            source,
        };
        let old_tree = match base {
            Some(base) => self.commit_tree(base)?,
            None => self.inner.empty_tree(),
        };
        let new_tree = self.commit_tree(head)?;
        let mut options = gix::diff::Options::default();
        options.track_rewrites(Some(gix::diff::Rewrites {
            copies: None,
            percentage: Some(0.5),
            limit: 1000,
            track_empty: false,
        }));
        let changes = self
            .inner
            .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), options)
            .map_err(|source| tree_error(Box::new(source)))?;
        let mut files: Vec<ChangedFile> = changes.into_iter().filter_map(to_changed_file).collect();
        files.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(files)
    }

    /// Raw content of a blob.
    pub fn read_blob(&self, id: ObjectId) -> Result<Vec<u8>, GitError> {
        let object = self
            .inner
            .find_object(id)
            .map_err(|source| GitError::Object {
                id,
                source: Box::new(source),
            })?;
        if object.kind != gix::object::Kind::Blob {
            return Err(GitError::NotABlob { id });
        }
        Ok(object.detach().data)
    }

    fn commit_tree(&self, commit: ObjectId) -> Result<gix::Tree<'_>, GitError> {
        let object_error = |source: Box<dyn std::error::Error + Send + Sync>| GitError::Object {
            id: commit,
            source,
        };
        self.inner
            .find_object(commit)
            .map_err(|source| object_error(Box::new(source)))?
            .peel_to_tree()
            .map_err(|source| object_error(Box::new(source)))
    }
}

fn entry_kind(mode: EntryMode) -> Option<EntryKind> {
    if mode.is_tree() {
        None
    } else if mode.is_commit() {
        Some(EntryKind::Submodule)
    } else if mode.is_link() {
        Some(EntryKind::Symlink)
    } else if mode.is_executable() {
        Some(EntryKind::Executable)
    } else {
        Some(EntryKind::File)
    }
}

fn version(path: &gix::bstr::BStr, id: ObjectId, mode: EntryMode) -> Option<FileVersion> {
    Some(FileVersion {
        path: String::from_utf8_lossy(path).into_owned(),
        id,
        kind: entry_kind(mode)?,
    })
}

/// Converts a gix change; directories (tree entries) are skipped, as in `git diff`.
fn to_changed_file(change: ChangeDetached) -> Option<ChangedFile> {
    match change {
        ChangeDetached::Addition {
            entry_mode,
            id,
            location,
            ..
        } => Some(ChangedFile {
            kind: ChangeKind::Added,
            old: None,
            new: Some(version(location.as_ref(), id, entry_mode)?),
        }),
        ChangeDetached::Deletion {
            entry_mode,
            id,
            location,
            ..
        } => Some(ChangedFile {
            kind: ChangeKind::Deleted,
            old: Some(version(location.as_ref(), id, entry_mode)?),
            new: None,
        }),
        ChangeDetached::Modification {
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
            location,
        } => {
            let old = version(location.as_ref(), previous_id, previous_entry_mode)?;
            let new = version(location.as_ref(), id, entry_mode)?;
            let kind = if old.kind.same_type(new.kind) {
                ChangeKind::Modified
            } else {
                ChangeKind::TypeChanged
            };
            Some(ChangedFile {
                kind,
                old: Some(old),
                new: Some(new),
            })
        }
        ChangeDetached::Rewrite {
            source_location,
            source_entry_mode,
            source_id,
            entry_mode,
            id,
            location,
            copy,
            ..
        } => {
            let old = version(source_location.as_ref(), source_id, source_entry_mode)?;
            let new = version(location.as_ref(), id, entry_mode)?;
            // Copy detection is off, so a rewrite is always a rename.
            debug_assert!(!copy);
            Some(ChangedFile {
                kind: ChangeKind::Renamed,
                old: Some(old),
                new: Some(new),
            })
        }
    }
}
