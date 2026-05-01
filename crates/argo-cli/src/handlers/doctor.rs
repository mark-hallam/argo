//! `argo doctor` — config / workspace sanity report. Default
//! output is human-readable text; `--json` emits a single JSON
//! object that monitoring pipelines can ingest.
use crate::home::{db_path, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(json: bool) -> ArgoResult<()> {
    let payload = collect().await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&payload)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
        return Ok(());
    }
    let cwd = payload["cwd"].as_str().unwrap_or("?").to_string();
    let toml_path = payload["argo_toml"]["path"].as_str().unwrap_or("?");
    let toml_ok = payload["argo_toml"]["ok"].as_bool().unwrap_or(false);
    let home_path = payload["argo_home"]["path"].as_str().unwrap_or("?");
    let home_ok = payload["argo_home"]["ok"].as_bool().unwrap_or(false);
    let profile = payload["profile"].as_str().unwrap_or("?");
    let tenant_id = payload["tenant"]["id"].as_str().unwrap_or("?");
    let tenant_mode = payload["tenant"]["mode"].as_str().unwrap_or("?");
    let bundle_missing: Vec<&str> = payload["bundle_dirs_missing"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let memfiles_missing: Vec<&str> = payload["memory_files_missing"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let toml_status = if toml_ok { "OK" } else { "MISSING" };
    let home_status = if home_ok {
        "OK"
    } else {
        "MISSING (will be created on first run)"
    };
    println!("argo doctor — workspace check");
    println!("  cwd:        {cwd}");
    println!("  argo.toml:  {toml_path} ({toml_status})");
    println!("  ARGO_HOME:  {home_path} ({home_status})");
    println!("  profile:    {profile}");
    println!("  tenant:     {tenant_id} (mode={tenant_mode})");
    if bundle_missing.is_empty() {
        println!("  bundle dirs: all present");
    } else {
        println!(
            "  bundle dirs missing: {} (run `argo workspace init`)",
            bundle_missing.join(", ")
        );
    }
    if memfiles_missing.is_empty() {
        println!("  memory files: all present");
    } else {
        println!(
            "  memory files missing: {} (run `argo workspace init`)",
            memfiles_missing.join(", ")
        );
    }
    print_db(&payload["db"]);
    print_audit(&payload["audit"]);
    print_cron(&payload["cron"]);
    print_creds(&payload["creds"]);
    print_gateway(&payload["gateway"]);
    print_routes(&payload["routes"]);
    Ok(())
}

fn print_routes(v: &serde_json::Value) {
    let total = v["total"].as_u64().unwrap_or(0);
    let get = v["get"].as_u64().unwrap_or(0);
    let post = v["post"].as_u64().unwrap_or(0);
    println!("  routes:     {total} endpoints ({get} GET, {post} POST)");
}

fn print_audit(v: &serde_json::Value) {
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

fn print_gateway(v: &serde_json::Value) {
    let running = v["running"].as_bool().unwrap_or(false);
    if running {
        let pid = v["pid"].as_u64().unwrap_or(0);
        println!("  gateway:    RUNNING (pid={pid})");
    } else {
        println!("  gateway:    not running");
    }
}

/// Gather every doctor check into a single JSON document. Shared
/// between the CLI handler and the gateway's `GET /doctor`
/// endpoint.
pub(crate) async fn collect() -> ArgoResult<serde_json::Value> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let toml = cwd.join("argo.toml");
    let toml_ok = toml.exists();
    let cfg = ArgoConfig::load(&toml).unwrap_or_default();
    let home = resolve_home(&cwd).unwrap_or_else(|_| cfg.core.resolved_home());
    let home_ok = home.exists();
    let bundle_dirs = ["skills", "personas", "cron", "rubrics"];
    let bundle_missing: Vec<&str> = bundle_dirs
        .iter()
        .copied()
        .filter(|d| !cwd.join(d).exists())
        .collect();
    let memfiles = ["MEMORY.md", "USER.md", "SOUL.md", "AGENTS.md"];
    let memfiles_missing: Vec<&str> = memfiles
        .iter()
        .copied()
        .filter(|f| !cwd.join(f).exists())
        .collect();
    let db = db_status(&home).await;
    let audit = audit_status(&home).await;
    let cron = cron_status(&cwd);
    let creds = creds_status();
    let gateway = gateway_status(&home);
    let routes = routes_count();
    Ok(serde_json::json!({
        "cwd": cwd.display().to_string(),
        "argo_toml": {"path": toml.display().to_string(), "ok": toml_ok},
        "argo_home": {"path": home.display().to_string(), "ok": home_ok},
        "profile": cfg.core.profile,
        "tenant": {"id": cfg.tenant.default_id, "mode": cfg.tenant.mode},
        "bundle_dirs_missing": bundle_missing,
        "memory_files_missing": memfiles_missing,
        "db": db,
        "audit": audit,
        "cron": cron,
        "creds": creds,
        "gateway": gateway,
        "routes": routes,
    }))
}

/// Reads the in-binary route catalog and reports counts by
/// verb. Pure data — no daemon required.
fn routes_count() -> serde_json::Value {
    let (_status, body) =
        crate::handlers::gateway::routes_catalog::routes_response(None);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Array(Vec::new()));
    let arr = parsed.as_array().cloned().unwrap_or_default();
    let total = arr.len();
    let get = arr
        .iter()
        .filter(|r| r["method"].as_str() == Some("GET"))
        .count();
    let post = arr
        .iter()
        .filter(|r| r["method"].as_str() == Some("POST"))
        .count();
    serde_json::json!({
        "total": total,
        "get": get,
        "post": post,
    })
}

/// Report whether the gateway daemon is running. Reads
/// `<ARGO_HOME>/gateway.pid`; if the file exists and the PID
/// is alive, returns `{running: true, pid: N}`. Otherwise
/// `{running: false}` (the file may be present but stale —
/// the live-PID check shells out to `kill -0` so we don't
/// pull in libc just for this one call).
fn gateway_status(home: &std::path::Path) -> serde_json::Value {
    let pid_path = home.join("gateway.pid");
    let Ok(s) = std::fs::read_to_string(&pid_path) else {
        return serde_json::json!({"running": false});
    };
    let Ok(pid) = s.trim().parse::<u32>() else {
        return serde_json::json!({"running": false});
    };
    // `kill -0 <pid>` exit 0 → live, !=0 → gone or not ours.
    // Safe, side-effect-free probe; works on every Unix.
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    serde_json::json!({
        "running": alive,
        "pid": pid,
    })
}

/// Report `{ok, rows, breaks}` for the global audit chain so
/// `argo doctor --json` can be wired into CI gates that should
/// fail when the chain is broken. Errors silently degrade to
/// `{"available": false}`.
async fn audit_status(home: &std::path::Path) -> serde_json::Value {
    let db = crate::home::db_path(home);
    let store = match argo_store::SessionStore::open(&db).await {
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

fn cron_status(cwd: &std::path::Path) -> serde_json::Value {
    let lock = cwd.join("cron").join(".tick.lock");
    if !lock.exists() {
        return serde_json::json!({"daemon_started": false});
    }
    let age_s = std::fs::metadata(&lock)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs());
    serde_json::json!({
        "daemon_started": true,
        "lock_path": lock.display().to_string(),
        "lock_age_seconds": age_s,
    })
}

fn creds_status() -> serde_json::Value {
    let known = [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "ELEVENLABS_API_KEY",
    ];
    let mut found: Vec<&str> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();
    for k in known {
        if std::env::var_os(k).is_some_and(|v| !v.is_empty()) {
            found.push(k);
        } else {
            missing.push(k);
        }
    }
    serde_json::json!({"set": found, "unset": missing})
}

async fn db_status(home: &std::path::Path) -> serde_json::Value {
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

fn print_db(v: &serde_json::Value) {
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

fn print_cron(v: &serde_json::Value) {
    if !v.get("daemon_started").and_then(|b| b.as_bool()).unwrap_or(false) {
        println!("  cron:       no lock file (daemon not started)");
        return;
    }
    let path = v.get("lock_path").and_then(|s| s.as_str()).unwrap_or("?");
    let age = v.get("lock_age_seconds").and_then(|n| n.as_u64());
    match age {
        Some(s) => println!("  cron:       lock present at {path} (mtime {s}s ago)"),
        None => println!("  cron:       lock present at {path}"),
    }
}

fn print_creds(v: &serde_json::Value) {
    let set: Vec<String> = v
        .get("set")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let unset: Vec<String> = v
        .get("unset")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if set.is_empty() {
        println!(
            "  creds:      none of {} are set",
            unset.join(", ")
        );
    } else {
        println!("  creds:      set: {}", set.join(", "));
        if !unset.is_empty() {
            println!("              unset: {}", unset.join(", "));
        }
    }
}
