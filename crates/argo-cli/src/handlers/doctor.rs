//! `argo doctor` — config / workspace sanity report. Default
//! output is human-readable text; `--json` emits a single JSON
//! object that monitoring pipelines can ingest.
use crate::cli::DoctorCmd;
use crate::home::resolve_home;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

#[path = "doctor_channels.rs"]
mod doctor_channels;
#[path = "doctor_h_stack.rs"]
mod doctor_h_stack;

/// Single entry point for every `argo doctor [...]` invocation —
/// keeps the per-subcommand wiring out of `main.rs` so the dispatch
/// can grow without ballooning the binary entry file.
pub async fn dispatch(cmd: Option<DoctorCmd>, json: bool, format: String) -> ArgoResult<()> {
    match cmd {
        None => run(json, &format).await,
        Some(DoctorCmd::Fix { auto, issue, json: fix_json }) => {
            super::doctor_fix::run(auto, issue.as_deref(), json || fix_json).await
        }
        Some(DoctorCmd::Watch { interval, clear }) => {
            super::doctor_watch::run(&interval, clear).await
        }
        Some(DoctorCmd::WalStats { json: ws_json }) => {
            super::doctor_storage::wal_stats(json || ws_json).await
        }
        Some(DoctorCmd::Approvals {
            since,
            json: cmd_json,
        }) => super::doctor_approvals::run(&since, json || cmd_json).await,
        Some(DoctorCmd::Throttles { json: t_json }) => {
            super::doctor_throttles::run(json || t_json).await
        }
    }
}

pub async fn run(json: bool, format: &str) -> ArgoResult<()> {
    let payload = collect().await?;
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            println!(
                "{}",
                serde_json::to_string(&payload)
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
            );
            return Ok(());
        }
        "markdown" => {
            super::doctor_md::print_markdown(&payload);
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
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
    super::doctor_storage::print_db(&payload["db"]);
    super::doctor_storage::print_audit(&payload["audit"]);
    super::doctor_env::print_cron(&payload["cron"]);
    super::doctor_env::print_creds(&payload["creds"]);
    super::doctor_gateway::print_gateway(&payload["gateway"]);
    super::doctor_gateway::print_health(&payload["gateway_health"]);
    super::doctor_gateway::print_routes(&payload["routes"]);
    doctor_h_stack::print_text(&payload["auto_approval"]);
    print_channels(&payload["channels"]);
    Ok(())
}

fn print_channels(v: &serde_json::Value) {
    println!("  Channels:");
    for key in ["email", "slack", "sms", "push"] {
        let item = &v[key];
        let status = item["status"].as_str().unwrap_or("ERROR");
        let detail = item["detail"].as_str().unwrap_or("unavailable");
        println!("    {key}: {status} - {detail}");
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
    let db = super::doctor_storage::db_status(&home).await;
    let audit = super::doctor_storage::audit_status(&home).await;
    let cron = super::doctor_env::cron_status(&cwd);
    let creds = super::doctor_env::creds_status();
    let gateway = super::doctor_gateway::gateway_status(&home);
    let gateway_health = super::doctor_gateway::gateway_health_probe(&gateway).await;
    let routes = super::doctor_gateway::routes_count();
    let auto_approval = doctor_h_stack::collect(&cwd, &home).await;
    let channels = doctor_channels::collect().await;
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
        "gateway_health": gateway_health,
        "routes": routes,
        "auto_approval": auto_approval,
        "channels": channels,
    }))
}
