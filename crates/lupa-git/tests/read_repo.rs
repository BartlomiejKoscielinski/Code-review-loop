use lupa_git::{ChangeKind, ChangedFile, EntryKind, FileVersion, GitError, Repo};
use lupa_testkit::TestRepo;

const ZERO: &str = "0000000000000000000000000000000000000000";

fn mode(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::File => "100644",
        EntryKind::Executable => "100755",
        EntryKind::Symlink => "120000",
        EntryKind::Submodule => "160000",
    }
}

fn side(version: Option<&FileVersion>) -> (&'static str, String, &str) {
    match version {
        Some(v) => (mode(v.kind), v.id.to_string(), v.path.as_str()),
        None => ("000000", ZERO.to_owned(), ""),
    }
}

/// Our changes in `git diff --raw` form (rename scores dropped), in our order.
fn raw(files: &[ChangedFile]) -> Vec<String> {
    files
        .iter()
        .map(|file| {
            let (old_mode, old_id, old_path) = side(file.old.as_ref());
            let (new_mode, new_id, new_path) = side(file.new.as_ref());
            let (status, paths) = match file.kind {
                ChangeKind::Added => ("A", file.path().to_owned()),
                ChangeKind::Deleted => ("D", file.path().to_owned()),
                ChangeKind::Modified => ("M", file.path().to_owned()),
                ChangeKind::TypeChanged => ("T", file.path().to_owned()),
                ChangeKind::Renamed => ("R", format!("{old_path} -> {new_path}")),
            };
            format!(":{old_mode} {new_mode} {old_id} {new_id} {status} {paths}")
        })
        .collect()
}

/// `git diff --raw -M -z --abbrev=40` in the same form, in Git's order.
fn git_raw(repo: &TestRepo, base: &str, head: &str) -> Vec<String> {
    let output = repo.git(&["diff", "--raw", "-M", "-z", "--abbrev=40", base, head]);
    let mut fields = output.split('\0').filter(|f| !f.is_empty());
    let mut out = Vec::new();
    while let Some(meta) = fields.next() {
        let parts: Vec<&str> = meta.split(' ').collect();
        let status = &parts[4][..1];
        let first = fields.next().unwrap();
        let paths = if status == "R" {
            format!("{first} -> {}", fields.next().unwrap())
        } else {
            first.to_owned()
        };
        out.push(format!(
            "{} {} {} {} {status} {paths}",
            parts[0], parts[1], parts[2], parts[3]
        ));
    }
    out
}

fn changes(repo: &TestRepo, base: &str, head: &str) -> Vec<ChangedFile> {
    let opened = Repo::open(&repo.path()).unwrap();
    let base = opened.resolve_commit(base).unwrap();
    let head = opened.resolve_commit(head).unwrap();
    opened.changed_files(Some(base), head).unwrap()
}

const SERVICE: &str = "namespace App.Services;\n\npublic sealed class FrameAnalyzer\n{\n    public int Threshold { get; set; } = 10;\n\n    public bool Analyze(int value)\n    {\n        return value > Threshold;\n    }\n}\n";

#[test]
fn changed_files_match_git_name_status() {
    let mut repo = TestRepo::new();
    repo.write("src/FrameAnalyzer.cs", SERVICE)
        .write("src/Obsolete.cs", "class Obsolete {}\n")
        .write("src/Modified.cs", "class Modified { }\n")
        .write("assets/logo.bin", [0u8, 159, 146, 150, 0, 1])
        .write("scripts/build.sh", "echo build\n")
        .write("docs/Windows.txt", "line one\r\nline two\r\n")
        .write("docs/zażółć gęślą.md", "przed\n");
    let base = repo.commit("base");

    repo.remove("src/FrameAnalyzer.cs")
        .write(
            "src/Scanning/FrameAnalyzer.cs",
            SERVICE.replace("> Threshold", ">= Threshold"),
        )
        .remove("src/Obsolete.cs")
        .write("src/Modified.cs", "class Modified { int x; }\n")
        .write("src/Added.cs", "class Added {}\n")
        .write("assets/logo.bin", [0u8, 159, 146, 150, 0, 2])
        .write("docs/Windows.txt", "line one\r\nline 2\r\n")
        .write("docs/zażółć gęślą.md", "po\n");
    repo.git(&["add", "-A"]);
    repo.git(&["update-index", "--chmod=+x", "scripts/build.sh"]);
    let head = repo.commit_index("head");

    let ours = raw(&changes(&repo, &base, &head));
    assert_eq!(ours, git_raw(&repo, &base, &head));
    assert!(
        ours.iter()
            .any(|l| l.ends_with(" R src/FrameAnalyzer.cs -> src/Scanning/FrameAnalyzer.cs"))
    );
    assert!(
        ours.iter()
            .any(|l| l.starts_with(":100644 100755 ") && l.ends_with(" M scripts/build.sh"))
    );
}

#[test]
fn reports_entry_kinds_for_mode_and_submodule_changes() {
    let mut repo = TestRepo::new();
    let first_pointer = "1111111111111111111111111111111111111111";
    let second_pointer = "2222222222222222222222222222222222222222";
    repo.write("run.sh", "echo run\n");
    repo.git(&["add", "-A"]);
    repo.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{first_pointer},vendor/lib"),
    ]);
    let base = repo.commit_index("base");
    repo.git(&["update-index", "--chmod=+x", "run.sh"]);
    repo.git(&[
        "update-index",
        "--cacheinfo",
        &format!("160000,{second_pointer},vendor/lib"),
    ]);
    let head = repo.commit_index("head");

    let files = changes(&repo, &base, &head);
    assert_eq!(raw(&files), git_raw(&repo, &base, &head));

    let script = files.iter().find(|f| f.path() == "run.sh").unwrap();
    assert_eq!(script.old.as_ref().unwrap().kind, EntryKind::File);
    assert_eq!(script.new.as_ref().unwrap().kind, EntryKind::Executable);

    let submodule = files.iter().find(|f| f.path() == "vendor/lib").unwrap();
    assert_eq!(submodule.kind, ChangeKind::Modified);
    assert_eq!(submodule.new.as_ref().unwrap().kind, EntryKind::Submodule);
    assert_eq!(
        submodule.new.as_ref().unwrap().id.to_string(),
        second_pointer
    );
}

#[test]
fn directory_rename_lists_every_file_like_git() {
    let mut repo = TestRepo::new();
    for name in ["Camera", "Frame", "Scanner"] {
        repo.write(
            &format!("src/Old/{name}.cs"),
            format!("namespace App;\n\npublic sealed class {name}\n{{\n    public int Id {{ get; set; }}\n}}\n"),
        );
    }
    repo.write("src/Other.cs", "class Other {}\n");
    let base = repo.commit("base");
    repo.git(&["mv", "src/Old", "src/New"]);
    repo.write(
        "src/New/Frame.cs",
        "namespace App;\n\npublic sealed class Frame\n{\n    public int Id { get; set; }\n    public int Width { get; set; }\n}\n",
    );
    let head = repo.commit("rename directory");

    let ours = raw(&changes(&repo, &base, &head));
    assert_eq!(ours, git_raw(&repo, &base, &head));
    assert_eq!(ours.len(), 3, "{ours:#?}");
}

#[test]
fn type_change_between_file_and_symlink_is_reported_like_git() {
    let mut repo = TestRepo::new();
    repo.write("config/app.json", "{}\n");
    let base = repo.commit("file");
    // A symlink entry via the index works on Windows without symlink rights.
    let link = repo.git_with_stdin(&["hash-object", "-w", "--stdin"], "../shared/app.json");
    repo.git(&[
        "update-index",
        "--cacheinfo",
        &format!("120000,{link},config/app.json"),
    ]);
    let head = repo.commit_index("symlink");

    let files = changes(&repo, &base, &head);
    assert_eq!(raw(&files), git_raw(&repo, &base, &head));
    assert_eq!(files[0].kind, ChangeKind::TypeChanged);
    assert_eq!(files[0].new.as_ref().unwrap().kind, EntryKind::Symlink);
}

#[test]
fn reading_a_tree_as_blob_is_an_error() {
    let mut repo = TestRepo::new();
    repo.write("src/a.txt", "a\n");
    repo.commit("first");
    let tree = repo.git(&["rev-parse", "HEAD:src"]);
    let opened = Repo::open(&repo.path()).unwrap();
    let error = opened.read_blob(tree.parse().unwrap()).unwrap_err();
    assert!(matches!(error, GitError::NotABlob { .. }), "{error:?}");
}

#[test]
fn reads_blob_content_byte_for_byte() {
    let mut repo = TestRepo::new();
    repo.write("docs/Windows.txt", "a\r\nb\r\n");
    let commit = repo.commit("crlf");
    let opened = Repo::open(&repo.path()).unwrap();
    assert_eq!(opened.resolve_commit("HEAD").unwrap().to_string(), commit);
    let blob = repo.git(&["rev-parse", "HEAD:docs/Windows.txt"]);
    assert_eq!(
        opened.read_blob(blob.parse().unwrap()).unwrap(),
        b"a\r\nb\r\n"
    );
}

#[test]
fn merge_base_matches_git() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "1\n");
    repo.commit("root");
    repo.write("a.txt", "2\n");
    let fork = repo.commit("fork point");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("feature.txt", "f\n");
    let feature = repo.commit("feature work");
    repo.git(&["switch", "-q", "main"]);
    repo.write("main.txt", "m\n");
    let main = repo.commit("main work");

    let opened = Repo::open(&repo.path()).unwrap();
    let base = opened
        .merge_base(main.parse().unwrap(), feature.parse().unwrap())
        .unwrap();
    assert_eq!(base.to_string(), fork);
    assert_eq!(repo.git(&["merge-base", "main", "feature"]), fork);
}

#[test]
fn resolves_branch_tag_and_relative_revisions() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "1\n");
    let first = repo.commit("first");
    repo.write("a.txt", "2\n");
    let second = repo.commit("second");
    repo.git(&["tag", "-a", "v1", "-m", "annotated", &first]);

    let opened = Repo::open(&repo.path()).unwrap();
    assert_eq!(opened.resolve_commit("main").unwrap().to_string(), second);
    assert_eq!(opened.resolve_commit("HEAD~1").unwrap().to_string(), first);
    assert_eq!(opened.resolve_commit("v1").unwrap().to_string(), first);
    assert_eq!(
        opened.resolve_commit(&second[..10]).unwrap().to_string(),
        second
    );
}

#[test]
fn reports_unknown_revision_and_non_commit_objects() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "1\n");
    repo.commit("first");
    let opened = Repo::open(&repo.path()).unwrap();

    let unknown = opened.resolve_commit("no-such-branch").unwrap_err();
    assert!(matches!(unknown, GitError::UnknownRevision { .. }));
    assert_eq!(
        unknown.to_string(),
        "cannot resolve revision no-such-branch"
    );

    let blob = opened.resolve_commit("HEAD:a.txt").unwrap_err();
    assert!(matches!(blob, GitError::NotACommit { .. }), "{blob:?}");
}

#[test]
fn opening_a_plain_directory_is_an_error() {
    let dir = lupa_testkit::plain_dir();
    let error = Repo::open(dir.path()).err().unwrap();
    assert!(matches!(error, GitError::NotARepository { .. }));
}
