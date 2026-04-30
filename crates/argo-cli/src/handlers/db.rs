//! `argo db` — schema migration ops.
use crate::cli::DbCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::ArgoResult;

pub async fn run(cmd: DbCmd) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| argo_types::ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    match cmd {
        DbCmd::Status => {
            let v = store.schema_version().await?;
            println!("schema_version: {v}");
            println!("db_path: {}", db.display());
        }
        DbCmd::MigrateUp => {
            let before = store.schema_version().await?;
            store.migrate().await?;
            let after = store.schema_version().await?;
            println!("migrated: {before} -> {after}");
        }
        DbCmd::MigrateDown { to } => {
            let before = store.schema_version().await?;
            store.migrate_down_to(to).await?;
            let after = store.schema_version().await?;
            println!("rolled back: {before} -> {after}");
        }
        DbCmd::Prune {
            older_than_days,
            force,
        } => {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);
            let cutoff_micros = cutoff.timestamp_micros();
            if !force {
                let count = store.count_sessions_before(cutoff_micros).await?;
                println!(
                    "dry run: would delete {count} session(s) older than {} ({}d ago).",
                    cutoff, older_than_days
                );
                println!("re-run with --force to actually drop them.");
            } else {
                let n = store.prune_sessions_before(cutoff_micros).await?;
                println!(
                    "pruned {n} session(s) older than {} ({}d ago); cascaded children dropped.",
                    cutoff, older_than_days
                );
            }
        }
    }
    Ok(())
}
