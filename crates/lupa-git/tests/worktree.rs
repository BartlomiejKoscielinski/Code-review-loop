use lupa_git::{ChangeKind, ChangedFile, Repo, WorkTree};
use lupa_testkit::{TestRepo, plain_dir};

fn summary(files: &[ChangedFile]) -> Vec<String> {
    files
        .iter()
        .map(|file| {
            let letter = match file.kind {
                ChangeKind::Added => "A",
                ChangeKind::Deleted => "D",
                ChangeKind::Modified => "M",
                ChangeKind::Renamed => "R",
                ChangeKind::TypeChanged => "T",
            };
            format!("{letter} {}", file.path())
        })
        .collect()
}

fn head(repo: &TestRepo) -> lupa_git::ObjectId {
    repo.git(&["rev-parse", "HEAD"]).parse().unwrap()
}

fn object_count(repo: &TestRepo) -> String {
    repo.git(&["count-objects", "-v"])
}

#[test]
fn lists_staged_unstaged_deleted_and_untracked_changes() {
    let mut repo = TestRepo::new();
    repo.write(".gitignore", "bin/\n")
        .write("src/Staged.cs", "class Staged {}\n")
        .write("src/Unstaged.cs", "class Unstaged {}\n")
        .write("src/Deleted.cs", "class Deleted {}\n");
    repo.commit("base");
    repo.write("src/Staged.cs", "class Staged { int s; }\n");
    repo.git(&["add", "src/Staged.cs"]);
    repo.write("src/Unstaged.cs", "class Unstaged { int u; }\n")
        .remove("src/Deleted.cs")
        .write("src/New.cs", "class New {}\n")
        .write("bin/Debug/App.dll", "ignored");

    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    let files = tree.changed_files(Some(head(&repo))).unwrap();
    assert_eq!(
        summary(&files),
        [
            "D src/Deleted.cs",
            "A src/New.cs",
            "M src/Staged.cs",
            "M src/Unstaged.cs"
        ]
    );
}

#[test]
fn new_side_ids_equal_git_hash_object_and_content_is_readable() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "one\n");
    repo.commit("base");
    repo.write("a.txt", "two\n").write("b.txt", "new\n");

    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    let files = tree.changed_files(Some(head(&repo))).unwrap();
    for file in &files {
        let new = file.new.as_ref().unwrap();
        assert_eq!(new.id.to_string(), repo.git(&["hash-object", &new.path]));
        let expected = std::fs::read(repo.path().join(&new.path)).unwrap();
        assert_eq!(tree.read_blob(new.id).unwrap(), expected);
    }
    let old = files[0].old.as_ref().unwrap();
    assert_eq!(tree.read_blob(old.id).unwrap(), b"one\n");
}

#[test]
fn crlf_checkout_of_lf_blob_is_not_a_change_with_autocrlf() {
    let mut repo = TestRepo::new();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("Page.xaml.cs", "line1\nline2\n");
    repo.commit("lf in repository");
    repo.write("Page.xaml.cs", "line1\r\nline2\r\n");
    repo.write("Other.cs", "a\r\nb\r\n");
    repo.git(&["add", "Other.cs"]);
    repo.commit("other");
    repo.write("Other.cs", "a\r\nB\r\n");

    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    let files = tree.changed_files(Some(head(&repo))).unwrap();
    assert_eq!(summary(&files), ["M Other.cs"]);
    // Stored after Git's clean filter: LF, like the blob it is compared with.
    let new = files[0].new.as_ref().unwrap();
    assert_eq!(tree.read_blob(new.id).unwrap(), b"a\nB\n");
}

#[test]
fn does_not_write_objects_into_the_repository() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "one\n");
    repo.commit("base");
    repo.write("a.txt", "two\n").write("b.txt", "three\n");
    let before = object_count(&repo);

    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    tree.changed_files(Some(head(&repo))).unwrap();

    assert_eq!(object_count(&repo), before);
    assert!(
        std::fs::read_dir(cache.path().join("objects"))
            .unwrap()
            .count()
            > 0
    );
}

#[test]
fn merge_base_mode_includes_committed_branch_work() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "a\n");
    repo.commit("base");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("committed.txt", "c\n");
    repo.commit("branch work");
    repo.write("uncommitted.txt", "u\n");

    let opened = Repo::open(&repo.path()).unwrap();
    let base = opened
        .merge_base(
            opened.resolve_commit("main").unwrap(),
            opened.resolve_commit("HEAD").unwrap(),
        )
        .unwrap();
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    assert_eq!(
        summary(&tree.changed_files(Some(base)).unwrap()),
        ["A committed.txt", "A uncommitted.txt"]
    );
    assert_eq!(
        summary(&tree.changed_files(Some(head(&repo))).unwrap()),
        ["A uncommitted.txt"]
    );
}

#[test]
fn repository_without_commits_compares_with_the_empty_tree() {
    let repo = TestRepo::new();
    repo.write("first.txt", "1\n");
    repo.git(&["add", "first.txt"]);
    repo.write("second.txt", "2\n");
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    assert_eq!(
        summary(&tree.changed_files(None).unwrap()),
        ["A first.txt", "A second.txt"]
    );
}

#[test]
fn touched_but_unchanged_file_is_not_listed() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "same\n");
    repo.commit("base");
    repo.write("a.txt", "same\n");
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    assert!(tree.changed_files(Some(head(&repo))).unwrap().is_empty());
}
