use super::{
    doctor_audit_chain, doctor_channels, doctor_h_stack, doctor_network, doctor_workers,
    print_build_info, summarize,
};
use crate::home::resolve_home;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(json: bool, format: &str, skip_network: bool) -> ArgoResult<()> {
    let payload = collect_with_skip(skip_network).await?;
    let summary = summarize(&payload);
    let pick = if json { "json" } else { format };
    match pick {
        "summary" | "summary-json" => {
            let line = if pick == "summary-json" {
                serde_json::to_string(&summary.json())
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
            } else {
                summary.line()
            };
            println!("{line}");
            std::process::exit(summary.state.exit_code());
        }
        "json" => {
            println!(
                "{}",
                serde_json::to_string(&payload)
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
            );
            std::process::exit(summary.state.exit_code());
        }
        "markdown" => {
            crate::handlers::doctor_md::print_markdown(&payload);
            std::process::exit(summary.state.exit_code());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', 'markdown', or 'summary')"
            )));
        }
    }
    print_build_info(&payload["build_info"]);
    print_workspace_text(&payload);
    crate::handlers::doctor_storage::print_db(&payload["db"]);
    crate::handlers::doctor_storage::print_audit(&payload["audit"]);
    crate::handlers::doctor_env::print_cron(&payload["cron"]);
    crate::handlers::doctor_env::print_creds(&payload["creds"]);
    crate::handlers::doctor_gateway::print_gateway(&payload["gateway"]);
    crate::handlers::doctor_gateway::print_health(&payload["gateway_health"]);
    crate::handlers::doctor_gateway::print_routes(&payload["routes"]);
    doctor_network::print_text(&payload["network_checks"]);
    doctor_h_stack::print_text(&payload["auto_approval"]);
    doctor_channels::print_text(&payload["channels"]);
    doctor_audit_chain::print_text(&payload["audit_chain"]);
    crate::handlers::doctor_storage::print_capacity(&payload["storage"]);
    doctor_workers::print_text(&payload["workers"]);
    std::process::exit(summary.state.exit_code());
}

fn print_workspace_text(payload: &serde_json::Value) {
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
}

/// Gather every doctor check into a single JSON document. Shared
/// between the CLI handler and the gateway's `GET /doctor`
/// endpoint.
pub(crate) async fn collect() -> ArgoResult<serde_json::Value> {
    collect_with_skip(false).await
}

pub(crate) async fn collect_with_skip(skip_network: bool) -> ArgoResult<serde_json::Value> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
    let db = crate::handlers::doctor_storage::db_status(&home).await;
    let audit = crate::handlers::doctor_storage::audit_status(&home).await;
    let cron = crate::handlers::doctor_env::cron_status(&cwd);
    let creds = crate::handlers::doctor_env::creds_status();
    let gateway = crate::handlers::doctor_gateway::gateway_status(&home);
    let gateway_health = crate::handlers::doctor_gateway::gateway_health_probe(&gateway).await;
    let routes = crate::handlers::doctor_gateway::routes_count();
    let auto_approval = doctor_h_stack::collect(&cwd, &home).await;
    let skip_online = skip_network || doctor_network::skip_from_env();
    let channels = if skip_online {
        doctor_channels::collect_skipped()
    } else {
        doctor_channels::collect().await
    };
    let network_checks = doctor_network::collect(&cwd, skip_online).await;
    let audit_chain = if skip_online {
        doctor_audit_chain::skipped()
    } else {
        doctor_audit_chain::collect(&home).await
    };
    let storage = crate::handlers::doctor_storage::capacity_status(&home, &cwd).await;
    let workers = doctor_workers::collect(&home, &cwd).await;
    let build = crate::build_info::BuildInfo::current();
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let build_info_json = serde_json::json!({
        "version": build.version,
        "git_sha": build.git_sha,
        "git_date": build.git_date,
        "target": build.target,
        "rustc": build.rustc,
        "profile": build.profile,
        "features": build.features,
        "binary_path": binary_path,
    });
    Ok(serde_json::json!({
        "build_info": build_info_json,
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
        "network_checks": network_checks,
        "audit_chain": audit_chain,
        "storage": storage,
        "workers": workers,
    }))
}
