//! Compares lupa-diff with `git diff --histogram` on generated file pairs.

use lupa_diff::{DiffOptions, FileDiff, LineRange, diff, split_lines};
use lupa_testkit::TestRepo;

/// Small deterministic generator (xorshift), so failures reproduce.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const STATEMENTS: &[&str] = &[
    "        var frame = await source.NextFrameAsync(token);",
    "        if (frame is null)",
    "        {",
    "            return;",
    "        }",
    "        analyzer.Analyze(frame, token);",
    "        frame.Dispose();",
    "",
    "    public int Threshold { get; set; } = 10;",
    "    // Releases the camera when the page disappears.",
    "        MainThread.BeginInvokeOnMainThread(() => Found?.Invoke(this, code));",
    "    }",
    "    protected override void OnAppearing()",
    "    {",
    "        base.OnAppearing();",
];

fn generate(rng: &mut Rng) -> (String, String) {
    let len = 5 + rng.below(80);
    let old: Vec<String> = (0..len)
        .map(|_| STATEMENTS[rng.below(STATEMENTS.len())].to_owned())
        .collect();
    let mut new = old.clone();
    for _ in 0..1 + rng.below(6) {
        let at = rng.below(new.len() + 1);
        match rng.below(4) {
            0 => new.insert(at, STATEMENTS[rng.below(STATEMENTS.len())].to_owned()),
            1 if at < new.len() => {
                new.remove(at);
            }
            2 if at < new.len() => new[at] = format!("{} // changed", new[at]),
            _ => {
                let block: Vec<String> = (0..1 + rng.below(4))
                    .map(|_| STATEMENTS[rng.below(STATEMENTS.len())].to_owned())
                    .collect();
                for (offset, line) in block.into_iter().enumerate() {
                    new.insert((at + offset).min(new.len()), line);
                }
            }
        }
    }
    let join = |lines: &[String], rng: &mut Rng| {
        let mut text = lines.join("\n");
        // Most files end with a newline; some do not, as in real repositories.
        if rng.below(5) != 0 {
            text.push('\n');
        }
        text
    };
    let old_text = join(&old, rng);
    let new_text = join(&new, rng);
    (old_text, new_text)
}

/// Our hunks rendered like `git diff` output after the file header.
fn render(ours: &FileDiff, old: &[u8], new: &[u8]) -> String {
    let old_lines = split_lines(old);
    let new_lines = split_lines(new);
    let text = |line: &[u8]| {
        String::from_utf8_lossy(line.strip_suffix(b"\n").unwrap_or(line)).into_owned()
    };
    let mut out = String::new();
    for hunk in &ours.hunks {
        out.push_str(&hunk.header());
        out.push('\n');
        let mut o = hunk.old.start;
        for change in &hunk.changes {
            while o < change.old.start {
                out.push_str(&format!(" {}\n", text(old_lines[o as usize])));
                o += 1;
            }
            for i in change.old.start..change.old.end() {
                out.push_str(&format!("-{}\n", text(old_lines[i as usize])));
            }
            for i in change.new.start..change.new.end() {
                out.push_str(&format!("+{}\n", text(new_lines[i as usize])));
            }
            o = change.old.end();
        }
        while o < hunk.old.end() {
            out.push_str(&format!(" {}\n", text(old_lines[o as usize])));
            o += 1;
        }
    }
    out
}

/// `git diff` body from the first `@@`, function names after `@@` and
/// "\ No newline at end of file" markers removed.
fn git_body(repo: &TestRepo, old: &str, new: &str, extra: &[&str]) -> String {
    let a = repo.git_with_stdin(&["hash-object", "-w", "--stdin"], old);
    let b = repo.git_with_stdin(&["hash-object", "-w", "--stdin"], new);
    let mut args = vec!["diff", "--histogram", "--no-color"];
    args.extend_from_slice(extra);
    args.push(&a);
    args.push(&b);
    let output = repo.git(&args);
    let mut out = String::new();
    // split('\n'), not lines(): lines() would also strip the '\r' of CRLF lines.
    for line in output
        .split('\n')
        .skip_while(|line| !line.starts_with("@@"))
        .filter(|line| !line.is_empty())
    {
        if line.starts_with('\\') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let end = rest.find(" @@").unwrap();
            out.push_str(&format!("@@ {} @@\n", &rest[..end]));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn ranges(ours: &FileDiff) -> Vec<(LineRange, LineRange)> {
    ours.hunks
        .iter()
        .flat_map(|hunk| hunk.changes.iter().map(|c| (c.old, c.new)))
        .collect()
}

#[test]
fn hunks_match_git_on_200_generated_pairs() {
    let repo = TestRepo::new();
    let mut rng = Rng(0x5eed_1234_abcd_0001);
    let mut checked = 0;
    for case in 0..200 {
        let (old, new) = generate(&mut rng);
        let ours = diff(old.as_bytes(), new.as_bytes(), DiffOptions::default());
        let expected = git_body(&repo, &old, &new, &["-U3"]);
        let actual = render(&ours, old.as_bytes(), new.as_bytes());
        assert_eq!(
            actual, expected,
            "case {case}\n--- old\n{old}\n--- new\n{new}"
        );

        let zero = diff(
            old.as_bytes(),
            new.as_bytes(),
            DiffOptions {
                context: 0,
                ..DiffOptions::default()
            },
        );
        let headers: String = zero
            .hunks
            .iter()
            .map(|h| format!("{}\n", h.header()))
            .collect();
        let git_headers: String = git_body(&repo, &old, &new, &["-U0"])
            .lines()
            .filter(|line| line.starts_with("@@"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(headers, git_headers, "case {case} (-U0)");
        assert_eq!(
            ranges(&ours),
            ranges(&zero),
            "case {case}: context changed the ranges"
        );
        checked += 1;
    }
    assert_eq!(checked, 200);
}

#[test]
fn hunk_near_file_start_has_three_context_lines_like_git() {
    let repo = TestRepo::new();
    let old = "using A;\nusing B;\nusing C;\nusing D;\n\nnamespace App;\n\nclass X {}\n";
    let new = "using A;\nusing B;\nusing C;\nusing D;\nusing E;\n\nnamespace App;\n\nclass X {}\n";
    let ours = diff(old.as_bytes(), new.as_bytes(), DiffOptions::default());
    assert_eq!(
        render(&ours, old.as_bytes(), new.as_bytes()),
        git_body(&repo, old, new, &["-U3"])
    );
    assert_eq!(ours.hunks[0].header(), "@@ -2,6 +2,7 @@");
}

#[test]
fn crlf_lines_and_missing_final_newline_match_git() {
    let repo = TestRepo::new();
    let old = "a\r\nb\r\nc\r\nd";
    let new = "a\r\nB\r\nc\r\nd\r\n";
    let ours = diff(old.as_bytes(), new.as_bytes(), DiffOptions::default());
    assert_eq!(
        render(&ours, old.as_bytes(), new.as_bytes()),
        git_body(&repo, old, new, &["-U3"])
    );
    assert_eq!((ours.added, ours.removed), (2, 2));
}

#[test]
fn ignoring_indentation_matches_git_ignore_space_change_for_reindented_code() {
    let repo = TestRepo::new();
    let old = "void Run()\n{\n    if (ready)\n    {\n        Start();\n    }\n}\n";
    let new =
        "void Run()\n{\n    if (ready)\n    {\n            Start();\n        Log();\n    }\n}\n";
    let options = DiffOptions {
        context: 0,
        ignore_indentation: true,
    };
    let ours = diff(old.as_bytes(), new.as_bytes(), options);
    let headers: Vec<String> = ours.hunks.iter().map(|h| h.header()).collect();
    let git: Vec<String> = git_body(&repo, old, new, &["-U0", "--ignore-space-change"])
        .lines()
        .filter(|line| line.starts_with("@@"))
        .map(str::to_owned)
        .collect();
    assert_eq!(headers, git);
    assert_eq!((ours.added, ours.removed), (1, 0));
}
