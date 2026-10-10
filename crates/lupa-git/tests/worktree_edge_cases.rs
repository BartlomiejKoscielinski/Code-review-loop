//! Working tree cases from the P1-07 review: conflicts, symlinks, nested
//! repositories, unusual paths, subdirectories and many changed files.

use std::time::Instant;

use lupa_git::{ChangeKind, ChangedFile, EntryKind, WorkTree};
use lupa_testkit::{TestRepo, plain_dir};

fn head(repo: &TestRepo) -> lupa_git::ObjectId {
    repo.git(&["rev-parse", "HEAD"]).parse().unwrap()
}

fn changes(repo: &TestRepo) -> Vec<ChangedFile> {
    let cache = plain_dir();
    WorkTree::new(&repo.path(), cache.path())
        .unwrap()
        .changed_files(Some(head(repo)))
        .unwrap()
}

fn paths(files: &[ChangedFile]) -> Vec<&str> {
    files.iter().map(ChangedFile::path).collect()
}

#[test]
fn merge_conflict_shows_the_file_with_conflict_markers() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "base\n");
    repo.commit("base");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("a.txt", "other\n");
    repo.commit("other");
    repo.git(&["switch", "-q", "main"]);
    repo.write("a.txt", "main\n");
    repo.commit("main");
    let merge = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["merge", "-q", "other"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(!merge.status.success(), "merge must conflict");

    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path(), cache.path()).unwrap();
    let files = tree.changed_files(Some(head(&repo))).unwrap();
    assert_eq!(paths(&files), ["a.txt"]);
    let content = tree.read_blob(files[0].new.as_ref().unwrap().id).unwrap();
    assert!(String::from_utf8(content).unwrap().contains("<<<<<<<"));
}

#[test]
fn symlinks_and_dangling_symlinks_are_symlink_entries() {
    if cfg!(windows) {
        // Creating symlinks needs developer mode on Windows; the file/symlink
        // type change itself is covered by read_repo.rs through the index.
        return;
    }
    let mut repo = TestRepo::new();
    repo.write("target.txt", "content\n");
    repo.commit("base");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("target.txt", repo.path().join("link")).unwrap();
        std::os::unix::fs::symlink("missing.txt", repo.path().join("dangling")).unwrap();
    }
    let files = changes(&repo);
    assert_eq!(paths(&files), ["dangling", "link"]);
    for file in &files {
        assert_eq!(file.new.as_ref().unwrap().kind, EntryKind::Symlink);
    }
}

#[test]
fn nested_repository_is_a_submodule_entry_not_an_error() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "a\n");
    repo.commit("base");
    let mut nested = TestRepo::new();
    nested.write("lib.txt", "lib\n");
    nested.commit("nested");
    let target = repo.path().join("vendor").join("lib");
    std::fs::create_dir_all(repo.path().join("vendor")).unwrap();
    let clone = std::process::Command::new("git")
        .args(["clone", "-q"])
        .arg(nested.path())
        .arg(&target)
        .output()
        .unwrap();
    assert!(clone.status.success());

    let files = changes(&repo);
    assert_eq!(paths(&files), ["vendor/lib"]);
    assert_eq!(files[0].new.as_ref().unwrap().kind, EntryKind::Submodule);
}

#[test]
fn unusual_file_names_keep_their_paths() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "a\n");
    repo.commit("base");
    let names = [
        "docs/zażółć gęślą.md",
        "docs/\"quoted\" name.txt",
        "docs/tab\there.txt",
    ];
    for name in names {
        if cfg!(windows) && (name.contains('"') || name.contains('\t')) {
            continue;
        }
        repo.write(name, "x\n");
    }
    let files = changes(&repo);
    let mut expected: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| !cfg!(windows) || !(n.contains('"') || n.contains('\t')))
        .collect();
    expected.sort();
    assert_eq!(paths(&files), expected);
}

#[test]
fn opening_from_a_subdirectory_reports_paths_from_the_root() {
    let mut repo = TestRepo::new();
    repo.write("src/App/Page.cs", "class Page {}\n");
    repo.commit("base");
    repo.write("src/App/Page.cs", "class Page { }\n")
        .write("README.md", "new\n");
    let cache = plain_dir();
    let tree = WorkTree::new(&repo.path().join("src").join("App"), cache.path()).unwrap();
    let files = tree.changed_files(Some(head(&repo))).unwrap();
    assert_eq!(paths(&files), ["README.md", "src/App/Page.cs"]);
}

#[test]
fn cache_inside_the_working_tree_is_rejected() {
    let repo = TestRepo::new();
    let error = WorkTree::new(&repo.path(), &repo.path().join(".lupa-cache"))
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("inside the working tree"),
        "{error}"
    );
}

#[test]
fn staged_rename_is_reported_as_rename() {
    let mut repo = TestRepo::new();
    repo.write("Old.cs", "class Moved\n{\n    int a;\n    int b;\n}\n");
    repo.commit("base");
    repo.git(&["mv", "Old.cs", "New.cs"]);
    let files = changes(&repo);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].kind, ChangeKind::Renamed);
    assert_eq!(files[0].old.as_ref().unwrap().path, "Old.cs");
    assert_eq!(files[0].new.as_ref().unwrap().path, "New.cs");
}

#[test]
fn thousands_of_changed_files_finish_quickly() {
    let mut repo = TestRepo::new();
    for i in 0..3000 {
        repo.write(&format!("src/F{i}.cs"), format!("class F{i} {{}}\n"));
    }
    repo.commit("base");
    for i in 0..3000 {
        repo.write(
            &format!("src/F{i}.cs"),
            format!("class F{i} {{ int x; }}\n"),
        );
    }
    let started = Instant::now();
    let files = changes(&repo);
    assert_eq!(files.len(), 3000);
    assert!(
        started.elapsed().as_secs() < 30,
        "took {:?}",
        started.elapsed()
    );
}
