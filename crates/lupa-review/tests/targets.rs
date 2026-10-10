use lupa_git::{ChangedFile, Repo};
use lupa_review::{Base, ReviewError, TargetSpec, changed_files, resolve};
use lupa_testkit::TestRepo;

fn names(files: &[ChangedFile]) -> Vec<String> {
    files.iter().map(|f| f.path().to_owned()).collect()
}

fn git_names(repo: &TestRepo, args: &[&str]) -> Vec<String> {
    let mut full = vec!["diff", "--name-only", "-M", "-z"];
    full.extend_from_slice(args);
    let mut out: Vec<String> = repo
        .git(&full)
        .split('\0')
        .filter(|f| !f.is_empty())
        .map(str::to_owned)
        .collect();
    out.sort();
    out
}

fn review(repo: &TestRepo, spec: TargetSpec) -> Vec<String> {
    let opened = Repo::open(&repo.path()).unwrap();
    let target = resolve(&opened, &spec).unwrap();
    names(&changed_files(&opened, &target).unwrap())
}

/// main: root -> m1 -> merge(feature) -> m2; feature: f1 -> f2 from root.
fn history() -> (TestRepo, [String; 6]) {
    let mut repo = TestRepo::new();
    repo.write("app/App.cs", "class App {}\n");
    let root = repo.commit("root");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("app/Feature.cs", "class Feature {}\n");
    let f1 = repo.commit("f1");
    repo.write("app/Feature.cs", "class Feature { int x; }\n");
    let f2 = repo.commit("f2");
    repo.git(&["switch", "-q", "main"]);
    repo.write("app/Main.cs", "class Main {}\n");
    let m1 = repo.commit("m1");
    repo.git(&["merge", "-q", "--no-ff", "--no-edit", "feature"]);
    let merge = repo.git(&["rev-parse", "HEAD"]);
    repo.write("app/App.cs", "class App { }\n");
    let m2 = repo.commit("m2");
    (repo, [root, f1, f2, m1, merge, m2])
}

#[test]
fn commit_matches_git_show() {
    let (repo, [_, _, f2, ..]) = history();
    let spec = TargetSpec::Commit {
        revision: f2.clone(),
        parent: 1,
    };
    assert_eq!(
        review(&repo, spec),
        git_names(&repo, &[&format!("{f2}^"), &f2])
    );
}

#[test]
fn merge_commit_against_second_parent_matches_git() {
    let (repo, [.., merge, _]) = history();
    for parent in [1, 2] {
        let spec = TargetSpec::Commit {
            revision: merge.clone(),
            parent,
        };
        let expected = git_names(&repo, &[&format!("{merge}^{parent}"), &merge]);
        assert_eq!(review(&repo, spec), expected, "parent {parent}");
    }
}

#[test]
fn root_commit_shows_every_file_as_added() {
    let (repo, [root, ..]) = history();
    let opened = Repo::open(&repo.path()).unwrap();
    let target = resolve(
        &opened,
        &TargetSpec::Commit {
            revision: root,
            parent: 1,
        },
    )
    .unwrap();
    assert_eq!(target.base, Base::EmptyTree);
    let files = changed_files(&opened, &target).unwrap();
    assert_eq!(names(&files), ["app/App.cs"]);
    assert_eq!(files[0].kind, lupa_git::ChangeKind::Added);
}

#[test]
fn missing_parent_is_reported() {
    let (repo, [_, f1, ..]) = history();
    let opened = Repo::open(&repo.path()).unwrap();
    let error = resolve(
        &opened,
        &TargetSpec::Commit {
            revision: f1,
            parent: 2,
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, ReviewError::NoSuchParent { available: 1, .. }),
        "{error}"
    );
}

#[test]
fn range_matches_git_diff_between_two_revisions() {
    let (repo, [root, .., m2]) = history();
    let spec = TargetSpec::Range {
        from: root.clone(),
        to: m2.clone(),
    };
    assert_eq!(review(&repo, spec), git_names(&repo, &[&root, &m2]));
}

#[test]
fn branch_matches_git_three_dot_diff() {
    let (mut repo, _) = history();
    repo.git(&["switch", "-q", "-c", "topic", "main~1"]);
    repo.write("app/Topic.cs", "class Topic {}\n");
    repo.commit("topic work");
    repo.git(&["switch", "-q", "main"]);
    let spec = TargetSpec::Branch {
        target: "main".into(),
        source: "topic".into(),
    };
    assert_eq!(review(&repo, spec), git_names(&repo, &["main...topic"]));
}

#[test]
fn target_id_is_stable_and_names_resolved_commits() {
    let (repo, [root, .., m2]) = history();
    let opened = Repo::open(&repo.path()).unwrap();
    let by_sha = resolve(
        &opened,
        &TargetSpec::Range {
            from: root.clone(),
            to: m2.clone(),
        },
    )
    .unwrap();
    let by_name = resolve(
        &opened,
        &TargetSpec::Range {
            from: root.clone(),
            to: "main".into(),
        },
    )
    .unwrap();
    assert_eq!(by_sha.id, format!("range:{root}..{m2}"));
    assert_eq!(by_sha.id, by_name.id);
}

#[test]
fn criss_cross_merge_base_matches_git() {
    let mut repo = TestRepo::new();
    repo.write("a.txt", "0\n");
    repo.commit("root");
    repo.git(&["switch", "-q", "-c", "left"]);
    repo.write("left.txt", "l\n");
    repo.commit("l1");
    repo.git(&["switch", "-q", "main"]);
    repo.write("right.txt", "r\n");
    repo.commit("r1");
    repo.git(&["switch", "-q", "-c", "right"]);
    repo.git(&["merge", "-q", "--no-edit", "left"]);
    repo.git(&["switch", "-q", "left"]);
    repo.git(&["merge", "-q", "--no-edit", "main"]);
    repo.write("left.txt", "l2\n");
    repo.commit("l2");
    repo.git(&["switch", "-q", "right"]);
    repo.write("right.txt", "r2\n");
    repo.commit("r2");

    let bases = repo.git(&["merge-base", "--all", "left", "right"]);
    assert_eq!(bases.lines().count(), 2, "history must be criss-cross");
    let spec = TargetSpec::Branch {
        target: "left".into(),
        source: "right".into(),
    };
    let opened = Repo::open(&repo.path()).unwrap();
    let target = resolve(&opened, &spec).unwrap();
    let git_base = repo.git(&["merge-base", "left", "right"]);
    assert_eq!(target.base, Base::Commit(git_base.parse().unwrap()));
    assert_eq!(review(&repo, spec), git_names(&repo, &["left...right"]));
}
