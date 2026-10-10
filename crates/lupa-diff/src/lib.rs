//! Line diffs that match `git diff --histogram` (ADR 0005).
//!
//! The diff works on bytes, so files in any encoding diff the same way Git
//! does. Lines keep their terminator (`\n` or `\r\n`); a last line without one
//! is a different line from the same text with one, as in Git. The result is
//! line ranges only: the UI already holds both file versions, so no text is
//! copied into the diff.

use imara_diff::{Algorithm, Diff, InternedInput, Interner, Token};

/// Diff settings shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffOptions {
    /// Unchanged lines shown around each change; Git's default is 3.
    pub context: u32,
    /// "Ignoruj wcięcia": lines that differ only in leading spaces and tabs
    /// count as unchanged.
    pub ignore_indentation: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            context: 3,
            ignore_indentation: false,
        }
    }
}

/// A run of lines: `start` is 0-based, `len` may be 0 (an insertion point).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,
    pub len: u32,
}

impl LineRange {
    pub fn end(self) -> u32 {
        self.start + self.len
    }

    /// Start as Git prints it in a hunk header: 1-based, or the line before
    /// the insertion point for an empty range.
    pub fn git_start(self) -> u32 {
        if self.len == 0 {
            self.start
        } else {
            self.start + 1
        }
    }
}

/// Lines removed from the old file and added to the new one at one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub old: LineRange,
    pub new: LineRange,
}

/// Changes close enough to share their context, as one `@@` block in Git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// Old lines covered by the hunk, context included.
    pub old: LineRange,
    /// New lines covered by the hunk, context included.
    pub new: LineRange,
    pub changes: Vec<Change>,
}

impl Hunk {
    /// `@@ -a,b +c,d @@` exactly as Git prints it (without the function name).
    pub fn header(&self) -> String {
        format!("@@ -{} +{} @@", git_range(self.old), git_range(self.new))
    }
}

fn git_range(range: LineRange) -> String {
    if range.len == 1 {
        format!("{}", range.git_start())
    } else {
        format!("{},{}", range.git_start(), range.len)
    }
}

/// Diff of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub old_lines: u32,
    pub new_lines: u32,
    pub hunks: Vec<Hunk>,
    pub added: u32,
    pub removed: u32,
}

/// Splits into lines with their terminators, like Git and `imara-diff`.
pub fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    data.split_inclusive(|&byte| byte == b'\n').collect()
}

/// Diffs two file versions.
pub fn diff(old: &[u8], new: &[u8], options: DiffOptions) -> FileDiff {
    let old_lines = split_lines(old);
    let new_lines = split_lines(new);
    let input = intern(&old_lines, &new_lines, options.ignore_indentation);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);

    let changes: Vec<Change> = diff
        .hunks()
        .map(|hunk| Change {
            old: LineRange {
                start: hunk.before.start,
                len: hunk.before.end - hunk.before.start,
            },
            new: LineRange {
                start: hunk.after.start,
                len: hunk.after.end - hunk.after.start,
            },
        })
        .collect();
    let old_count = to_u32(old_lines.len());
    let new_count = to_u32(new_lines.len());
    FileDiff {
        old_lines: old_count,
        new_lines: new_count,
        hunks: group(changes, options.context, old_count, new_count),
        added: diff.count_additions(),
        removed: diff.count_removals(),
    }
}

/// `imara-diff` works with `u32` token counts; files larger than that are
/// rejected before diffing (P1-10 limits file size far below this).
fn to_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn intern<'a>(
    old: &[&'a [u8]],
    new: &[&'a [u8]],
    ignore_indentation: bool,
) -> InternedInput<&'a [u8]> {
    let key = |line: &'a [u8]| -> &'a [u8] {
        if ignore_indentation {
            let indent = line
                .iter()
                .take_while(|&&byte| byte == b' ' || byte == b'\t')
                .count();
            &line[indent..]
        } else {
            line
        }
    };
    let mut interner = Interner::new(old.len() + new.len());
    let before: Vec<Token> = old.iter().map(|line| interner.intern(key(line))).collect();
    let after: Vec<Token> = new.iter().map(|line| interner.intern(key(line))).collect();
    InternedInput {
        before,
        after,
        interner,
    }
}

/// Groups changes into hunks like Git: changes whose context would touch or
/// overlap (at most `2 * context` unchanged lines apart) share one hunk.
fn group(changes: Vec<Change>, context: u32, old_count: u32, new_count: u32) -> Vec<Hunk> {
    let mut hunks: Vec<Hunk> = Vec::new();
    for change in changes {
        let starts_new_hunk = hunks.last().is_none_or(|hunk| {
            let previous = hunk.changes[hunk.changes.len() - 1];
            change.old.start - previous.old.end() > 2 * context
        });
        if starts_new_hunk {
            hunks.push(Hunk {
                old: change.old,
                new: change.new,
                changes: vec![change],
            });
        } else if let Some(hunk) = hunks.last_mut() {
            hunk.changes.push(change);
        }
    }
    for hunk in &mut hunks {
        let first = hunk.changes[0];
        let last = hunk.changes[hunk.changes.len() - 1];
        let before = context.min(first.old.start).min(first.new.start);
        let after = context
            .min(old_count - last.old.end())
            .min(new_count - last.new.end());
        hunk.old = LineRange {
            start: first.old.start - before,
            len: last.old.end() + after - (first.old.start - before),
        };
        hunk.new = LineRange {
            start: first.new.start - before,
            len: last.new.end() + after - (first.new.start - before),
        };
    }
    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(old: &str, new: &str) -> Vec<String> {
        diff(old.as_bytes(), new.as_bytes(), DiffOptions::default())
            .hunks
            .iter()
            .map(Hunk::header)
            .collect()
    }

    #[test]
    fn identical_files_have_no_hunks() {
        assert!(headers("a\nb\n", "a\nb\n").is_empty());
    }

    #[test]
    fn added_file_is_one_hunk_from_line_zero() {
        assert_eq!(headers("", "a\nb\n"), ["@@ -0,0 +1,2 @@"]);
    }

    #[test]
    fn change_in_first_line_has_no_leading_context() {
        assert_eq!(
            headers("a\nb\nc\nd\ne\n", "A\nb\nc\nd\ne\n"),
            ["@@ -1,4 +1,4 @@"]
        );
    }

    #[test]
    fn ignoring_indentation_hides_reindented_lines() {
        let old = "if (x)\n{\n    call();\n}\n";
        let new = "if (x)\n{\n        call();\n}\n";
        let options = DiffOptions {
            ignore_indentation: true,
            ..DiffOptions::default()
        };
        assert!(
            diff(old.as_bytes(), new.as_bytes(), options)
                .hunks
                .is_empty()
        );
        assert_eq!(headers(old, new).len(), 1);
    }
}
