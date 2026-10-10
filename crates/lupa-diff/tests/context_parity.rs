//! Hunks with context widths other than Git's default 3 match `git diff -U<n>`.

use lupa_diff::{DiffOptions, FileDiff, diff, split_lines};
use lupa_testkit::TestRepo;

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn generate(rng: &mut Rng) -> (String, String) {
    let words = [
        "alpha",
        "beta",
        "gamma",
        "delta",
        "",
        "    nested();",
        "}",
        "{",
    ];
    let old: Vec<&str> = (0..10 + rng.below(60))
        .map(|_| words[rng.below(words.len())])
        .collect();
    let mut new = old.clone();
    for _ in 0..1 + rng.below(5) {
        let at = rng.below(new.len() + 1);
        if rng.below(2) == 0 || at == new.len() {
            new.insert(at, words[rng.below(words.len())]);
        } else {
            new.remove(at);
        }
    }
    (
        format!("{}\n", old.join("\n")),
        format!("{}\n", new.join("\n")),
    )
}

fn render(ours: &FileDiff, old: &[u8], new: &[u8]) -> String {
    let (old_lines, new_lines) = (split_lines(old), split_lines(new));
    let text = |line: &[u8]| {
        String::from_utf8_lossy(line.strip_suffix(b"\n").unwrap_or(line)).into_owned()
    };
    let mut out = String::new();
    for hunk in &ours.hunks {
        out.push_str(&format!("{}\n", hunk.header()));
        let mut o = hunk.old.start;
        for change in &hunk.changes {
            for i in o..change.old.start {
                out.push_str(&format!(" {}\n", text(old_lines[i as usize])));
            }
            for i in change.old.start..change.old.end() {
                out.push_str(&format!("-{}\n", text(old_lines[i as usize])));
            }
            for i in change.new.start..change.new.end() {
                out.push_str(&format!("+{}\n", text(new_lines[i as usize])));
            }
            o = change.old.end();
        }
        for i in o..hunk.old.end() {
            out.push_str(&format!(" {}\n", text(old_lines[i as usize])));
        }
    }
    out
}

fn git_body(repo: &TestRepo, old: &str, new: &str, context: u32) -> String {
    let a = repo.git_with_stdin(&["hash-object", "-w", "--stdin"], old);
    let b = repo.git_with_stdin(&["hash-object", "-w", "--stdin"], new);
    let flag = format!("-U{context}");
    let output = repo.git(&["diff", "--histogram", "--no-color", &flag, &a, &b]);
    output
        .split('\n')
        .skip_while(|line| !line.starts_with("@@"))
        .filter(|line| !line.is_empty() && !line.starts_with('\\'))
        .map(|line| match line.strip_prefix("@@ ") {
            Some(rest) => format!("@@ {} @@\n", &rest[..rest.find(" @@").unwrap()]),
            None => format!("{line}\n"),
        })
        .collect()
}

#[test]
fn hunks_match_git_for_context_1_and_10() {
    let repo = TestRepo::new();
    let mut rng = Rng(0x00c0_ffee_1234_5678);
    for case in 0..150 {
        let (old, new) = generate(&mut rng);
        for context in [1, 10] {
            let options = DiffOptions {
                context,
                ..DiffOptions::default()
            };
            let ours = diff(old.as_bytes(), new.as_bytes(), options);
            assert_eq!(
                render(&ours, old.as_bytes(), new.as_bytes()),
                git_body(&repo, &old, &new, context),
                "case {case}, -U{context}\n--- old\n{old}\n--- new\n{new}"
            );
        }
    }
}

#[test]
fn huge_context_does_not_overflow() {
    let options = DiffOptions {
        context: u32::MAX,
        ..DiffOptions::default()
    };
    // Two changes far apart: the hunk-merging check compares their distance
    // with twice the context, which must not overflow.
    let ours = diff(b"a\nb\nc\nd\ne\nf\n", b"A\nb\nc\nd\ne\nF\n", options);
    assert_eq!(ours.hunks.len(), 1);
    assert_eq!(ours.hunks[0].header(), "@@ -1,6 +1,6 @@");
}
