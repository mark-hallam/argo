//! Snapshot tests for the deps handler renderers. The store-side
//! invariants (cycle detection, impact closure) live in
//! `argo_store::worker_deps_tests`; here we lock the rendered surface
//! the operator copy-pastes into graphviz / docs.

use super::*;
use argo_store::worker_deps::{DepKind, DepRow, ImpactedWorker};

fn row(a: &str, b: &str, kind: DepKind, note: Option<&str>) -> DepRow {
    DepRow {
        a_worker_id: a.into(),
        b_worker_id: b.into(),
        kind,
        declared_at: 1_700_000_000_000_000,
        note: note.map(str::to_string),
    }
}

#[test]
fn render_dot_snapshot_matches_brief_color_legend() {
    let rows = vec![
        row("A", "B", DepKind::Data, None),
        row("B", "C", DepKind::Approval, Some("month-end")),
        row("C", "D", DepKind::Handoff, None),
        row("D", "E", DepKind::Skill, None),
    ];
    let dot = render_dot(&rows);
    let expected = "digraph worker_deps {\n  \
        rankdir=LR;\n  \
        node [shape=box, style=rounded];\n  \
        \"A\";\n  \
        \"B\";\n  \
        \"C\";\n  \
        \"D\";\n  \
        \"E\";\n  \
        \"A\" -> \"B\" [color=\"blue\", label=\"data\"];\n  \
        \"B\" -> \"C\" [color=\"red\", label=\"approval\"];\n  \
        \"C\" -> \"D\" [color=\"green\", label=\"handoff\"];\n  \
        \"D\" -> \"E\" [color=\"purple\", label=\"skill\"];\n}\n";
    assert_eq!(dot, expected);
}

#[test]
fn render_dot_escapes_quotes_in_worker_ids() {
    let rows = vec![row("alpha\"bravo", "charlie", DepKind::Data, None)];
    let dot = render_dot(&rows);
    assert!(
        dot.contains("\"alpha\\\"bravo\""),
        "quote escape missing:\n{dot}"
    );
}

#[test]
fn render_list_markdown_handles_empty_set() {
    let md = render_list_markdown("X", argo_store::worker_deps::Direction::Out, &[]);
    assert!(md.contains("# Worker X dependencies (out)"));
    assert!(md.contains("_No dependencies declared._"));
}

#[test]
fn render_impact_markdown_renders_depths() {
    let entries = vec![
        ImpactedWorker { depth: 1, worker_id: "B".into() },
        ImpactedWorker { depth: 2, worker_id: "C".into() },
    ];
    let md = render_impact_markdown("A", &entries);
    assert!(md.contains("Workers impacted by A outage"));
    assert!(md.contains("Total impacted: **2**"));
    assert!(md.contains("| 1 | B |"));
    assert!(md.contains("| 2 | C |"));
}
