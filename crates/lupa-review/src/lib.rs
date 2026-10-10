//! Review targets (ADR 0003): which two snapshots a review compares.
//!
//! Every source of changes resolves to a base and a head. This module covers
//! commits, ranges and branches; local changes and merge requests are added
//! by P1-13 and P1-15.

use lupa_git::{ChangedFile, GitError, ObjectId, Repo};

/// What the user asked to review, before revisions are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSpec {
    /// One commit against one of its parents (1 = first parent, as `git show`).
    Commit { revision: String, parent: u32 },
    /// Everything between two revisions: `git diff from to`.
    Range { from: String, to: String },
    /// A branch against the point where it left the target branch:
    /// `git diff target...source`.
    Branch { target: String, source: String },
}

/// Base of a comparison: a commit, or nothing (a root commit adds every file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Commit(ObjectId),
    EmptyTree,
}

impl Base {
    fn commit(self) -> Option<ObjectId> {
        match self {
            Base::Commit(id) => Some(id),
            Base::EmptyTree => None,
        }
    }
}

/// A resolved review target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTarget {
    /// Stable identifier built from the resolved commits, so reopening the
    /// same comparison finds its saved review state.
    pub id: String,
    pub spec: TargetSpec,
    pub base: Base,
    pub head: ObjectId,
}

#[derive(Debug, thiserror::Error)]
pub enum ReviewError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("commit {commit} has no parent number {parent}; it has {available}")]
    NoSuchParent {
        commit: ObjectId,
        parent: u32,
        available: usize,
    },
}

/// Resolves revisions and computes base and head.
pub fn resolve(repo: &Repo, spec: &TargetSpec) -> Result<ReviewTarget, ReviewError> {
    let (base, head, id) = match spec {
        TargetSpec::Commit { revision, parent } => {
            let head = repo.resolve_commit(revision)?;
            let parents = repo.parents(head)?;
            let base = if parents.is_empty() && *parent == 1 {
                Base::EmptyTree
            } else {
                let index = usize::try_from(*parent).unwrap_or(usize::MAX);
                match index.checked_sub(1).and_then(|i| parents.get(i)) {
                    Some(id) => Base::Commit(*id),
                    None => {
                        return Err(ReviewError::NoSuchParent {
                            commit: head,
                            parent: *parent,
                            available: parents.len(),
                        });
                    }
                }
            };
            (base, head, format!("commit:{head}:{parent}"))
        }
        TargetSpec::Range { from, to } => {
            let base = repo.resolve_commit(from)?;
            let head = repo.resolve_commit(to)?;
            (Base::Commit(base), head, format!("range:{base}..{head}"))
        }
        TargetSpec::Branch { target, source } => {
            let target = repo.resolve_commit(target)?;
            let head = repo.resolve_commit(source)?;
            let base = repo.merge_base(target, head)?;
            (Base::Commit(base), head, format!("branch:{base}...{head}"))
        }
    };
    Ok(ReviewTarget {
        id,
        spec: spec.clone(),
        base,
        head,
    })
}

/// Files that differ between the target's base and head.
pub fn changed_files(repo: &Repo, target: &ReviewTarget) -> Result<Vec<ChangedFile>, ReviewError> {
    Ok(repo.changed_files(target.base.commit(), target.head)?)
}
