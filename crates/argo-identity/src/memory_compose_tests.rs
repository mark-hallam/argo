//! Unit tests for `MemoryNs::compose_context` formatting (Phase B
//! mission-wire-memory-compose-context).
//!
//! Loaded via `#[cfg(test)] #[path = "memory_compose_tests.rs"] mod tests;`
//! from `memory_compose.rs`. Tests use the pure parameterized helper
//! `compose(rows, top_n)` to stay parallel-safe — env-var resolution
//! is exercised at the call site (`MemoryNs::compose_context` doctest).

use super::{compose, DEFAULT_TOP_N};
use chrono::{TimeZone, Utc};

fn ts(year: i32, month: u32, day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap()
}

#[test]
fn empty_rows_render_as_empty_string() {
    assert_eq!(compose(&[], DEFAULT_TOP_N), "");
}

#[test]
fn single_row_renders_as_one_bullet() {
    let rows = vec![(ts(2026, 5, 7), "single entry".into())];
    let out = compose(&rows, DEFAULT_TOP_N);
    assert!(
        out.starts_with("## Memory (workspace-shared, last 1)\n"),
        "got: {out}"
    );
    assert!(out.contains("- 2026-05-07: single entry\n"), "got: {out}");
}

#[test]
fn eight_rows_all_appear_in_block_newest_first() {
    let rows: Vec<_> = (1..=8)
        .map(|d| (ts(2026, 1, d), format!("entry {d}")))
        .collect();
    let out = compose(&rows, DEFAULT_TOP_N);
    assert!(
        out.starts_with("## Memory (workspace-shared, last 8)\n"),
        "got: {out}"
    );
    for d in 1..=8 {
        assert!(
            out.contains(&format!("entry {d}")),
            "expected 'entry {d}' in:\n{out}"
        );
    }
    let pos_8 = out.find("entry 8").expect("entry 8 present");
    let pos_1 = out.find("entry 1").expect("entry 1 present");
    assert!(
        pos_8 < pos_1,
        "expected newest first; entry 8 should precede entry 1"
    );
}

#[test]
fn twenty_rows_truncate_to_top_eight_most_recent() {
    let rows: Vec<_> = (1..=20)
        .map(|d| (ts(2026, 1, d), format!("entry {d}")))
        .collect();
    let out = compose(&rows, DEFAULT_TOP_N);
    assert!(
        out.contains("## Memory (workspace-shared, last 8)\n"),
        "got: {out}"
    );
    for d in 13..=20 {
        assert!(
            out.contains(&format!("entry {d}")),
            "expected 'entry {d}' in:\n{out}"
        );
    }
    for d in 1..=12 {
        assert!(
            !out.contains(&format!("- 2026-01-{:02}: entry {d}", d)),
            "did not expect 'entry {d}' in:\n{out}"
        );
    }
}
