//! `argo gateway` — start / stop / status. PID file at
//! `<ARGO_HOME>/gateway.pid` mediates the start↔stop handshake.
//!
//! Sub-modules:
//!   - `lifecycle`:            PidGuard + signal helpers (SIGTERM/SIGINT).
//!   - `boot`:                 start/stop command bodies + supporting helpers.
//!   - `start_banner`:         the one-time route summary printed on `start`.
//!   - `start_banner_config`:  layered config loader for the banner.
//!   - `print_routes`:         `argo gateway routes` CLI subcommand printer.
//!   - `routes`:               HTTP route dispatch + per-endpoint handlers.
mod approval_expiry_sweeper;
mod service_token_binding_sweeper;
mod approval_reminder;
mod auth_magic_link;
mod billing;
mod boot;
mod boot_helpers;
mod boot_router;
mod boot_spawns;
mod boot_telemetry;
mod boot_telemetry_admin;
mod bootstrap_token;
mod cors;
mod dashboard;
mod dashboard_fleet;
mod email_outbound;
mod hipaa_startup_check;
mod ip_allowlist;
mod jwt;
mod lifecycle;
mod maintenance_middleware;
mod marketplace;
mod marketplace_render;
mod middleware_csrf;
mod middleware_origin_check;
mod middleware_session_verify;
mod metrics;
mod metrics_actions;
mod metrics_json;
mod metrics_text;
mod onboarding_provision;
mod routes_onboarding;
mod routes_onboarding_helpers;
mod peer;
mod print_routes;
mod proxy_guard;
mod quota_middleware;
mod rate_limit;
mod rate_limit_ip;
mod rate_limit_tenant;
mod region_gate;
mod request_audit;
mod role_gate;
mod routes;
mod session_jwt;
mod routes_approval_bundles;
mod routes_approvals_json;
mod routes_audit;
mod routes_audit_detail;
mod routes_audit_export;
mod routes_audit_record;
mod routes_audit_replay;
mod routes_audit_detail_refs;
mod routes_audit_detail_render;
mod routes_audit_feed;
mod routes_audit_feed_indexed;
mod routes_audit_feed_scope;
mod routes_audit_rollups;
mod routes_audit_rollups_tools;
mod routes_audit_screenshot;
mod routes_audit_session;
mod routes_audit_stream;
mod routes_auth;
mod routes_auth_csrf;
mod routes_auth_logout;
mod routes_auth_principal;
mod routes_auth_production_mode;
pub(crate) mod routes_catalog;
mod routes_channels;
mod routes_channels_discord;
mod routes_channels_email;
mod routes_channels_imessage;
mod routes_channels_push;
mod routes_channels_slack;
mod routes_channels_voice;
mod routes_channels_webinbox;
mod routes_channels_whatsapp;
mod routes_cohorts;
mod routes_costs_by_day;
mod routes_counts;
mod routes_cron;
mod routes_dashboard;
mod routes_dispatch_costs;
mod routes_dispatch_delete;
mod routes_dispatch_forge;
mod routes_dispatch_get;
mod routes_dispatch_get_admin;
mod routes_dispatch_get_api;
mod usage_endpoint;
mod routes_dispatch_get_catalog;
mod routes_dispatch_get_forge_audit;
mod routes_dispatch_patch;
mod routes_dispatch_post;
mod routes_dispatch_post_approvals;
mod routes_dispatch_post_audit;
mod routes_dispatch_post_duties;
mod boot_serve;
mod routes_dispatch_post_memory;
mod routes_dispatch_post_paths;
mod routes_dispatch_post_retros;
mod routes_dispatch_post_workers;
mod routes_dispatch_rollups;
mod routes_duties;
mod routes_elevated_view;
mod routes_evals;
mod routes_forge;
mod routes_forge_promote;
mod routes_forge_rollups;
mod routes_forge_runs_list;
mod routes_genesis;
mod routes_ghostwriter;
mod routes_health;
mod routes_lists;
mod routes_lists_memory;
mod routes_me_export;
mod routes_messages_rollups;
mod routes_messages_rollups_recent;
mod routes_meta;
mod routes_meta_error_reporting;
mod routes_meta_tools;
mod routes_metrics_cardinality;
mod routes_msgs;
mod routes_msgs_pick;
mod routes_openmetrics;
mod routes_orchestration;
mod routes_orchestrator;
mod routes_orchestrator_handoffs;
mod routes_orchestrator_sessions;
mod routes_orchestrator_slas;
mod routes_peer;
mod routes_post;
mod routes_post_admin;
mod routes_post_costs;
mod routes_post_forge;
mod routes_post_judge;
mod routes_post_messages;
mod routes_post_runs;
mod routes_post_sessions;
mod routes_post_writes;
mod routes_post_writes_db;
mod routes_post_writes_skills;
mod routes_providers;
mod routes_query;
mod routes_request_audit;
mod routes_retrain;
mod routes_retros;
mod routes_scim_users;
mod routes_scim_users_envelope;
mod routes_search;
mod routes_session;
mod routes_session_approval;
mod routes_session_audit_rollups;
mod routes_session_clone;
mod routes_session_get;
mod routes_session_get_export;
mod routes_session_skills;
mod routes_session_skills_lifecycle;
mod routes_session_stats;
mod routes_sessions_rollups;
mod routes_skill_ir;
mod routes_skill_refine;
mod routes_skills;
mod routes_skills_quality;
mod routes_skills_show;
mod routes_support_tickets;
mod routes_notifications;
mod routes_team_list;
mod routes_top;
mod routes_top_sessions;
mod routes_trust_streaks;
mod routes_verticals;
mod routes_verticals_doctor;
mod routes_verticals_lint;
mod routes_verticals_run;
mod routes_voice;
mod routes_voice_auth;
mod routes_wizard;
mod routes_worker_activity;
mod routes_worker_detail;
mod routes_worker_freeze;
mod routes_worker_memory;
mod routes_worker_overview;
mod routes_worker_policies;
mod routes_worker_settings;
mod routes_worker_skills;
mod routes_worker_skills_helpers;
mod routes_worker_trust_grade;
mod routes_workers_hire;
mod routes_workflow_runs;
mod routes_knowledge;
mod routes_knowledge_handlers;
mod routes_workflows;
mod routes_workflows_handlers;
mod routes_workspace;
mod routes_workspace_members;
mod routes_workspace_members_accept;
mod routes_workspace_members_invite;
mod runs_streaming;
mod scim_auth;
mod security_headers;
mod shutdown;
mod signup_protection;
pub(super) mod sso_dispatch;
mod sso_oidc;
mod sso_oidc_helpers;
mod sso_saml;
mod sso_session;
mod start_banner;
mod start_banner_config;
mod startup_check;
mod streaming;
mod sweepers;
mod tier_capability_filter;
mod ws_inbox_events;
mod ws_skill_trace;
mod ws_upgrade;

use crate::cli::cli_gateway_cmds::GatewayBootstrapCmd;
use crate::cli::GatewayCmd;
use argo_config::ArgoConfig;
use argo_types::{ArgoError, ArgoResult};
use lifecycle::pid_is_alive;

pub async fn run(cmd: GatewayCmd) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let mut cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    if let Ok(env_cfg) = ArgoConfig::from_env() {
        cfg.merge(env_cfg);
    }
    match cmd {
        GatewayCmd::Status { json } => status(&cfg, json),
        GatewayCmd::Start {
            bind,
            port,
            max_requests,
            quiet,
            profile_startup,
            skip_startup_check,
            bind_non_loopback,
            accept_insecure_bind_warning,
            insecure_bind,
        } => {
            let bind_non_loopback = resolve_non_loopback_override(
                bind_non_loopback,
                accept_insecure_bind_warning,
                insecure_bind,
            )?;
            // Record gateway-start instant for the `/health` +
            // `/readyz` `uptime_seconds` field. Idempotent across
            // re-entry — see `routes_health::mark_start`.
            routes_health::mark_start();
            // Initialise the per-route, per-tenant token-bucket limiter
            // and spawn its eviction sweep before the listener accepts.
            // Idempotent — only the first caller actually wires state.
            let _ = rate_limit::init_with_config(&cfg.gateway.rate_limit.route);
            // mission-gateway-cors-preflight: install the CORS allowlist
            // registry. Pre-builds (status × origin) response strings so
            // dispatch lookups are zero-allocation. Idempotent.
            cors::install(&cfg.gateway.cors);
            // mission-per-tenant-rate-limiting: layer the per-tenant
            // limiter on top of the `[gateway.rate_limit.tenant]`
            // config block and hydrate per-tenant overrides from the
            // workspace `tenant_rate_limit` table. Best-effort: a
            // missing store falls through to the limiter's default.
            let tenant_limiter =
                rate_limit_tenant::init_with_config(&cfg.gateway.rate_limit.tenant);
            if let Ok(overrides) = load_tenant_rate_limit_overrides().await {
                tenant_limiter.install_overrides(overrides);
            }
            boot::start(
                &bind,
                port,
                max_requests,
                quiet,
                profile_startup,
                skip_startup_check,
                bind_non_loopback,
            )
            .await
        }
        GatewayCmd::Stop => boot::stop().await,
        GatewayCmd::Bootstrap { cmd } => match cmd {
            GatewayBootstrapCmd::Rotate => bootstrap_token::rotate_command().await,
        },
        GatewayCmd::Routes {
            json,
            format,
            method,
            grep,
            limit,
        } => print_routes::print_routes(json, &format, method.as_deref(), grep.as_deref(), limit),
        GatewayCmd::Audit { cmd } => super::gateway_audit::run(cmd).await,
        GatewayCmd::Token { cmd } => super::gateway_token_bind::run(cmd).await,
        GatewayCmd::Probe {
            kind,
            host,
            port,
            json,
        } => super::gateway_probe::run(kind, host, port, json).await,
    }
}

fn resolve_non_loopback_override(
    bind_non_loopback: bool,
    accept_warning: bool,
    deprecated_insecure_bind: bool,
) -> ArgoResult<bool> {
    if deprecated_insecure_bind {
        return Err(ArgoError::Schema(
            "--insecure-bind is deprecated; use --bind-non-loopback and pass \
             --accept-insecure-bind-warning to confirm"
                .into(),
        ));
    }
    if bind_non_loopback && !accept_warning {
        return Err(ArgoError::Schema(
            "--bind-non-loopback requires --accept-insecure-bind-warning".into(),
        ));
    }
    if accept_warning && !bind_non_loopback {
        return Err(ArgoError::Schema(
            "--accept-insecure-bind-warning requires --bind-non-loopback".into(),
        ));
    }
    Ok(bind_non_loopback)
}

fn status(cfg: &ArgoConfig, json: bool) -> ArgoResult<()> {
    // Best-effort: read the PID file so the JSON form can report
    // a real running state, not just the config bits.
    let pid_alive = boot::pid_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(pid_is_alive)
        .unwrap_or(false);
    if json {
        let payload = serde_json::json!({
            "enabled": cfg.gateway.enabled,
            "adapters": cfg.gateway.adapters,
            "running": pid_alive,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return Ok(());
    }
    println!("gateway:");
    println!(
        "  enabled:   {} (set [gateway].enabled in argo.toml)",
        cfg.gateway.enabled
    );
    println!("  running:   {pid_alive}");
    if cfg.gateway.adapters.is_empty() {
        println!("  adapters:  (none configured)");
    } else {
        println!("  adapters:");
        for a in &cfg.gateway.adapters {
            println!("    - {a}");
        }
    }
    Ok(())
}

/// Best-effort load of the per-tenant rate-limit override map from
/// the workspace store. Returns the `(tenant_id_string, rpm)` pairs
/// suitable for `TenantRateLimiter::install_overrides`. Errors fall
/// through to an empty Vec so a missing/uninitialised store leaves
/// the limiter at its config-driven default. Lives here rather than
/// in `boot.rs` so the gateway-mod boot path stays inside the
/// mission brief's may-touch list.
async fn load_tenant_rate_limit_overrides() -> ArgoResult<Vec<(String, u32)>> {
    use crate::home::{db_path, ensure_dir, resolve_home};
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = argo_store::SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let rows = argo_store::tenant_rate_limit::TenantRateLimitStore::new(store.pool())
        .list()
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.tenant_id.to_string(), r.requests_per_minute))
        .collect())
}

pub(super) fn store_unavailable() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"session store unavailable\"}".to_string(),
    )
}

pub(super) fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

/// Count the `ToolCall` audit rows scoped to one session — cheaper
/// than walking message content_json. Used by `GET /sessions/<id>`.
pub(super) async fn session_tool_call_count(
    store: &argo_store::SessionStore,
    sid: argo_types::SessionId,
) -> argo_types::ArgoResult<u64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE session_id = ? AND kind = 'ToolCall'",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("audit count: {e}")))?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// Sum of spend ledger micros for one session.
pub(super) async fn session_spend_micros(
    store: &argo_store::SessionStore,
    sid: argo_types::SessionId,
) -> argo_types::ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE session_id = ?",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("spend sum: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

/// `(session_id_blob → SUM(amount_usd_micros))` map across every
/// session that has a spend row. One indexed query — used by
/// `GET /sessions` to enrich the list response.
pub(super) async fn spend_per_session(
    store: &argo_store::SessionStore,
) -> argo_types::ArgoResult<std::collections::HashMap<Vec<u8>, u64>> {
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         GROUP BY session_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("spend_per_session: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(sid, total)| (sid, u64::try_from(total).unwrap_or(0)))
        .collect())
}

#[cfg(test)]
mod bind_flag_tests {
    use super::resolve_non_loopback_override;

    #[test]
    fn two_flag_override_required() {
        assert!(resolve_non_loopback_override(false, false, false).is_ok_and(|v| !v));
        assert!(resolve_non_loopback_override(true, true, false).is_ok_and(|v| v));
        assert!(resolve_non_loopback_override(true, false, false).is_err());
        assert!(resolve_non_loopback_override(false, true, false).is_err());
    }

    #[test]
    fn deprecated_insecure_bind_is_rejected() {
        let msg = resolve_non_loopback_override(false, false, true)
            .expect_err("deprecated flag must fail")
            .to_string();
        assert!(msg.contains("--insecure-bind is deprecated"), "{msg}");
        assert!(msg.contains("--bind-non-loopback"), "{msg}");
        assert!(msg.contains("--accept-insecure-bind-warning"), "{msg}");
    }
}
