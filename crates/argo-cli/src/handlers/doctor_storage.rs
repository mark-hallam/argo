//! Storage-related checks for `argo doctor` (db, audit chain, WAL stats).

use crate::home::{db_path, resolve_home};
use argo_config::ArgoConfig;
use argo_store::wal_tuning::{self, WalTuningProfile};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;
use std::str::FromStr;

pub(super) async fn db_status(home: &Path) -> serde_json::Value {
    let db = db_path(home);
    if !db.exists() {
        return serde_json::json!({"present": false});
    }
    let store = match SessionStore::open(&db).await {
        Ok(s) => s,
        Err(e) => return serde_json::json!({"present": true, "open_error": e.to_string()}),
    };
    if let Err(e) = store.migrate().await {
        return serde_json::json!({"present": true, "migrate_error": e.to_string()});
    }
    let version = store.schema_version().await.unwrap_or(0);
    let workspace_ok = matches!(store.first_workspace().await, Ok(Some(_)));
    let stats = store.db_stats().await.ok();
    serde_json::json!({
        "present": true,
        "schema_version": version,
        "workspace_registered": workspace_ok,
        "rows": stats.map(|s| serde_json::json!({
            "sessions": s.sessions,
            "messages": s.messages,
            "audit": s.audit_rows,
            "memory": s.memory_entries,
            "skills": s.skills,
            "forge_runs": s.forge_runs,
            "spend_rows": s.spend_rows,
        })),
    })
}

pub(super) fn print_db(v: &serde_json::Value) {
    let present = v.get("present").and_then(|b| b.as_bool()).unwrap_or(false);
    if !present {
        println!("  db:         (no sessions.db yet — run `argo workspace init`)");
        return;
    }
    if let Some(e) = v.get("open_error").and_then(|s| s.as_str()) {
        println!("  db:         OPEN FAILED ({e})");
        return;
    }
    if let Some(e) = v.get("migrate_error").and_then(|s| s.as_str()) {
        println!("  db:         MIGRATE FAILED ({e})");
        return;
    }
    let version = v.get("schema_version").and_then(|n| n.as_u64()).unwrap_or(0);
    let ws = v
        .get("workspace_registered")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    println!(
        "  db:         schema v{version}  workspace={}",
        if ws { "registered" } else { "MISSING" }
    );
    if let Some(rows) = v.get("rows") {
        let n = |k| rows.get(k).and_then(|n| n.as_u64()).unwrap_or(0);
        println!(
            "  rows:       sessions={} messages={} audit={} memory={} skills={}",
            n("sessions"),
            n("messages"),
            n("audit"),
            n("memory"),
            n("skills")
        );
    }
}

/// Report `{ok, rows, breaks}` for the global audit chain so
/// `argo doctor --json` can be wired into CI gates that should
/// fail when the chain is broken. Errors silently degrade to
/// `{"available": false}`.
pub(super) async fn audit_status(home: &Path) -> serde_json::Value {
    let db = db_path(home);
    let store = match SessionStore::open(&db).await {
        Ok(s) => std::sync::Arc::new(s),
        Err(_) => return serde_json::json!({"available": false}),
    };
    if store.migrate().await.is_err() {
        return serde_json::json!({"available": false});
    }
    let report = match argo_audit::verify_all(&store).await {
        Ok(r) => r,
        Err(_) => return serde_json::json!({"available": false}),
    };
    serde_json::json!({
        "available": true,
        "ok": report.ok,
        "rows": report.rows,
        "breaks": report.breaks.len(),
    })
}

pub(super) fn print_audit(v: &serde_json::Value) {
    if !v["available"].as_bool().unwrap_or(false) {
        println!("  audit:      (store not available)");
        return;
    }
    let ok = v["ok"].as_bool().unwrap_or(false);
    let rows = v["rows"].as_u64().unwrap_or(0);
    let breaks = v["breaks"].as_u64().unwrap_or(0);
    let status = if ok { "OK" } else { "BROKEN" };
    println!("  audit:      {status} ({rows} row(s), {breaks} break(s))");
}

/// `argo doctor wal-stats` — checkpoint the WAL (PASSIVE) and report
/// the active WAL pragmas plus the `*.db-wal` file size. The store is
/// opened with the profile from `[store.wal_tuning].profile` so the
/// readout reflects what the runtime would actually apply.
pub(super) async fn wal_stats(json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
    let db = db_path(&home);
    let profile = WalTuningProfile::from_str(&cfg.store.wal_tuning.profile)?;
    let store = SessionStore::open_with_profile(&db, profile).await?;
    sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("wal_checkpoint: {e}")))?;
    let mut readouts: Vec<(&'static str, String)> = Vec::new();
    for name in ["journal_mode", "synchronous", "cache_size", "mmap_size"] {
        readouts.push((name, wal_tuning::read_pragma(store.pool(), name).await?));
    }
    let wal_path = db.with_extension("db-wal");
    let wal_bytes = std::fs::metadata(&wal_path).ok().map(|m| m.len());
    if json {
        let pragmas: serde_json::Map<String, serde_json::Value> = readouts
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::Value::String(v.clone())))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "db": db.display().to_string(),
                "profile": cfg.store.wal_tuning.profile,
                "pragmas": pragmas,
                "wal_path": wal_path.display().to_string(),
                "wal_bytes": wal_bytes,
            }))
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
        return Ok(());
    }
    println!("argo doctor wal-stats");
    println!("  db:         {}", db.display());
    println!("  profile:    {}", cfg.store.wal_tuning.profile);
    for (k, v) in &readouts {
        println!("  {k:11}: {v}");
    }
    match wal_bytes {
        Some(n) => println!("  wal_bytes:  {n}  ({})", wal_path.display()),
        None => println!("  wal_bytes:  (no *.db-wal — checkpointed or memory)"),
    }
    Ok(())
}
