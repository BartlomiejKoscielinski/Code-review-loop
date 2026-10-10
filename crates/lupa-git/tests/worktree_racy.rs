//! A file rewritten with the same size in the same second as the commit
//! ("racily clean" in Git terms) must still show up as changed. The copied
//! index must keep the original's modification time, or Git trusts stale
//! stat data and misses the change.

use lupa_git::WorkTree;
use lupa_testkit::{TestRepo, plain_dir};

#[test]
fn same_size_rewrite_right_after_commit_is_detected() {
    let cache = plain_dir();
    for round in 0..40 {
        let mut repo = TestRepo::new();
        repo.write("a.txt", "one\n");
        let head = repo.commit("base");
        repo.write("a.txt", "two\n");
        let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
        let files = tree.changed_files(Some(head.parse().unwrap())).unwrap();
        assert_eq!(files.len(), 1, "round {round}: rewrite of a.txt was missed");
    }
}
