//! `argo audit verify` — chain verification for a single session
//! or every session in the store. Format dispatch (json / markdown /
//! text) lives here so `audit::run` stays under the fn-LOC budget.
use argo_audit::{verify_all, verify_session, VerifyReport};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

pub(super) async fn run(
    store: &Arc<SessionStore>,
    session: Option<String>,
    json: bool,
    format: String,
) -> ArgoResult<()> {
    let scope = session.clone();
    let report = match session {
        Some(raw) => {
            let id = uuid::Uuid::parse_str(&raw)
                .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
            verify_session(store, SessionId(id)).await?
        }
        None => verify_all(store).await?,
    };
    let pick = if json { "json" } else { format.as_str() };
    match pick {
        "json" => print_json(&report)?,
        "markdown" => print_markdown(&report, scope.as_deref()),
        "text" => print_text(&report),
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if report.ok {
        Ok(())
    } else {
        Err(ArgoError::Provider(
            "audit chain verification failed".into(),
        ))
    }
}

fn print_json(report: &VerifyReport) -> ArgoResult<()> {
    let breaks_arr: Vec<_> = report
        .breaks
        .iter()
        .map(|b| {
            serde_json::json!({
                "seq": b.seq,
                "kind": b.kind,
                "reason": b.reason,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "ok": report.ok,
        "rows": report.rows,
        "kinds": report.kind_counts.len(),
        "breaks": breaks_arr,
    });
    let serialized = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize verify: {e}")))?;
    println!("{serialized}");
    Ok(())
}

fn print_markdown(report: &VerifyReport, scope: Option<&str>) {
    println!("# Audit verify");
    println!();
    if let Some(s) = scope {
        println!("- **Scope**: session `{s}`");
    } else {
        println!("- **Scope**: all sessions");
    }
    println!(
        "- **Status**: {}",
        if report.ok { "OK ✓" } else { "**BROKEN**" }
    );
    println!("- **Rows**: {}", report.rows);
    println!("- **Kinds**: {}", report.kind_counts.len());
    println!("- **Breaks**: {}", report.breaks.len());
    if !report.breaks.is_empty() {
        println!();
        println!("## Break detail");
        println!();
        println!("| seq | kind | reason |");
        println!("| ---: | --- | --- |");
        for b in &report.breaks {
            let reason = b.reason.replace('|', "\\|");
            println!("| {} | `{}` | {reason} |", b.seq, b.kind);
        }
    }
}

fn print_text(report: &VerifyReport) {
    if report.ok {
        println!(
            "audit chain ok: {} rows verified across {} kinds",
            report.rows,
            report.kind_counts.len()
        );
    } else {
        println!(
            "audit chain broken: {} break(s) detected across {} rows",
            report.breaks.len(),
            report.rows
        );
        for b in &report.breaks {
            println!("  seq {} kind={} ({})", b.seq, b.kind, b.reason);
        }
    }
}
