//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{export_ndjson, tail_rows, verify_all};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
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
        AuditCmd::Verify { from: _, to: _ } => {
            let report = verify_all(&store).await?;
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
        AuditCmd::Show { tail } => {
            let report = verify_all(&store).await?;
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
            if let Some(n) = tail {
                let rows = tail_rows(&store, n).await?;
                println!("\nlast {} rows:", rows.len());
                for r in rows {
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
