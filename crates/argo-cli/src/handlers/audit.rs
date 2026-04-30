//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::verify_all;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
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
        AuditCmd::Show => {
            println!("argo audit show: render-only path lands with the TUI; use --format ndjson for export.");
            Ok(())
        }
        AuditCmd::Export { format } => {
            // ndjson is the canonical export format per §17.
            if format != "ndjson" {
                eprintln!("warning: only ndjson export is implemented; got '{format}'");
            }
            println!(
                "audit export not yet wired to stdout streaming; the daemon writes ndjson per §17.4."
            );
            Ok(())
        }
    }
}
