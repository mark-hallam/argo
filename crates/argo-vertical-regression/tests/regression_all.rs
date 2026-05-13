//! Per-vertical regression — workspace-level integration test.
//!
//! Walks the on-disk `verticals/` directory and cross-checks every
//! fixture's `expected.action_type` (and approval flag) against
//! the matching vertical manifest. New drift fails the test; the
//! `KNOWN_DRIFT` baseline in `lib.rs` carries the small set of
//! pre-existing inconsistencies the brief's may-touch list
//! forbids fixing inside this PR.

use argo_vertical_regression::{check, discover, partition, workspace_root};

#[test]
fn every_fixture_matches_manifest_actions_vocab() {
    let verticals_dir = workspace_root().join("verticals");
    let cases = discover(&verticals_dir);
    assert!(
        !cases.is_empty(),
        "expected at least one role-bearing vertical under {}",
        verticals_dir.display()
    );

    let mut violations = Vec::new();
    for case in &cases {
        violations.extend(check(case));
    }

    let (new_violations, retired_baseline) = partition(&violations);

    if new_violations.is_empty() && retired_baseline.is_empty() {
        return;
    }

    let mut msg = String::from("per-vertical regression failed:");
    if !new_violations.is_empty() {
        msg.push_str(&format!(
            "\n\nNEW violations ({}): drift introduced by this branch (or first seen here).",
            new_violations.len()
        ));
        for v in &new_violations {
            msg.push_str("\n  - ");
            msg.push_str(&v.render());
        }
    }
    if !retired_baseline.is_empty() {
        msg.push_str(&format!(
            "\n\nRETIRED baseline ({}): the following KNOWN_DRIFT rows in src/lib.rs no longer reproduce — please delete them in the same PR that fixed the underlying drift.",
            retired_baseline.len()
        ));
        for (v, a) in &retired_baseline {
            msg.push_str(&format!("\n  - ({v}, {a})"));
        }
    }
    panic!("{msg}");
}
