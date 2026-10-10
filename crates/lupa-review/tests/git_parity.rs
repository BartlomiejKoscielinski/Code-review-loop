//! P1-12 acceptance: for commits, ranges and branches, statuses and hunks of
//! every changed file match `git diff --name-status -M` and
//! `git diff --histogram -U3`.

use lupa_diff::{DiffOptions, diff};
use lupa_git::{ChangeKind, Repo};
use lupa_review::{TargetSpec, changed_files, resolve};
use lupa_testkit::TestRepo;

fn history() -> TestRepo {
    let mut repo = TestRepo::new();
    let service = (1..=40)
        .map(|i| format!("    public int Value{i} {{ get; set; }}\n"))
        .collect::<String>();
    repo.write(
        "src/Service.cs",
        format!("class Service\n{{\n{service}}}\n"),
    )
    .write("src/Old.cs", "class Old {}\n")
    .write(
        "src/Moved.cs",
        "class Moved\n{\n    int a;\n    int b;\n    int c;\n}\n",
    );
    repo.commit("base");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write(
        "src/Service.cs",
        format!(
            "class Service\n{{\n{}}}\n",
            service
                .replace("Value3 ", "Third ")
                .replace("Value30 ", "Thirtieth ")
        ),
    )
    .remove("src/Old.cs")
    .remove("src/Moved.cs")
    .write(
        "src/Domain/Moved.cs",
        "class Moved\n{\n    int a;\n    int b;\n    int c;\n    int d;\n}\n",
    )
    .write("src/New.cs", "class New {}\n");
    repo.commit("feature work");
    repo.git(&["switch", "-q", "main"]);
    repo.write("README.md", "main moves on\n");
    repo.commit("main work");
    repo
}

fn status_letter(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "A",
        ChangeKind::Deleted => "D",
        ChangeKind::Modified => "M",
        ChangeKind::Renamed => "R",
        ChangeKind::TypeChanged => "T",
    }
}

/// Checks one target against git, given the two revisions git should diff.
fn assert_matches_git(repo: &TestRepo, spec: TargetSpec, git_base: &str, git_head: &str) {
    let opened = Repo::open(&repo.path()).unwrap();
    let target = resolve(&opened, &spec).unwrap();
    let files = changed_files(&opened, &target).unwrap();

    let mut ours: Vec<String> = files
        .iter()
        .map(|f| match f.kind {
            ChangeKind::Renamed => format!(
                "R {} {}",
                f.old.as_ref().unwrap().path,
                f.new.as_ref().unwrap().path
            ),
            kind => format!("{} {}", status_letter(kind), f.path()),
        })
        .collect();
    ours.sort();
    let raw = repo.git(&["diff", "--name-status", "-M", "-z", git_base, git_head]);
    let mut fields = raw.split('\0').filter(|f| !f.is_empty());
    let mut git = Vec::new();
    while let Some(status) = fields.next() {
        let path = fields.next().unwrap();
        if status.starts_with('R') {
            git.push(format!("R {path} {}", fields.next().unwrap()));
        } else {
            git.push(format!("{status} {path}"));
        }
    }
    git.sort();
    assert_eq!(ours, git, "{spec:?}: statuses");

    for file in &files {
        let read = |side: Option<&lupa_git::FileVersion>| {
            side.map(|v| opened.read_blob(v.id).unwrap())
                .unwrap_or_default()
        };
        let ours = diff(
            &read(file.old.as_ref()),
            &read(file.new.as_ref()),
            DiffOptions::default(),
        );
        let ours: Vec<String> = ours.hunks.iter().map(|h| h.header()).collect();
        let mut args = vec!["diff", "--histogram", "-U3", "-M", git_base, git_head, "--"];
        if let Some(old) = &file.old {
            args.push(&old.path);
        }
        if let Some(new) = &file.new {
            args.push(&new.path);
        }
        let git: Vec<String> = repo
            .git(&args)
            .lines()
            .filter_map(|line| line.strip_prefix("@@ "))
            .map(|rest| format!("@@ {} @@", &rest[..rest.find(" @@").unwrap()]))
            .collect();
        assert_eq!(ours, git, "{spec:?}: hunks of {}", file.path());
    }
}

#[test]
fn commit_statuses_and_hunks_match_git_show() {
    let repo = history();
    assert_matches_git(
        &repo,
        TargetSpec::Commit {
            revision: "feature".into(),
            parent: 1,
        },
        "feature^",
        "feature",
    );
}

#[test]
fn range_statuses_and_hunks_match_git_diff() {
    let repo = history();
    assert_matches_git(
        &repo,
        TargetSpec::Range {
            from: "main".into(),
            to: "feature".into(),
        },
        "main",
        "feature",
    );
}

#[test]
fn branch_statuses_and_hunks_match_git_three_dot_diff() {
    let repo = history();
    let base = repo.git(&["merge-base", "main", "feature"]);
    assert_matches_git(
        &repo,
        TargetSpec::Branch {
            target: "main".into(),
            source: "feature".into(),
        },
        &base,
        "feature",
    );
}
