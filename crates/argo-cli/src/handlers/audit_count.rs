//! `argo audit count` + `argo audit breaks` impls. Lifted out
//! of `audit.rs` so that file stays under the 500-LOC budget.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_types::{ArgoError, ArgoResult};

/// `argo audit count` — mirror of `GET /audit/count` (#458).
/// Caps the underlying fetch at 10k rows; for any realistic
/// chain this is plenty.
pub(super) async fn print_count(
    kind: Option<String>,
    since_minutes: Option<i64>,
    session: Option<String>,
    json: bool,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = std::sync::Arc::new(argo_store::SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let since_micros = since_minutes.map(|m| {
        (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
    });
    let session_bytes = match session.as_deref() {
        Some(s) => Some(
            uuid::Uuid::parse_str(s)
                .map_err(|e| ArgoError::Provider(format!("invalid session id '{s}': {e}")))?
                .as_bytes()
                .to_vec(),
        ),
        None => None,
    };
    let filters = argo_audit::TailFilters {
        kind: kind.as_deref(),
        since_micros,
        session_id: session_bytes.as_deref(),
    };
    let rows = argo_audit::tail_rows_filtered(&store, 10_000, &filters).await?;
    let n = rows.len();
    if json {
        let payload = serde_json::json!({
            "count": n,
            "kind": kind,
            "session": session,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{n}");
    }
    Ok(())
}

/// `argo audit breaks [--json]` — print just the chain-break
/// rows. Mirrors `GET /audit/breaks` (#406). Empty output (or
/// `[]` with `--json`) when the chain is clean.
pub(super) async fn print_breaks(json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = std::sync::Arc::new(argo_store::SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let report = argo_audit::verify_all(&store).await?;
    if json {
        let arr: Vec<serde_json::Value> = report
            .breaks
            .iter()
            .map(|b| {
                serde_json::json!({
                    "seq":    b.seq,
                    "kind":   b.kind,
                    "reason": b.reason,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        for b in &report.breaks {
            println!("[{}] {} — {}", b.seq, b.kind, b.reason);
        }
        if report.breaks.is_empty() {
            eprintln!("(audit chain is clean)");
        }
    }
    Ok(())
}
