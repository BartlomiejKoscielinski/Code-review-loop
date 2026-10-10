//! Throwaway Git repositories for tests.
//!
//! Repositories are built with the `git` CLI in a temporary directory, so the
//! tests compare Lupa against real Git behaviour. Git runs hermetically: no
//! system or global configuration and no `GIT_*` variables inherited from the
//! caller (a Git hook running the tests would otherwise point them at the
//! developer's repository). Line endings, author, dates and the object format
//! are pinned, so the same test gives the same result on Windows and Linux.
//!
//! This crate is only used from tests, so it panics on failure instead of
//! returning errors.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

/// Variables Git reads that would redirect or reconfigure the test repository.
const INHERITED_GIT_VARIABLES: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_CEILING_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

/// A Git repository in a temporary directory, removed on drop.
pub struct TestRepo {
    root: TempDir,
    commits: u32,
}

impl TestRepo {
    /// Creates an empty SHA-1 repository with branch `main`.
    pub fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("lupa-test-")
            .tempdir()
            .unwrap();
        std::fs::create_dir(root.path().join("repo")).unwrap();
        std::fs::write(root.path().join("gitconfig"), "").unwrap();
        let repo = Self { root, commits: 0 };
        repo.git(&["init", "-q", "-b", "main", "--object-format=sha1"]);
        for (key, value) in [
            ("user.name", "Lupa Test"),
            ("user.email", "test@lupa.invalid"),
            ("core.autocrlf", "false"),
            ("core.safecrlf", "false"),
        ] {
            repo.git(&["config", key, value]);
        }
        repo
    }

    pub fn path(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    /// Writes a file (creating parent directories) without staging it.
    pub fn write(&self, relative: &str, content: impl AsRef<[u8]>) -> &Self {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
        self
    }

    /// Removes a file from the working tree without staging the removal.
    pub fn remove(&self, relative: &str) -> &Self {
        std::fs::remove_file(self.path().join(relative)).unwrap();
        self
    }

    /// Stages everything and commits. Returns the commit SHA.
    pub fn commit(&mut self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.commit_index(message)
    }

    /// Commits the index as it is, without staging the working tree. Needed
    /// for entries prepared with `git update-index`, such as submodule pointers.
    ///
    /// Each commit is dated one second after the previous one, so SHAs are
    /// stable between runs and commit order follows creation order.
    pub fn commit_index(&mut self, message: &str) -> String {
        let date = format!("{} +0000", 1_767_225_600 + u64::from(self.commits));
        self.commits += 1;
        let output = self
            .command(&["commit", "-q", "--allow-empty", "-m", message])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        check(&output, "commit");
        self.git(&["rev-parse", "HEAD"])
    }

    /// Runs git in the repository and returns stdout without the trailing
    /// newline. Panics on failure.
    pub fn git(&self, args: &[&str]) -> String {
        let output = self.command(args).output().unwrap();
        check(&output, &args.join(" "));
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end_matches('\n')
            .to_owned()
    }

    /// Like [`Self::git`], with `input` written to stdin.
    pub fn git_with_stdin(&self, args: &[&str], input: &str) -> String {
        use std::io::Write;
        let mut child = self
            .command(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        check(&output, &args.join(" "));
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end_matches('\n')
            .to_owned()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        for variable in INHERITED_GIT_VARIABLES {
            command.env_remove(variable);
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root.path().join("gitconfig"))
            .env("GIT_TERMINAL_PROMPT", "0")
            .arg("-C")
            .arg(self.path())
            .args(args);
        command
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}

fn check(output: &std::process::Output, what: &str) {
    assert!(
        output.status.success(),
        "git {what} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An empty temporary directory that is not a Git repository.
pub fn plain_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("lupa-plain-")
        .tempdir()
        .unwrap()
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

    #[test]
    fn git_runs_without_inherited_git_variables_or_user_config() {
        let repo = TestRepo::new();
        let command = repo.command(&["status"]);
        let envs: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        for variable in INHERITED_GIT_VARIABLES {
            assert!(
                envs.contains(&((*variable).to_owned(), None)),
                "{variable} not removed"
            );
        }
        assert!(envs.contains(&("GIT_CONFIG_NOSYSTEM".to_owned(), Some("1".to_owned()))));
        assert_eq!(repo.git(&["config", "--global", "--list"]), "");
    }
}
