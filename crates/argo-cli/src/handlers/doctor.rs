//! `argo doctor` — config / workspace sanity report. Default
//! output is human-readable text; `--json` emits a single JSON
//! object that monitoring pipelines can ingest.
use crate::cli::DoctorCmd;
use crate::home::resolve_home;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};

#[path = "doctor_audit_chain.rs"]
mod doctor_audit_chain;
#[path = "doctor_channels.rs"]
mod doctor_channels;
#[path = "doctor_h_stack.rs"]
mod doctor_h_stack;
#[path = "doctor_network.rs"]
mod doctor_network;
#[path = "doctor_workers.rs"]
mod doctor_workers;

/// Single entry point for every `argo doctor [...]` invocation —
/// keeps the per-subcommand wiring out of `main.rs` so the dispatch
/// can grow without ballooning the binary entry file.
pub async fn dispatch(
    cmd: Option<DoctorCmd>,
    json: bool,
    format: String,
    skip_network: bool,
) -> ArgoResult<()> {
    match cmd {
        None => run(json, &format, skip_network).await,
        Some(DoctorCmd::Fix {
            auto,
            issue,
            json: fix_json,
        }) => super::doctor_fix::run(auto, issue.as_deref(), json || fix_json).await,
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
        Some(DoctorCmd::InstallHooks {
            force,
            json: ih_json,
        }) => super::doctor_install_hooks::run(force, json || ih_json),
        Some(DoctorCmd::Throttles { json: t_json }) => {
            super::doctor_throttles::run(json || t_json).await
        }
    }
}

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
            super::doctor_md::print_markdown(&payload);
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
    doctor_network::print_text(&payload["network_checks"]);
    doctor_h_stack::print_text(&payload["auto_approval"]);
    doctor_channels::print_text(&payload["channels"]);
    doctor_audit_chain::print_text(&payload["audit_chain"]);
    super::doctor_storage::print_capacity(&payload["storage"]);
    doctor_workers::print_text(&payload["workers"]);
    std::process::exit(summary.state.exit_code());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SummaryState {
    Ok,
    Degraded,
    Fail,
}

impl SummaryState {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Degraded => "DEGRADED",
            Self::Fail => "FAIL",
        }
    }
    fn exit_code(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::Degraded => 1,
            Self::Fail => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorSummary {
    pub(crate) state: SummaryState,
    pub(crate) passed: usize,
    pub(crate) warned: usize,
    pub(crate) failed: usize,
    issue: Option<&'static str>,
}
impl DoctorSummary {
    pub(crate) fn line(&self) -> String {
        let total = self.passed + self.warned + self.failed;
        let base = format!("{} {}/{} checks", self.state.label(), self.passed, total);
        match (self.issue, self.warned, self.failed) {
            (Some(issue), warned, 0) if warned > 0 => format!("{base} ({warned} warn: {issue})"),
            (Some(issue), _, failed) if failed > 0 => format!("{base} ({failed} fail: {issue})"),
            _ => base,
        }
    }

    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({"state": self.state.label(), "passed": self.passed, "warned": self.warned, "failed": self.failed})
    }
}

pub(crate) fn summarize(v: &serde_json::Value) -> DoctorSummary {
    let checks = [
        (
            "argo_toml",
            yes(
                v["argo_toml"]["ok"].as_bool().unwrap_or(false),
                SummaryState::Fail,
            ),
        ),
        (
            "argo_home",
            yes(
                v["argo_home"]["ok"].as_bool().unwrap_or(false),
                SummaryState::Degraded,
            ),
        ),
        (
            "bundle_dirs",
            yes(
                v["bundle_dirs_missing"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                SummaryState::Degraded,
            ),
        ),
        (
            "memory_files",
            yes(
                v["memory_files_missing"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                SummaryState::Degraded,
            ),
        ),
        ("db", db_state(&v["db"])),
        ("audit", audit_state(&v["audit"])),
        ("cron", SummaryState::Ok),
        (
            "creds",
            yes(
                v["creds"]["set"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                SummaryState::Degraded,
            ),
        ),
        (
            "gateway",
            yes(
                v["gateway"]["running"].as_bool().unwrap_or(false),
                SummaryState::Degraded,
            ),
        ),
        ("gateway_health", gateway_health_state(v)),
        (
            "routes",
            yes(
                v["routes"]["total"].as_u64().unwrap_or(0) > 0,
                SummaryState::Fail,
            ),
        ),
        ("auto_approval", status_group_state(&v["auto_approval"])),
        ("channels", status_group_state(&v["channels"])),
        ("network", network_state(&v["network_checks"])),
        ("audit_chain", status_item_state(&v["audit_chain"])),
        ("storage", status_item_state(&v["storage"])),
        ("workers", status_item_state(&v["workers"])),
    ];
    let mut out = DoctorSummary {
        state: SummaryState::Ok,
        passed: 0,
        warned: 0,
        failed: 0,
        issue: None,
    };
    for (name, state) in checks {
        match state {
            SummaryState::Ok => out.passed += 1,
            SummaryState::Degraded => out.warned += 1,
            SummaryState::Fail => out.failed += 1,
        }
        if state > out.state {
            out.state = state;
            out.issue = Some(name);
        }
    }
    out
}
fn yes(ok: bool, otherwise: SummaryState) -> SummaryState {
    if ok {
        SummaryState::Ok
    } else {
        otherwise
    }
}
fn db_state(v: &serde_json::Value) -> SummaryState {
    if v["open_error"].is_string() || v["migrate_error"].is_string() {
        SummaryState::Fail
    } else if !v["present"].as_bool().unwrap_or(false)
        || !v["workspace_registered"].as_bool().unwrap_or(false)
    {
        SummaryState::Degraded
    } else {
        SummaryState::Ok
    }
}
fn audit_state(v: &serde_json::Value) -> SummaryState {
    if !v["available"].as_bool().unwrap_or(false) {
        SummaryState::Degraded
    } else {
        yes(v["ok"].as_bool().unwrap_or(false), SummaryState::Fail)
    }
}
fn gateway_health_state(v: &serde_json::Value) -> SummaryState {
    if !v["gateway"]["running"].as_bool().unwrap_or(false) {
        return SummaryState::Degraded;
    }
    yes(
        v["gateway_health"]["reachable"].as_bool().unwrap_or(false)
            && v["gateway_health"]["status_code"].as_u64().unwrap_or(0) < 500,
        SummaryState::Fail,
    )
}
fn status_group_state(v: &serde_json::Value) -> SummaryState {
    let Some(obj) = v.as_object() else {
        return SummaryState::Degraded;
    };
    let mut state = SummaryState::Ok;
    for status in obj
        .values()
        .filter_map(|item| item.get("status").and_then(|s| s.as_str()))
    {
        match status {
            "ERROR" | "FAIL" | "BROKEN" => return SummaryState::Fail,
            "WARN" | "MISSING" => state = SummaryState::Degraded,
            _ => {}
        }
    }
    state
}
fn status_item_state(v: &serde_json::Value) -> SummaryState {
    match v["status"].as_str().unwrap_or("WARN") {
        "OK" | "SKIP" => SummaryState::Ok,
        "FAIL" | "ERROR" | "BROKEN" => SummaryState::Fail,
        _ => SummaryState::Degraded,
    }
}
fn network_state(v: &serde_json::Value) -> SummaryState {
    if v["skipped"].as_bool().unwrap_or(false) {
        return SummaryState::Ok;
    }
    let failures = ["dns", "tls", "auth"]
        .into_iter()
        .filter_map(|key| v[key].as_array())
        .flat_map(|items| items.iter())
        .filter(|item| item["status"].as_str() == Some("FAIL"))
        .count();
    match failures {
        0 => SummaryState::Ok,
        1 => SummaryState::Degraded,
        _ => SummaryState::Fail,
    }
}

/// Render the "Build Info" block printed above the existing health
/// checks (`mission-cli-version-doctor-extras`). Silently no-ops if
/// the payload lacks `build_info` (e.g. older callers that build the
/// JSON directly) so the rest of the doctor output still prints.
fn print_build_info(v: &serde_json::Value) {
    if !v.is_object() {
        return;
    }
    let version = v["version"].as_str().unwrap_or("?");
    let git_sha = v["git_sha"].as_str().unwrap_or("?");
    let git_date = v["git_date"].as_str().unwrap_or("?");
    let target = v["target"].as_str().unwrap_or("?");
    let rustc = v["rustc"].as_str().unwrap_or("?");
    let profile = v["profile"].as_str().unwrap_or("?");
    let binary_path = v["binary_path"].as_str().unwrap_or("?");
    let features: Vec<&str> = v["features"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let features_str = if features.is_empty() {
        "(none)".to_string()
    } else {
        features.join(",")
    };
    println!("argo doctor — Build Info");
    println!("  version:  {version} ({git_sha} {git_date})");
    println!("  target:   {target}");
    println!("  rustc:    {rustc}");
    println!("  profile:  {profile}");
    println!("  binary:   {binary_path}");
    println!("  features: {features_str}");
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
    let db = super::doctor_storage::db_status(&home).await;
    let audit = super::doctor_storage::audit_status(&home).await;
    let cron = super::doctor_env::cron_status(&cwd);
    let creds = super::doctor_env::creds_status();
    let gateway = super::doctor_gateway::gateway_status(&home);
    let gateway_health = super::doctor_gateway::gateway_health_probe(&gateway).await;
    let routes = super::doctor_gateway::routes_count();
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
    let storage = super::doctor_storage::capacity_status(&home, &cwd).await;
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

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod doctor_tests;
