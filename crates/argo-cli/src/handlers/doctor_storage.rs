//! Storage-related checks for `argo doctor` (db, audit chain, WAL stats).

use crate::home::{db_path, resolve_home};
use argo_config::ArgoConfig;
use argo_store::wal_tuning::{self, WalTuningProfile};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

pub(super) async fn db_status(home: &Path) -> serde_json::Value {
    let db = db_path(home);
    let encryption = encryption_status();
    if !db.exists() {
        return serde_json::json!({"present": false, "encryption": encryption});
    }
    let store = match SessionStore::open(&db).await {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({
                "present": true,
                "open_error": e.to_string(),
                "encryption": encryption,
            })
        }
    };
    if let Err(e) = store.migrate().await {
        return serde_json::json!({
            "present": true,
            "migrate_error": e.to_string(),
            "encryption": encryption,
        });
    }
    let version = store.schema_version().await.unwrap_or(0);
    let workspace_ok = matches!(store.first_workspace().await, Ok(Some(_)));
    let stats = store.db_stats().await.ok();
    serde_json::json!({
        "present": true,
        "schema_version": version,
        "workspace_registered": workspace_ok,
        "encryption": encryption,
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

/// Report at-rest-encryption posture for `argo doctor`. v1 stock
/// builds ship the pure-Rust `sqlx` SQLite driver which is NOT
/// linked against SQLCipher, so `PRAGMA key` is silently dropped
/// and pages are written plaintext regardless of whether
/// `[security.encryption] key_source` is configured. A configured
/// key is therefore necessary-but-not-sufficient; the operator
/// also needs a SQLCipher-linked downstream build. See
/// `docs/skycot/security/at-rest-encryption.md`.
fn encryption_status() -> serde_json::Value {
    let cwd = std::env::current_dir().unwrap_or_default();
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let key_source = cfg.security.encryption.key_source.clone();
    let configured = !key_source.eq_ignore_ascii_case("none") && !key_source.is_empty();
    let acknowledged = cfg.security.encryption.acknowledge_plaintext;
    // v1 stock build always reports OFF; the configured-but-stock case
    // is the most dangerous (operator thinks they're encrypted, they
    // are not) so it's called out explicitly.
    let state = if configured {
        "OFF (key configured but v1 stock build ships pure-Rust sqlx — \
         driver silently drops PRAGMA key; ship a SQLCipher-linked build)"
    } else if acknowledged {
        "OFF (plaintext; acknowledged via acknowledge_plaintext = true)"
    } else {
        "OFF (plaintext — key_source = \"none\"; \
         see docs/skycot/security/at-rest-encryption.md)"
    };
    serde_json::json!({
        "state": state,
        "key_source": key_source,
        "acknowledge_plaintext": acknowledged,
    })
}

pub(super) fn print_db(v: &serde_json::Value) {
    let present = v.get("present").and_then(|b| b.as_bool()).unwrap_or(false);
    if !present {
        println!("  db:         (no sessions.db yet — run `argo workspace init`)");
        print_encryption(v);
        return;
    }
    if let Some(e) = v.get("open_error").and_then(|s| s.as_str()) {
        println!("  db:         OPEN FAILED ({e})");
        print_encryption(v);
        return;
    }
    if let Some(e) = v.get("migrate_error").and_then(|s| s.as_str()) {
        println!("  db:         MIGRATE FAILED ({e})");
        print_encryption(v);
        return;
    }
    let version = v
        .get("schema_version")
        .and_then(|n| n.as_u64())
        .unwrap_or(0);
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
    print_encryption(v);
}

fn print_encryption(v: &serde_json::Value) {
    let Some(enc) = v.get("encryption") else {
        return;
    };
    let state = enc
        .get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN");
    println!("  encryption: {state}");
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

pub(super) async fn capacity_status(home: &Path, cwd: &Path) -> serde_json::Value {
    let warn_pct = doctor_u64(cwd, "storage_warn_pct", 80);
    let fail_pct = doctor_u64(cwd, "storage_fail_pct", 95);
    let disk_path = if home.exists() {
        home
    } else {
        home.parent().unwrap_or_else(|| Path::new("."))
    };
    let disk = disk_usage(disk_path);
    let mut status = match disk {
        Some((_, _, used_pct)) if used_pct >= fail_pct => "FAIL",
        Some((_, _, used_pct)) if used_pct >= warn_pct => "WARN",
        Some(_) => "OK",
        None => "WARN",
    };
    let db = db_path(home);
    let (audit_rows, memory_rows, audit_bytes, memory_bytes, db_error) =
        match table_stats(&db).await {
            Ok(v) => v,
            Err(e) => {
                status = "FAIL";
                (0, 0, None, None, Some(e))
            }
        };
    serde_json::json!({
        "status": status,
        "path": home.display().to_string(),
        "total_bytes": disk.map(|d| d.0),
        "free_bytes": disk.map(|d| d.1),
        "used_pct": disk.map(|d| d.2),
        "warn_pct": warn_pct,
        "fail_pct": fail_pct,
        "tables": {
            "audit_record": {"rows": audit_rows, "bytes": audit_bytes},
            "memory_row": {"rows": memory_rows, "bytes": memory_bytes}
        },
        "suggested_action": if status == "OK" { serde_json::Value::Null } else { serde_json::json!("run `argo gdpr delete` or `argo audit gc`") },
        "error": db_error,
    })
}

pub(super) fn print_capacity(v: &serde_json::Value) {
    println!("  Storage capacity:");
    println!(
        "    {:<4} used={}%, free={} bytes, audit_rows={}, memory_rows={}",
        v["status"].as_str().unwrap_or("WARN"),
        v["used_pct"].as_u64().unwrap_or(0),
        v["free_bytes"].as_u64().unwrap_or(0),
        v["tables"]["audit_record"]["rows"].as_i64().unwrap_or(0),
        v["tables"]["memory_row"]["rows"].as_i64().unwrap_or(0)
    );
}

async fn table_stats(
    db: &Path,
) -> Result<(i64, i64, Option<i64>, Option<i64>, Option<String>), String> {
    if !db.exists() {
        return Ok((0, 0, None, None, None));
    }
    let store = SessionStore::open(db)
        .await
        .map_err(|e| format!("open store: {e}"))?;
    store.migrate().await.map_err(|e| format!("migrate: {e}"))?;
    let stats = store
        .db_stats()
        .await
        .map_err(|e| format!("db stats: {e}"))?;
    let audit_bytes = dbstat_bytes(&store, "audit_record").await;
    let memory_bytes = dbstat_bytes(&store, "memory_entry").await;
    Ok((
        stats.audit_rows,
        stats.memory_entries,
        audit_bytes,
        memory_bytes,
        None,
    ))
}

async fn dbstat_bytes(store: &SessionStore, table: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name = ?")
        .bind(table)
        .fetch_one(store.pool())
        .await
        .ok()
}

fn disk_usage(path: &Path) -> Option<(u64, u64, u64)> {
    let out = Command::new("df").arg("-Pk").arg(path).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let cols: Vec<&str> = text.lines().nth(1)?.split_whitespace().collect();
    let total = cols.get(1)?.parse::<u64>().ok()?.saturating_mul(1024);
    let free = cols.get(3)?.parse::<u64>().ok()?.saturating_mul(1024);
    let used_pct = cols.get(4)?.trim_end_matches('%').parse::<u64>().ok()?;
    Some((total, free, used_pct))
}

fn doctor_u64(cwd: &Path, key: &str, default: u64) -> u64 {
    std::fs::read_to_string(cwd.join("argo.toml"))
        .ok()
        .and_then(|body| toml::from_str::<toml::Value>(&body).ok())
        .and_then(|v| v.get("doctor")?.get(key)?.as_integer())
        .and_then(|n| u64::try_from(n).ok())
        .unwrap_or(default)
}

/// `argo doctor wal-stats` — checkpoint the WAL (PASSIVE) and report
/// the active WAL pragmas plus the `*.db-wal` file size. The store is
/// opened with the profile from `[store.wal_tuning].profile` so the
/// readout reflects what the runtime would actually apply.
pub(super) async fn wal_stats(json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
