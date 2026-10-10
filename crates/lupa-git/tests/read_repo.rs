use lupa_git::{ChangeKind, ChangedFile, EntryKind, GitError, Repo};
use lupa_testkit::TestRepo;

/// Our changes in `git diff --name-status` form, rename scores dropped.
fn name_status(files: &[ChangedFile]) -> Vec<String> {
    files
        .iter()
        .map(|file| match file.kind {
            ChangeKind::Added => format!("A {}", file.path()),
            ChangeKind::Deleted => format!("D {}", file.path()),
            ChangeKind::Modified => format!("M {}", file.path()),
            ChangeKind::Renamed => format!(
                "R {} {}",
                file.old.as_ref().unwrap().path,
                file.new.as_ref().unwrap().path
            ),
        })
        .collect()
}

/// `git diff --name-status -M -z` in the same form, sorted by path shown.
fn git_name_status(repo: &TestRepo, base: &str, head: &str) -> Vec<String> {
    let raw = repo.git(&["diff", "--name-status", "-M", "-z", base, head]);
    let mut fields = raw.split('\0').filter(|f| !f.is_empty());
    let mut out = Vec::new();
    while let Some(status) = fields.next() {
        let first = fields.next().unwrap();
        if status.starts_with('R') {
            let second = fields.next().unwrap();
            out.push((second.to_owned(), format!("R {first} {second}")));
        } else {
            out.push((first.to_owned(), format!("{} {first}", &status[..1])));
        }
    }
    out.sort();
    out.into_iter().map(|(_, line)| line).collect()
}

fn changes(repo: &TestRepo, base: &str, head: &str) -> Vec<ChangedFile> {
    let opened = Repo::open(repo.path()).unwrap();
    let base = opened.resolve_commit(base).unwrap();
    let head = opened.resolve_commit(head).unwrap();
    opened.changed_files(base, head).unwrap()
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

    let ours = name_status(&changes(&repo, &base, &head));
    let git = git_name_status(&repo, &base, &head);
    assert_eq!(ours, git);
    assert!(ours.contains(&"R src/FrameAnalyzer.cs src/Scanning/FrameAnalyzer.cs".to_owned()));
    assert!(ours.contains(&"M scripts/build.sh".to_owned()));
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
    assert_eq!(name_status(&files), git_name_status(&repo, &base, &head));

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
fn reads_blob_content_byte_for_byte() {
    let mut repo = TestRepo::new();
    repo.write("docs/Windows.txt", "a\r\nb\r\n");
    let commit = repo.commit("crlf");
    let opened = Repo::open(repo.path()).unwrap();
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

    let opened = Repo::open(repo.path()).unwrap();
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

    let opened = Repo::open(repo.path()).unwrap();
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
    let opened = Repo::open(repo.path()).unwrap();

    let unknown = opened.resolve_commit("no-such-branch").unwrap_err();
    assert!(matches!(unknown, GitError::UnknownRevision { .. }));
    assert_eq!(unknown.to_string(), "revision no-such-branch not found");

    let blob = opened.resolve_commit("HEAD:a.txt").unwrap_err();
    assert!(matches!(blob, GitError::NotACommit { .. }), "{blob:?}");
}

#[test]
fn opening_a_plain_directory_is_an_error() {
    let dir = lupa_testkit::plain_dir();
    let error = Repo::open(dir.path()).err().unwrap();
    assert!(matches!(error, GitError::NotARepository { .. }));
}
