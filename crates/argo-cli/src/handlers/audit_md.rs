//! Markdown render of `argo audit show`. Extracted from
//! `audit.rs` so that file stays under the 500-LOC budget. Two
//! shapes — per-session (full chain) and global (histogram +
//! optional tail) — both as shareable markdown docs.

use argo_audit::{VerifyReport, ExportedRow};

use super::audit::{payload_detail, SessionSummary};

/// Per-session render: header + summary bullet list + full
/// `seq | kind | detail` table.
pub(super) fn render_session(raw: &str, rows: &[ExportedRow], summary: &SessionSummary) {
    println!("# Audit chain — session `{raw}`");
    println!();
    println!("- **Rows**: {}", rows.len());
    println!("- **Turns**: {}", summary.turns);
    println!(
        "- **Tool calls**: {} ({} error(s))",
        summary.tool_calls, summary.tool_errors
    );
    println!(
        "- **Tokens**: {} prompt + {} completion",
        summary.tokens_in, summary.tokens_out
    );
    println!();
    if rows.is_empty() {
        return;
    }
    println!("| seq | kind | detail |");
    println!("| ---: | --- | --- |");
    for r in rows {
        let detail = payload_detail(&r.kind, &r.payload).replace('|', "\\|");
        println!("| {} | `{}` | {detail} |", r.seq, r.kind);
    }
}

/// Global render: header + status bullets + kind histogram +
/// optional tail table.
pub(super) fn render_global(report: &VerifyReport, tail_rows: &[ExportedRow]) {
    println!("# Audit log");
    println!();
    println!("- **Total rows**: {}", report.rows);
    println!(
        "- **Status**: {}",
        if report.ok { "OK ✓" } else { "**BROKEN**" }
    );
    println!("- **Breaks**: {}", report.breaks.len());
    println!();
    if !report.kind_counts.is_empty() {
        println!("## Histogram");
        println!();
        println!("| kind | count |");
        println!("| --- | ---: |");
        let mut kinds: Vec<(&String, &usize)> = report.kind_counts.iter().collect();
        kinds.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (kind, count) in kinds {
            println!("| `{kind}` | {count} |");
        }
        println!();
    }
    if !tail_rows.is_empty() {
        println!("## Tail ({} rows)", tail_rows.len());
        println!();
        println!("| seq | kind | detail |");
        println!("| ---: | --- | --- |");
        for r in tail_rows {
            let detail = payload_detail(&r.kind, &r.payload).replace('|', "\\|");
            println!("| {} | `{}` | {detail} |", r.seq, r.kind);
        }
    }
}
