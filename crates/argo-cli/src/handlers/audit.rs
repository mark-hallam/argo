//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{export_ndjson, tail_rows, verify_all, verify_session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::io::{BufWriter, Write};
use std::sync::Arc;

pub async fn run(cmd: AuditCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    match cmd {
        AuditCmd::Verify {
            from: _,
            to: _,
            session,
            json,
        } => {
            let report = match session {
                Some(raw) => {
                    let id = uuid::Uuid::parse_str(&raw).map_err(|e| {
                        ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                    })?;
                    verify_session(&store, SessionId(id)).await?
                }
                None => verify_all(&store).await?,
            };
            if json {
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
                if !report.ok {
                    return Err(ArgoError::Provider(
                        "audit chain verification failed".into(),
                    ));
                }
                return Ok(());
            }
            if report.ok {
                println!(
                    "audit chain ok: {} rows verified across {} kinds",
                    report.rows,
                    report.kind_counts.len()
                );
                Ok(())
            } else {
                println!(
                    "audit chain broken: {} break(s) detected across {} rows",
                    report.breaks.len(),
                    report.rows
                );
                for b in &report.breaks {
                    println!("  seq {} kind={} ({})", b.seq, b.kind, b.reason);
                }
                Err(ArgoError::Provider(
                    "audit chain verification failed".into(),
                ))
            }
        }
        AuditCmd::Show { tail, json } => {
            let report = verify_all(&store).await?;
            let tail_rows_data = match tail {
                Some(n) => tail_rows(&store, n).await?,
                None => Vec::new(),
            };
            if json {
                let kinds_obj: serde_json::Map<String, serde_json::Value> = report
                    .kind_counts
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                    .collect();
                let tail_arr: Vec<_> = tail_rows_data
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "seq": r.seq,
                            "kind": r.kind,
                            "session_id": r.session_id,
                        })
                    })
                    .collect();
                let payload = serde_json::json!({
                    "rows": report.rows,
                    "ok": report.ok,
                    "breaks": report.breaks.len(),
                    "kinds": kinds_obj,
                    "tail": tail_arr,
                });
                let serialized = serde_json::to_string(&payload)
                    .map_err(|e| ArgoError::Provider(format!("serialize audit: {e}")))?;
                println!("{serialized}");
                return Ok(());
            }
            println!("audit log — {} rows", report.rows);
            let mut kinds: Vec<(&String, &usize)> = report.kind_counts.iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            for (kind, count) in kinds {
                println!("  {count:>6}  {kind}");
            }
            if !report.ok {
                println!(
                    "(chain has {} break(s); run `argo audit verify` for detail)",
                    report.breaks.len()
                );
            }
            if !tail_rows_data.is_empty() {
                println!("\nlast {} rows:", tail_rows_data.len());
                for r in &tail_rows_data {
                    println!(
                        "  seq={:<6} kind={:<22} session={}",
                        r.seq,
                        r.kind,
                        r.session_id.as_deref().unwrap_or("-")
                    );
                }
            }
            Ok(())
        }
        AuditCmd::Export { format, out } => {
            if format != "ndjson" {
                return Err(ArgoError::Provider(format!(
                    "unsupported audit export format '{format}' (only 'ndjson' for now)"
                )));
            }
            let mut writer: Box<dyn Write> = match out.as_deref() {
                Some(path) => {
                    let f = std::fs::File::create(path)
                        .map_err(|e| ArgoError::Provider(format!("create {path}: {e}")))?;
                    Box::new(BufWriter::new(f))
                }
                None => Box::new(BufWriter::new(std::io::stdout())),
            };
            let n = export_ndjson(&store, |row| {
                let line = serde_json::to_string(row)
                    .map_err(|e| ArgoError::Provider(format!("serialize row: {e}")))?;
                writer
                    .write_all(line.as_bytes())
                    .and_then(|()| writer.write_all(b"\n"))
                    .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
                Ok(())
            })
            .await?;
            writer
                .flush()
                .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
            // Only print the summary if we wrote to a file (stdout
            // already has the rows).
            if out.is_some() {
                eprintln!("audit export: wrote {n} rows");
            }
            Ok(())
        }
    }
}
