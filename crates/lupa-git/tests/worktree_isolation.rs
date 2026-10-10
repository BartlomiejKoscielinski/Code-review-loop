//! Security review of P1-07 (ADR 0007): a snapshot must not run repository
//! hooks, must not change anything inside `.git`, and must keep its cache
//! out of the repository.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lupa_git::WorkTree;
use lupa_testkit::{TestRepo, plain_dir};

/// Every file under `dir` with its content, keyed by relative path.
fn files_under(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let key = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.insert(key, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn repo_with_changes() -> (TestRepo, String) {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "one\n").write("b.txt", "two\n");
    let head = repo.commit("base");
    repo.write("a.txt", "changed\n")
        .write("c.txt", "new\n")
        .remove("b.txt");
    (repo, head)
}

#[test]
fn snapshot_leaves_every_file_in_dot_git_unchanged() {
    let (repo, head) = repo_with_changes();
    let before = files_under(&repo.path().join(".git"));
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    assert_eq!(
        tree.changed_files(Some(head.parse().unwrap()))
            .unwrap()
            .len(),
        3
    );
    assert_eq!(files_under(&repo.path().join(".git")), before);
}

#[test]
fn split_index_repository_is_not_written_to() {
    let (repo, head) = repo_with_changes();
    repo.git(&["config", "core.splitIndex", "true"]);
    repo.git(&["update-index", "--split-index"]);
    let before = files_under(&repo.path().join(".git"));
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    assert_eq!(
        tree.changed_files(Some(head.parse().unwrap()))
            .unwrap()
            .len(),
        3
    );
    assert_eq!(files_under(&repo.path().join(".git")), before);
}

#[cfg(unix)]
#[test]
fn repository_hooks_do_not_run() {
    use std::os::unix::fs::PermissionsExt;

    let (repo, head) = repo_with_changes();
    let marker = plain_dir();
    let script = format!(
        "#!/bin/sh\ntouch '{}/$(basename \"$0\")-ran'\n",
        marker.path().display()
    );
    // A hook in .git/hooks and one in a checked-out directory used through a
    // relative core.hooksPath, as husky-style setups do.
    for dir in [
        repo.path().join(".git/hooks"),
        repo.path().join(".githooks"),
    ] {
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["post-index-change", "fsmonitor-watchman"] {
            let path = dir.join(name);
            std::fs::write(&path, &script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    for hooks_path in [None, Some(".githooks")] {
        if let Some(path) = hooks_path {
            repo.git(&["config", "core.hooksPath", path]);
        }
        let cache = plain_dir();
        let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
        tree.changed_files(Some(head.parse().unwrap())).unwrap();
        let ran: Vec<_> = std::fs::read_dir(marker.path()).unwrap().collect();
        assert!(
            ran.is_empty(),
            "hooks ran with core.hooksPath {hooks_path:?}"
        );
    }
}

#[test]
fn cache_inside_git_directory_is_rejected_and_not_left_behind() {
    let (repo, _) = repo_with_changes();
    let inside = repo.path().join(".git").join("lupa-cache");
    let error = WorkTree::new(&repo.path(), &inside).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("inside the working tree or Git directory"),
        "{error}"
    );
    assert!(!inside.exists());
    let in_tree = repo.path().join("new-cache-dir");
    assert!(WorkTree::new(&repo.path(), &in_tree).is_err());
    assert!(!in_tree.exists());
}

#[test]
fn concurrent_snapshots_agree() {
    let (repo, head) = repo_with_changes();
    let cache = plain_dir();
    let tree = Arc::new(WorkTree::new(&repo.path(), cache.path()).unwrap());
    let base: lupa_git::ObjectId = head.parse().unwrap();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let tree = Arc::clone(&tree);
            std::thread::spawn(move || tree.changed_files(Some(base)).unwrap())
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(results[0].len(), 3);
}
