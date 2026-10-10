//! Read-only access to a Git repository: resolving revisions, merge bases,
//! the list of changed files between two commits and blob contents.
//!
//! Object reading goes through `gix`. Working tree status and network
//! operations use the `git` CLI (spike S2) and live in other modules.

use std::path::Path;

pub use gix::ObjectId;
use gix::object::tree::diff::ChangeDetached;
use gix::objs::tree::EntryMode;

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
    #[error("revision {revision} not found")]
    UnknownRevision {
        revision: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("{revision} is not a commit")]
    NotACommit { revision: String },
    #[error("commits {one} and {two} have no common ancestor")]
    NoMergeBase { one: ObjectId, two: ObjectId },
    #[error("cannot read object {id}")]
    Object {
        id: ObjectId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("cannot compare trees of {base} and {head}")]
    TreeDiff {
        base: ObjectId,
        head: ObjectId,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// How a file changed between two commits, as in `git diff --name-status -M`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeKind {
    Added,
    Deleted,
    /// Content, mode or submodule pointer changed.
    Modified,
    Renamed,
}

/// Kind of tree entry on one side of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Executable,
    Symlink,
    /// Submodule pointer (gitlink); its id is a commit in another repository.
    Submodule,
}

/// One side of a change: path, object id and entry kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVersion {
    /// Path relative to the repository root, `/`-separated as Git stores it.
    pub path: String,
    pub id: ObjectId,
    pub kind: EntryKind,
}

/// A file that differs between two commits.
#[derive(Debug, Clone, PartialEq)]
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
            Err(source) => Err(GitError::Object {
                id: one,
                source: Box::new(source),
            }),
        }
    }

    /// Files that differ between the trees of `base` and `head`, with rename
    /// detection at 50% similarity like `git diff -M`. Sorted by path.
    pub fn changed_files(
        &self,
        base: ObjectId,
        head: ObjectId,
    ) -> Result<Vec<ChangedFile>, GitError> {
        let tree_error = |source: Box<dyn std::error::Error + Send + Sync>| GitError::TreeDiff {
            base,
            head,
            source,
        };
        let old_tree = self.commit_tree(base)?;
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
        } => Some(ChangedFile {
            kind: ChangeKind::Modified,
            old: Some(version(
                location.as_ref(),
                previous_id,
                previous_entry_mode,
            )?),
            new: Some(version(location.as_ref(), id, entry_mode)?),
        }),
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
