//! Naive line-diff used by `argo workflow def diff` (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! The brief calls for `similar`-crate unified diffs with a naive
//! fallback "if `similar` is unavailable". `similar` is NOT in the
//! workspace today (`cargo metadata` returns no row), so this module
//! is the production path — a small LCS-based implementation that
//! emits a unified-diff string and a structured `{added, removed,
//! changed}` summary. Self-contained: no extra crate deps, and no
//! `argo-store` widening.
//!
//! The pivot-line accounting is intentionally simple: any line in
//! the left side that is absent on the right is "removed"; any line
//! in the right that is absent on the left is "added"; entries that
//! appear in both with the same position offset are unchanged. Cases
//! where the same source line is duplicated do round-trip cleanly
//! because the LCS table indexes by `(left_idx, right_idx)`.

use std::collections::HashSet;

/// Structured form of a diff, suitable for `--json` consumers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineDiff {
    /// Lines present only on the right side.
    pub added: Vec<String>,
    /// Lines present only on the left side.
    pub removed: Vec<String>,
    /// Stable summary count of lines that changed across both
    /// sides — currently `added.len() + removed.len()`, retained as
    /// a separate field so future implementations can refine
    /// (`a → b` style edits) without breaking the JSON envelope.
    pub changed: usize,
}

/// Compute the additive / subtractive summary between two TOML
/// bodies. Whitespace-only lines are kept verbatim so operators can
/// see structural moves.
#[must_use]
pub fn compute_line_diff(a: &str, b: &str) -> LineDiff {
    let left: Vec<&str> = a.lines().collect();
    let right: Vec<&str> = b.lines().collect();
    let lcs = lcs_indices(&left, &right);
    let left_in_lcs: HashSet<usize> = lcs.iter().map(|(l, _)| *l).collect();
    let right_in_lcs: HashSet<usize> = lcs.iter().map(|(_, r)| *r).collect();
    let removed: Vec<String> = left
        .iter()
        .enumerate()
        .filter(|(i, _)| !left_in_lcs.contains(i))
        .map(|(_, s)| (*s).to_string())
        .collect();
    let added: Vec<String> = right
        .iter()
        .enumerate()
        .filter(|(i, _)| !right_in_lcs.contains(i))
        .map(|(_, s)| (*s).to_string())
        .collect();
    let changed = added.len() + removed.len();
    LineDiff {
        added,
        removed,
        changed,
    }
}

/// Render a unified-diff-style block walking both sides in order.
/// Lines unique to the left are prefixed with `- `; lines unique to
/// the right with `+ `; common lines with ` ` (space). The output
/// ends in a newline so callers can concatenate freely.
#[must_use]
pub fn format_unified_diff(a: &str, b: &str) -> String {
    let left: Vec<&str> = a.lines().collect();
    let right: Vec<&str> = b.lines().collect();
    let lcs = lcs_indices(&left, &right);
    let mut out = String::new();
    let mut i = 0_usize;
    let mut j = 0_usize;
    let mut lcs_iter = lcs.into_iter().peekable();
    while i < left.len() || j < right.len() {
        match lcs_iter.peek().copied() {
            Some((li, rj)) if li == i && rj == j => {
                out.push(' ');
                out.push_str(left[i]);
                out.push('\n');
                i += 1;
                j += 1;
                lcs_iter.next();
            }
            Some((li, _)) if li == i => {
                // Left already at the next LCS anchor — emit only
                // the right-side line.
                out.push_str("+ ");
                out.push_str(right[j]);
                out.push('\n');
                j += 1;
            }
            Some((_, rj)) if rj == j => {
                out.push_str("- ");
                out.push_str(left[i]);
                out.push('\n');
                i += 1;
            }
            Some(_) => {
                if i < left.len() {
                    out.push_str("- ");
                    out.push_str(left[i]);
                    out.push('\n');
                    i += 1;
                } else if j < right.len() {
                    out.push_str("+ ");
                    out.push_str(right[j]);
                    out.push('\n');
                    j += 1;
                }
            }
            None => {
                if i < left.len() {
                    out.push_str("- ");
                    out.push_str(left[i]);
                    out.push('\n');
                    i += 1;
                } else if j < right.len() {
                    out.push_str("+ ");
                    out.push_str(right[j]);
                    out.push('\n');
                    j += 1;
                }
            }
        }
    }
    out
}

/// O(n·m) LCS — fine for workflow TOML bodies (tens to low-hundreds
/// of lines). Returns the matched index pairs in increasing order.
fn lcs_indices(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let mut dp = vec![vec![0_u32; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            dp[i + 1][j + 1] = if a[i] == b[j] {
                dp[i][j] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            out.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical_inputs_is_empty() {
        let d = compute_line_diff("alpha\nbeta\n", "alpha\nbeta\n");
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
        assert_eq!(d.changed, 0);
    }

    #[test]
    fn diff_detects_added_and_removed_lines() {
        let a = "alpha\nbeta\ngamma\n";
        let b = "alpha\ndelta\ngamma\n";
        let d = compute_line_diff(a, b);
        assert_eq!(d.removed, vec!["beta".to_string()]);
        assert_eq!(d.added, vec!["delta".to_string()]);
        assert_eq!(d.changed, 2);
    }

    #[test]
    fn unified_diff_marks_added_removed_unchanged() {
        let a = "alpha\nbeta\n";
        let b = "alpha\ngamma\n";
        let u = format_unified_diff(a, b);
        assert!(u.contains(" alpha\n"));
        assert!(u.contains("- beta\n"));
        assert!(u.contains("+ gamma\n"));
    }

    #[test]
    fn empty_inputs_produce_empty_output() {
        let u = format_unified_diff("", "");
        assert!(u.is_empty());
        let d = compute_line_diff("", "");
        assert_eq!(d, LineDiff::default());
    }
}
