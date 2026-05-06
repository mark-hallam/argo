//! `argo gateway` — start / stop / status. PID file at
//! `<ARGO_HOME>/gateway.pid` mediates the start↔stop handshake.
//!
//! Sub-modules:
//!   - `lifecycle`: PidGuard + signal helpers (SIGTERM/SIGINT).
//!   - `routes`:    HTTP route dispatch + per-endpoint handlers.
mod lifecycle;
mod metrics;
mod metrics_actions;
mod routes;
mod sweepers;
mod routes_audit;
mod routes_audit_rollups;
pub(crate) mod routes_catalog;
mod routes_channels;
mod routes_costs_by_day;
mod routes_counts;
mod routes_cron;
mod routes_dispatch_costs;
mod routes_dispatch_forge;
mod routes_dispatch_post;
mod routes_dispatch_rollups;
mod routes_forge;
mod routes_forge_promote;
mod routes_forge_rollups;
mod routes_health;
mod routes_lists;
mod routes_messages_rollups;
mod routes_meta;
mod routes_msgs;
mod routes_orchestrator;
mod routes_post;
mod routes_post_admin;
mod routes_post_forge;
mod routes_post_writes;
mod routes_providers;
mod routes_query;
mod routes_search;
mod routes_session;
mod routes_session_approval;
mod routes_session_audit_rollups;
mod routes_session_clone;
mod routes_session_get;
mod routes_session_skills;
mod routes_session_stats;
mod routes_sessions_rollups;
mod routes_skill_refine;
mod routes_skills;
mod routes_skills_show;
mod routes_top;
mod routes_verticals;
mod routes_verticals_doctor;
mod routes_verticals_lint;
mod routes_verticals_run;
mod routes_workspace;
mod streaming;

use crate::cli::GatewayCmd;
use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use lifecycle::{pid_is_alive, shutdown_signal, signal_term, PidGuard};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run(cmd: GatewayCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    match cmd {
        GatewayCmd::Status { json } => status(&cfg, json),
        GatewayCmd::Start {
            bind,
            port,
            max_requests,
        } => start(&bind, port, max_requests).await,
        GatewayCmd::Stop => stop().await,
        GatewayCmd::Routes {
            json,
            format,
            method,
            grep,
            limit,
        } => print_routes(json, &format, method.as_deref(), grep.as_deref(), limit),
        GatewayCmd::Probe {
            kind,
            host,
            port,
            json,
        } => super::gateway_probe::run(kind, host, port, json).await,
    }
}

/// `argo gateway routes [--json|--format] [--method] [--grep] [--limit]`
/// — pretty-print the static route catalog. Filters apply in
/// order: method → grep → limit.
fn print_routes(
    json: bool,
    format: &str,
    method: Option<&str>,
    grep: Option<&str>,
    limit: Option<usize>,
) -> ArgoResult<()> {
    let (_status, body) = routes_catalog::routes_response(method);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse: {e}")))?;
    let mut rows = parsed.as_array().cloned().unwrap_or_default();
    if let Some(needle) = grep {
        let lower = needle.to_lowercase();
        rows.retain(|r| {
            let path = r["path"].as_str().unwrap_or("").to_lowercase();
            let desc = r["description"].as_str().unwrap_or("").to_lowercase();
            path.contains(&lower) || desc.contains(&lower)
        });
    }
    if let Some(n) = limit {
        rows.truncate(n);
    }
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            println!(
                "{}",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
            );
            return Ok(());
        }
        "markdown" => {
            println!("# Gateway routes ({} endpoints)", rows.len());
            println!();
            if rows.is_empty() {
                return Ok(());
            }
            println!("| method | path | description |");
            println!("| :---: | --- | --- |");
            for row in &rows {
                let m = row["method"].as_str().unwrap_or("?");
                let p = row["path"].as_str().unwrap_or("?");
                let d = row["description"]
                    .as_str()
                    .unwrap_or("")
                    .replace('|', "\\|");
                println!("| `{m}` | `{p}` | {d} |");
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    println!("argo gateway routes ({} endpoints)", rows.len());
    let width = (rows.len().max(1) as f64).log10().ceil() as usize;
    for (i, row) in rows.iter().enumerate() {
        let m = row["method"].as_str().unwrap_or("?");
        let p = row["path"].as_str().unwrap_or("?");
        let d = row["description"].as_str().unwrap_or("");
        let n = i + 1;
        println!("  {n:>width$}. {m:<5} {p:<40} {d}");
    }
    Ok(())
}

fn pid_path() -> ArgoResult<PathBuf> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    Ok(home.join("gateway.pid"))
}

async fn open_store(cwd: &std::path::Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn start(bind: &str, port: u16, max_requests: Option<u64>) -> ArgoResult<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| ArgoError::Provider(format!("bind {addr}: {e}")))?;

    // Open the session store so the routes that touch it can serve.
    // Failure to open is logged but doesn't kill the gateway —
    // /health and /version still answer.
    let cwd_for_store =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = match open_store(&cwd_for_store).await {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            eprintln!("argo gateway: store open failed ({e}); /sessions etc. will return 503");
            None
        }
    };
    let _sweeper = store
        .as_ref()
        .map(|s| sweepers::spawn_handoff_sweeper(Arc::clone(s)));

    // PID file. If one already exists with a live PID, refuse to
    // start. If it exists but the process is gone, overwrite.
    let pidfile = pid_path()?;
    if pidfile.exists() {
        if let Ok(prev) = std::fs::read_to_string(&pidfile) {
            if let Ok(prev_pid) = prev.trim().parse::<i32>() {
                if pid_is_alive(prev_pid) {
                    return Err(ArgoError::Provider(format!(
                        "gateway already running (pid {prev_pid}, see {})",
                        pidfile.display()
                    )));
                }
                eprintln!(
                    "argo gateway: stale pid {prev_pid} in {}; overwriting",
                    pidfile.display()
                );
            }
        }
    }
    let pid = std::process::id();
    std::fs::write(&pidfile, format!("{pid}\n"))
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", pidfile.display())))?;
    let _guard = PidGuard(pidfile.clone());

    metrics::init_boot_time();
    eprintln!("argo gateway: listening on {addr} (pid {pid})");
    if let Some(s) = store.as_ref() {
        // #718: print the cwd-bound workspace's name (matches the path
        // every request handler resolves to). Pre-fix this used
        // `first_workspace_name()` and on multi-workspace dbs surfaced
        // a stale workspace's name.
        if let Ok(id) = cwd_workspace(s).await {
            if let Ok(Some((_, name, _, _))) = s.workspace_info(id).await {
                eprintln!("argo gateway: workspace = '{name}'");
            }
        }
    }
    eprintln!("  GET  /health   → {{\"status\":\"ok\"}}");
    eprintln!("  GET  /version  → {{\"version\":\"<crate>\"}}");
    eprintln!("  GET  /sessions[?since-minutes=N] → recent sessions (JSON array; carries spent_usd_micros)");
    eprintln!(
        "  GET  /sessions/<uuid> → one session header + message_count + tool_call_count + spend"
    );
    eprintln!("  GET  /sessions/<uuid>/messages[?limit=N&role=user&since-minutes=M] → messages on the session");
    eprintln!("  GET  /sessions/<uuid>/messages/last → just the last message");
    eprintln!("  GET  /sessions/<uuid>/spend → per-session spend ledger rows + total");
    eprintln!("  GET  /sessions/<uuid>/spend/total → single-number total for one session");
    eprintln!("  POST /sessions → create a session row ({{model, provider, title?}})");
    eprintln!("  POST /sessions/<uuid>/messages → append a message ({{role, content}})");
    eprintln!("  POST /sessions/<uuid>/runs → run one agent turn ({{user?, stream:false, max_tool_rounds?}})");
    eprintln!("  POST /sessions/<uuid>/runs/stream → chunked-NDJSON stream of audit events as the run progresses");
    eprintln!("  POST /sessions/<uuid>/steer → inject a system-role steer ({{text}})");
    eprintln!("  POST /sessions/<uuid>/replay → replay user prompts in a fresh session ({{model?, provider?}})");
    eprintln!("  GET  /sessions/<uuid>/skills → skills attached to the session");
    eprintln!("  GET  /sessions/<uuid>/judge  → judge_record rows for the session");
    eprintln!("  POST /sessions/<uuid>/skills/{{attach,detach}} → manage per-session skill set");
    eprintln!("  POST /sessions/<uuid>/delete → delete a session + dependent rows");
    eprintln!("  POST /sessions/<uuid>/title  → rename ({{title}}; empty clears)");
    eprintln!("  POST /sessions/<uuid>/end    → mark a session ended (stamps ended_at)");
    eprintln!("  POST /sessions/<uuid>/replay/stream → chunked-NDJSON stream of replay events");
    eprintln!("  POST /channels/email/webhook → Postmark inbound approval webhook");
    eprintln!("  GET  /skills   → active skills (?include-superseded=true for full history)");
    eprintln!("  GET  /skills/<name> → one skill view (full body + frontmatter)");
    eprintln!("  GET  /skills/<name>/versions → supersede chain for that name");
    eprintln!("  POST /skills   → install a SKILL.md document ({{raw}})");
    eprintln!("  POST /memory   → append a memory entry ({{file: memory|user|soul|agents, body}})");
    eprintln!("  GET  /memory[?file=memory|user|soul|agents] → active memory entries");
    eprintln!("  GET  /memory/<id> → one memory entry (full body)");
    eprintln!("  GET  /memory/<id>/versions → supersede chain for that entry");
    eprintln!("  GET  /audit    → audit chain summary (ok / rows / breaks / kinds)");
    eprintln!("  GET  /providers → workspace provider manifests + their model lists");
    eprintln!("  GET  /providers/<id> → one provider manifest by basename");
    eprintln!("  GET  /providers/<id>/models → just the [[models]] array");
    eprintln!("  GET  /tools    → registered in-core tools + JSON-Schema parameters");
    eprintln!("  GET  /tools/<name> → one tool's name + description + parameters");
    eprintln!("  GET  /search?q=…[&limit=N&session=<id>] → FTS5 hits across the message corpus");
    eprintln!("  GET  /workspace/info → first workspace row (id, name, root_path, created_at)");
    eprintln!("  GET  /workspace/artifacts[?limit=N] → recorded marketplace pushes (oci_ref + digest + pushed_at)");
    eprintln!("  GET  /forge/runs[?limit=N&since-minutes=M] → recorded forge runs");
    eprintln!("  GET  /forge/runs/<id>      → one forge_run row");
    eprintln!("  GET  /cron                 → scheduled cron jobs (workspace cron/jobs/)");
    eprintln!("  GET  /cron/<name>          → one cron job by name");
    eprintln!(
        "  POST /cron                 → create a job ({{name, schedule, prompt, platform, chat}})"
    );
    eprintln!("  POST /workspace/init       → register a workspace row ({{name, root_path}})");
    eprintln!("  POST /judge/<record_id>/override → override a judge verdict ({{to}})");
    eprintln!("  POST /cron/<name>/delete   → remove the on-disk job file");
    eprintln!("  POST /forge/optimize → kick off `argo forge optimize` ({{skill, examples, iterations?, budget_usd?}})");
    eprintln!("  POST /forge/optimize/stream → chunked-NDJSON live forge progress");
    eprintln!("  POST /forge/bench    → kick off `argo forge bench` ({{skill, examples, provider?, model?}})");
    eprintln!("  POST /forge/runs/<id>/abort → flip a run's status to 'aborted'");
    eprintln!("  POST /forge/runs/<id>/promote → promote that run's candidate to active");
    eprintln!("  POST /skills/<name>/promote → promote a version to active ({{version}})");
    eprintln!("  POST /skills/<name>/{{lock,unlock,refine}} → lock / unlock / append hint refine");
    eprintln!("  POST /memory/<id>/{{lock,unlock,forget,edit}} → admin ops on a memory entry");
    eprintln!("  GET  /metrics        → Prometheus-style text exposition (text/plain)");
    eprintln!("  GET  /routes  → discovery: every route + a one-line description");
    eprintln!("  GET  /doctor       → JSON snapshot of `argo doctor` (cwd, db, cron, creds, …)");
    eprintln!("  GET  /health/full  → deep health (audit chain + skill + memory provenance)");
    eprintln!("  GET  /audit/sessions/<uuid>[?kind=<kind>] → per-session audit chain (JSON array)");
    eprintln!("  GET  /audit/sessions/<uuid>/verify → verify a session's chain integrity");
    eprintln!("  GET  /costs    → cost ceilings (per_session/day/month + notify_at_pct)");
    eprintln!("  GET  /db/stats → per-table row counts (sessions, messages, audit, …)");
    eprintln!("  POST /db/vacuum → run VACUUM on the SQLite store");
    eprintln!("  POST /db/prune  → prune sessions older than N days ({{older_than_days?}})");
    eprintln!("  GET  /costs/by-model    → top spend buckets grouped by (provider, model)");
    eprintln!("  GET  /costs/by-session  → top spend buckets grouped by session_id");
    eprintln!("press Ctrl-C or `argo gateway stop` to terminate");

    let mut served = 0_u64;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                eprintln!("argo gateway: shutdown signal received; exiting cleanly");
                return Ok(());
            }
            accept = listener.accept() => {
                let (mut stream, _peer) = accept
                    .map_err(|e| ArgoError::Provider(format!("accept: {e}")))?;
                let mut buf = [0_u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
                if streaming::is_streaming_runs_request(req) {
                    let _ = streaming::serve_streamed_run(req, store.as_ref(), &mut stream).await;
                    let _ = stream.shutdown().await;
                } else if metrics::is_metrics_request(req) {
                    let bytes = metrics::http_response(store.as_ref()).await;
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                } else if routes_channels::is_email_webhook(req) {
                    let (status_line, body) = routes_channels::email_webhook_response(req).await;
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_orchestrator::dispatch_route(req, store.as_ref()).await {
                    let resp = format!("HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else if let Some((status_line, body)) = routes_skills::dispatch_route(req, store.as_ref()).await {
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else {
                    let (status_line, body) = routes::dispatch_route(req, store.as_ref()).await;
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                }
                served += 1;
                if max_requests.is_some_and(|cap| served >= cap) {
                    eprintln!(
                        "argo gateway: served {served} request(s); exiting per --max-requests"
                    );
                    return Ok(());
                }
            }
        }
    }
}

async fn stop() -> ArgoResult<()> {
    let pidfile = pid_path()?;
    let raw = std::fs::read_to_string(&pidfile)
        .map_err(|e| ArgoError::Provider(format!("no PID file at {}: {e}", pidfile.display())))?;
    let pid: i32 = raw
        .trim()
        .parse()
        .map_err(|e| ArgoError::Provider(format!("invalid PID in {}: {e}", pidfile.display())))?;
    if !pid_is_alive(pid) {
        let _ = std::fs::remove_file(&pidfile);
        return Err(ArgoError::Provider(format!(
            "no live process at pid {pid} (cleaned stale {})",
            pidfile.display()
        )));
    }
    if let Err(e) = signal_term(pid) {
        return Err(ArgoError::Provider(format!("kill pid {pid}: {e}")));
    }
    println!("argo gateway: sent SIGTERM to pid {pid}");
    Ok(())
}

fn status(cfg: &ArgoConfig, json: bool) -> ArgoResult<()> {
    // Best-effort: read the PID file so the JSON form can report
    // a real running state, not just the config bits.
    let pid_alive = pid_path()
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
