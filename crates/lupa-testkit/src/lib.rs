//! Throwaway Git repositories for tests.
//!
//! Repositories are built with the `git` CLI in a temporary directory, so the
//! tests compare Lupa against real Git behaviour. Every repository pins the
//! settings that differ between machines (line endings, author, dates), so the
//! same test gives the same result on Windows and Linux.
//!
//! This crate is only used from tests, so it panics on failure instead of
//! returning errors.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// A Git repository in a temporary directory, removed on drop.
pub struct TestRepo {
    dir: TempDir,
    commits: u32,
}

impl TestRepo {
    /// Creates an empty repository with branch `main`.
    pub fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("lupa-test-")
            .tempdir()
            .unwrap();
        let repo = Self { dir, commits: 0 };
        repo.git(&["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.name", "Lupa Test"),
            ("user.email", "test@lupa.invalid"),
            ("core.autocrlf", "false"),
            ("core.safecrlf", "false"),
            ("commit.gpgsign", "false"),
            ("init.defaultBranch", "main"),
        ] {
            repo.git(&["config", key, value]);
        }
        repo
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes a file (creating parent directories) without staging it.
    pub fn write(&self, relative: &str, content: impl AsRef<[u8]>) -> &Self {
        let path = self.file(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
        self
    }

    /// Removes a file from the working tree without staging the removal.
    pub fn remove(&self, relative: &str) -> &Self {
        std::fs::remove_file(self.file(relative)).unwrap();
        self
    }

    /// Stages everything and commits. Returns the commit SHA.
    ///
    /// Author and committer dates advance by one minute per commit, so SHAs
    /// are stable between runs.
    pub fn commit(&mut self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.commit_index(message)
    }

    /// Commits the index as it is, without staging the working tree. Needed
    /// for entries prepared with `git update-index`, such as submodule pointers.
    pub fn commit_index(&mut self, message: &str) -> String {
        let date = format!("2026-01-01T00:{:02}:00Z", self.commits % 60);
        self.commits += 1;
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(["commit", "-q", "--allow-empty", "-m", message])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.git(&["rev-parse", "HEAD"])
    }

    /// Runs git in the repository and returns trimmed stdout. Panics on failure.
    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end()
            .to_owned()
    }

    fn file(&self, relative: &str) -> PathBuf {
        self.path().join(relative)
    }
}

/// An empty temporary directory that is not a Git repository.
pub fn plain_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("lupa-plain-")
        .tempdir()
        .unwrap()
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commits_get_stable_shas_across_runs() {
        let build = || {
            let mut repo = TestRepo::new();
            repo.write("a.txt", "hello\n");
            repo.commit("first")
        };
        assert_eq!(build(), build());
    }
}
