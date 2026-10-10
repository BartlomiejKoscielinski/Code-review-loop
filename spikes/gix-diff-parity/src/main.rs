//! Spike S2: does imara-diff (histogram + indent heuristic) produce the same
//! hunks as `git diff --histogram`, and how fast are gix tree diffs and blob
//! diffs on a real .NET MAUI repository? Throwaway code, see README.md.

use std::process::Command;
use std::time::Instant;

use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("parity") => parity(&args[2], args[3].parse().unwrap()),
        Some("perf") => perf(&args[2], &args[3], &args[4]),
        Some("show") => show(&args[2], &args[3], &args[4]),
        _ => eprintln!("usage: parity <repo> <pairs> | perf <repo> <from> <to>"),
    }
}

fn git(repo: &str, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    out.stdout
}

/// Unified diff body from imara-diff, hunk headers normalized like git's.
fn imara_unified(before: &str, after: &str) -> String {
    let input = InternedInput::new(before, after);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let mut config = UnifiedDiffConfig::default();
    config.context_len(3);
    let printer = BasicLineDiffPrinter(&input.interner);
    normalize(&diff.unified_diff(&printer, config, &input).to_string())
}

/// Hunks from `git diff --histogram`, starting at the first `@@`.
fn git_unified(repo: &str, old: &str, new: &str) -> String {
    let out = git(
        repo,
        &["diff", "--histogram", "-U3", "--no-color", old, new],
    );
    let text = String::from_utf8_lossy(&out);
    let start = text.find("\n@@").map(|i| i + 1).unwrap_or(text.len());
    normalize(&text[start..])
}

/// Drops the function-name suffix after `@@ ... @@`, the `,1` counts git
/// omits, and the "No newline at end of file" markers.
fn normalize(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with('\\') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let end = rest.find(" @@").unwrap();
            let ranges: Vec<String> = rest[..end]
                .split(' ')
                .map(|r| {
                    let (sign, body) = r.split_at(1);
                    let mut parts = body.split(',');
                    let start = parts.next().unwrap();
                    let len = parts.next().unwrap_or("1");
                    format!("{sign}{start},{len}")
                })
                .collect();
            out.push_str(&format!("@@ {} @@\n", ranges.join(" ")));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Modified text blobs (old, new, path) from the history of HEAD.
fn blob_pairs(repo: &str, limit: usize) -> Vec<(String, String, String)> {
    let log = String::from_utf8(git(repo, &["log", "--format=%H", "--no-merges"])).unwrap();
    let mut pairs = Vec::new();
    for commit in log.lines() {
        let raw = String::from_utf8(git(
            repo,
            &[
                "diff-tree",
                "-r",
                "--no-commit-id",
                "-M",
                &format!("{commit}^"),
                commit,
            ],
        ))
        .unwrap();
        for line in raw.lines() {
            // :old_mode new_mode old_sha new_sha status\tpath
            let (meta, path) = line.split_once('\t').unwrap();
            let fields: Vec<&str> = meta.split(' ').collect();
            let is_text = [".cs", ".xaml", ".csproj", ".json", ".md", ".yml", ".props"]
                .iter()
                .any(|ext| path.ends_with(ext));
            if fields[4] == "M" && is_text {
                pairs.push((fields[2].to_owned(), fields[3].to_owned(), path.to_owned()));
                if pairs.len() == limit {
                    return pairs;
                }
            }
        }
    }
    pairs
}

/// Changed ranges as git prints them with -U0: (old_start, old_len, new_start, new_len).
type Ranges = Vec<(u32, u32, u32, u32)>;

fn imara_ranges(before: &str, after: &str) -> Ranges {
    let input = InternedInput::new(before, after);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let git_start = |start: u32, len: u32| if len == 0 { start } else { start + 1 };
    diff.hunks()
        .map(|h| {
            let (ol, nl) = (h.before.end - h.before.start, h.after.end - h.after.start);
            (
                git_start(h.before.start, ol),
                ol,
                git_start(h.after.start, nl),
                nl,
            )
        })
        .collect()
}

fn git_ranges(repo: &str, old: &str, new: &str) -> Ranges {
    let out = git(
        repo,
        &["diff", "--histogram", "-U0", "--no-color", old, new],
    );
    let text = String::from_utf8_lossy(&out);
    let parse = |r: &str| -> (u32, u32) {
        let mut parts = r[1..].split(',');
        let start = parts.next().unwrap().parse().unwrap();
        let len = parts.next().map(|l| l.parse().unwrap()).unwrap_or(1);
        (start, len)
    };
    text.lines()
        .filter_map(|line| line.strip_prefix("@@ "))
        .map(|rest| {
            let mut it = rest.split(' ');
            let (os, ol) = parse(it.next().unwrap());
            let (ns, nl) = parse(it.next().unwrap());
            (os, ol, ns, nl)
        })
        .collect()
}

fn parity(repo: &str, limit: usize) {
    let pairs = blob_pairs(repo, limit);
    let mut mismatches = 0;
    let mut printer_mismatches = 0;
    let started = Instant::now();
    for (old, new, path) in &pairs {
        let before = String::from_utf8_lossy(&git(repo, &["cat-file", "-p", old])).into_owned();
        let after = String::from_utf8_lossy(&git(repo, &["cat-file", "-p", new])).into_owned();
        if imara_unified(&before, &after) != git_unified(repo, old, new) {
            printer_mismatches += 1;
        }
        let ours = imara_ranges(&before, &after);
        let theirs = git_ranges(repo, old, new);
        if ours != theirs {
            mismatches += 1;
            if mismatches <= 3 {
                println!("MISMATCH {path} {old}..{new}");
                println!("  imara: {ours:?}\n  git:   {theirs:?}");
            }
        }
    }
    println!(
        "pairs={} identical_ranges={} range_mismatches={} unified_printer_mismatches={} elapsed={:?}",
        pairs.len(),
        pairs.len() - mismatches,
        mismatches,
        printer_mismatches,
        started.elapsed()
    );
}

fn perf(repo_path: &str, from: &str, to: &str) {
    let started = Instant::now();
    let repo = gix::open(repo_path).unwrap();
    let opened = started.elapsed();

    let tree = |rev: &str| {
        repo.rev_parse_single(rev)
            .unwrap()
            .object()
            .unwrap()
            .peel_to_tree()
            .unwrap()
    };
    let t0 = Instant::now();
    let (old_tree, new_tree) = (tree(from), tree(to));
    let changes = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
        .unwrap();
    let tree_diff = t0.elapsed();

    let t1 = Instant::now();
    let mut lines_changed = 0u64;
    let mut blobs = 0;
    for change in &changes {
        if let gix::object::tree::diff::ChangeDetached::Modification {
            previous_id, id, ..
        } = change
        {
            let before = repo.find_object(*previous_id).unwrap().detach().data;
            let after = repo.find_object(*id).unwrap().detach().data;
            let (Ok(before), Ok(after)) =
                (std::str::from_utf8(&before), std::str::from_utf8(&after))
            else {
                continue;
            };
            let input = InternedInput::new(before, after);
            let mut diff = Diff::compute(Algorithm::Histogram, &input);
            diff.postprocess_lines(&input);
            lines_changed += u64::from(diff.count_additions() + diff.count_removals());
            blobs += 1;
        }
    }
    let blob_diffs = t1.elapsed();

    let t2 = Instant::now();
    let _ = git(repo_path, &["diff", "--histogram", "--numstat", from, to]);
    let git_numstat = t2.elapsed();

    println!(
        "open={opened:?} changes={} tree_diff={tree_diff:?} modified_text_blobs={blobs} \
         changed_lines={lines_changed} blob_diffs={blob_diffs:?} git_diff_numstat={git_numstat:?}",
        changes.len()
    );
}

fn show(repo: &str, old: &str, new: &str) {
    let before = String::from_utf8_lossy(&git(repo, &["cat-file", "-p", old])).into_owned();
    let after = String::from_utf8_lossy(&git(repo, &["cat-file", "-p", new])).into_owned();
    let input = InternedInput::new(before.as_str(), after.as_str());
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let mut config = UnifiedDiffConfig::default();
    config.context_len(3);
    let printer = BasicLineDiffPrinter(&input.interner);
    print!("{}", diff.unified_diff(&printer, config, &input));
}
