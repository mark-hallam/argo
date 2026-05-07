//! `argo worker portrait` — paste-friendly worker summary.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{AutoApprovalPolicy, IdentityStore, RosterStore, Worker, WorkerId};
use argo_memory::{parse_skill_candidate, MemoryFile, MemoryStore};
use argo_orchestrator::{Handoff, OrchestratorStore};
use argo_store::{anomaly, SessionStore};
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

#[derive(Debug, Default, Serialize)]
struct ActionStats {
    window_hours: u32,
    completed: u64,
    failed: u64,
    average_latency_ms: Option<f64>,
    average_retries: Option<f64>,
}

pub async fn run(worker_id: &str, tenant: &str, json_out: bool) -> ArgoResult<()> {
    let ctx = Context::open().await?;
    let tenant_id = parse_tenant(tenant)?;
    let worker_id = parse_worker(worker_id)?;
    let worker = load_worker(&ctx.store, tenant_id, worker_id).await?;
    let portrait = build_portrait(&ctx, worker, Utc::now()).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json_envelope(&portrait))
                .map_err(|e| ArgoError::Provider(format!("serialize portrait JSON: {e}")))?
        );
    } else {
        print!("{}", render_markdown(&portrait));
    }
    Ok(())
}

struct Context {
    store: Arc<SessionStore>,
    workspace: WorkspaceId,
    cwd: std::path::PathBuf,
}

impl Context {
    async fn open() -> ArgoResult<Self> {
        let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
        let home = resolve_home(&cwd)?;
        ensure_dir(&home)?;
        let store = SessionStore::open(db_path(&home)).await?;
        store.migrate().await?;
        let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
        let (_tenant, workspace) = store.ensure_workspace(name, &cwd.to_string_lossy()).await?;
        Ok(Self {
            store: Arc::new(store),
            workspace,
            cwd,
        })
    }
}

async fn build_portrait(ctx: &Context, worker: Worker, now: DateTime<Utc>) -> ArgoResult<Value> {
    let workspace_policy = super::worker_actions_auto_approval::load_workspace_policy(&ctx.cwd)?;
    let rosters = load_rosters(&ctx.store, worker.tenant_id, worker.id)
        .await
        .unwrap_or_default();
    let roster_policies: Vec<_> = rosters.iter().map(|r| r.policy.clone()).collect();
    let resolved = worker.resolved_policy_with_rosters(&roster_policies, &workspace_policy);
    let actions = super::worker_actions_query::collect_latest_actions(None)
        .await
        .unwrap_or_default();
    let anomalies = anomaly::list_for_worker(
        ctx.store.pool(),
        &worker.id.to_string(),
        Some((now - chrono::Duration::hours(24)).timestamp_micros()),
        Some(5),
    )
    .await
    .unwrap_or_default();
    Ok(json!({
        "generated_at": now.to_rfc3339(),
        "identity": identity_json(&worker),
        "policy": policy_json(&worker, &rosters, &workspace_policy, &resolved),
        "budget": budget_json(&ctx.store, &worker.id.to_string(), &resolved, now).await?,
        "recent_action_stats": action_stats_for(&actions, &worker.id.to_string(), now),
        "open_handoffs": handoffs_for(&ctx.store, worker.tenant_id, worker.id).await.unwrap_or_default(),
        "skill_candidates": skill_candidates_for(ctx, &worker).await.unwrap_or_default(),
        "anomalies": anomalies.into_iter().map(anomaly_json).collect::<Vec<_>>(),
        "circuit_breaker": serde_json::to_value(&worker.circuit_state).unwrap_or_else(|_| json!({"state":"closed"})),
    }))
}

fn identity_json(worker: &Worker) -> Value {
    json!({
        "worker_id": worker.id.to_string(),
        "tenant_id": worker.tenant_id.to_string(),
        "vertical_id": worker.vertical_id,
        "persona": worker.persona.name,
        "role_charter": worker.role.charter,
    })
}

fn policy_json(
    worker: &Worker,
    rosters: &[argo_identity::Roster],
    workspace: &AutoApprovalPolicy,
    resolved: &AutoApprovalPolicy,
) -> Value {
    let source = if worker.auto_approval.enabled {
        "worker"
    } else if rosters.iter().any(|r| r.policy.enabled) {
        "roster"
    } else if workspace.enabled {
        "workspace"
    } else {
        "default_disabled"
    };
    let roster_lineage: Vec<_> = rosters
        .iter()
        .map(|r| json!({"id": r.id.to_string(), "name": r.name, "policy_enabled": r.policy.enabled}))
        .collect();
    json!({
        "resolved_policy": resolved,
        "source": source,
        "worker_policy_enabled": worker.auto_approval.enabled,
        "roster_lineage": roster_lineage,
        "workspace_policy_enabled": workspace.enabled,
    })
}

async fn load_worker(store: &SessionStore, tenant: TenantId, worker_id: WorkerId) -> ArgoResult<Worker> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    identities
        .get_worker(tenant, worker_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no worker '{worker_id}' for tenant '{tenant}'")))
}

async fn load_rosters(store: &Arc<SessionStore>, tenant: TenantId, worker: WorkerId) -> ArgoResult<Vec<argo_identity::Roster>> {
    let rosters = RosterStore::new(Arc::clone(store));
    rosters.ensure_schema().await?;
    rosters.rosters_for_worker(tenant, worker).await
}

async fn budget_json(store: &SessionStore, worker_id: &str, policy: &AutoApprovalPolicy, now: DateTime<Utc>) -> ArgoResult<Value> {
    ensure_budget_schema(store).await?;
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default();
    let month_start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now);
    let today = spend_since(store, worker_id, today_start.and_utc().timestamp()).await?;
    let month = spend_since(store, worker_id, month_start.timestamp()).await?;
    Ok(json!({
        "spend_today_usd_micros": today,
        "spend_month_usd_micros": month,
        "spend_today_usd": micros_to_usd(today),
        "spend_month_usd": micros_to_usd(month),
        "daily_cap_usd_micros": policy.daily_budget_usd_micros,
        "daily_headroom_usd": policy.daily_budget_usd_micros.map(|cap| micros_to_usd(cap.saturating_sub(today))),
    }))
}

async fn spend_since(store: &SessionStore, worker_id: &str, since: i64) -> ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(since)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("worker portrait budget: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

async fn ensure_budget_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("worker budget schema: {e}")))?;
        }
    }
    Ok(())
}

fn action_stats_for(
    actions: &[super::worker_actions::ActionRecord],
    worker_id: &str,
    now: DateTime<Utc>,
) -> ActionStats {
    let cutoff = now - chrono::Duration::hours(24);
    let mut stats = ActionStats {
        window_hours: 24,
        ..ActionStats::default()
    };
    let mut latency = Vec::new();
    let mut retries = Vec::new();
    for action in actions.iter().filter(|a| a.worker_id.to_string() == worker_id) {
        let Some(created) = parse_time(&action.created_at) else {
            continue;
        };
        if created < cutoff {
            continue;
        }
        match action.state.as_str() {
            "completed" => stats.completed += 1,
            "failed" => stats.failed += 1,
            _ => {}
        }
        if matches!(action.state.as_str(), "completed" | "failed") {
            retries.push(super::worker_actions_dispatch::count_retry_chain(action) - 1);
            if let Some(start) = action.executed_at.as_deref().and_then(parse_time) {
                latency.push((created - start).num_milliseconds().max(0) as f64);
            }
        }
    }
    stats.average_latency_ms = average(&latency);
    stats.average_retries = average_u32(&retries);
    stats
}

async fn handoffs_for(store: &Arc<SessionStore>, tenant: TenantId, worker: WorkerId) -> ArgoResult<Vec<Value>> {
    let orch = OrchestratorStore::new(Arc::clone(store));
    let rows = orch.list_pending_handoffs_for_worker(tenant, worker).await?;
    Ok(rows.into_iter().map(handoff_json).collect())
}

fn handoff_json(h: Handoff) -> Value {
    json!({
        "id": h.id.to_string(),
        "session_id": h.session_id.to_string(),
        "from_worker_id": h.from.to_string(),
        "expected_outcome": h.expected_outcome,
        "status": format!("{:?}", h.status).to_lowercase(),
        "created_at": h.created_at.to_rfc3339(),
        "deadline": h.deadline.map(|d| d.to_rfc3339()),
    })
}

async fn skill_candidates_for(ctx: &Context, worker: &Worker) -> ArgoResult<Vec<Value>> {
    let memory = MemoryStore::new(Arc::clone(&ctx.store));
    let (registry, _errors) = argo_vertical::load_dir(&super::vertical::resolve_verticals_dir())?;
    let mut rows = Vec::new();
    for (vertical_id, _loaded) in registry.list() {
        for entry in memory
            .list_for_vertical(ctx.workspace, MemoryFile::Memory, vertical_id)
            .await
            .unwrap_or_default()
        {
            let Some(candidate) = parse_skill_candidate(&entry.body) else {
                continue;
            };
            if candidate_worker(&entry.body).as_deref() != Some(&worker.id.to_string()) {
                continue;
            }
            rows.push(json!({
                "skill_name": candidate.skill_name,
                "vertical_id": vertical_id,
                "learned_at": entry.learned_at.to_rfc3339(),
                "memory_entry_id": entry.id.to_string(),
                "problem": candidate.problem,
            }));
        }
    }
    rows.sort_by(|a, b| b["learned_at"].as_str().cmp(&a["learned_at"].as_str()));
    rows.truncate(5);
    Ok(rows)
}

fn candidate_worker(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    ["worker_id", "proposed_by_worker_id", "source_worker_id", "worker"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_str).map(str::to_string))
}

fn anomaly_json(e: anomaly::AnomalyEvent) -> Value {
    json!({
        "dimension": e.dimension,
        "action_id": e.action_id,
        "observed": e.observed,
        "z_score": e.z_score,
        "observed_at": e.observed_at.to_rfc3339(),
    })
}

fn render_markdown(p: &Value) -> String {
    let identity = &p["identity"];
    let policy = &p["policy"];
    let budget = &p["budget"];
    let stats = &p["recent_action_stats"];
    let mut out = format!("# Worker Portrait: {}\n\n", s(&identity["persona"], "worker"));
    out.push_str(&format!(
        "## Identity\n\n- Worker: `{}`\n- Tenant: `{}`\n- Vertical: `{}`\n- Role: {}\n\n",
        s(&identity["worker_id"], "-"),
        s(&identity["tenant_id"], "-"),
        s(&identity["vertical_id"], "-"),
        s(&identity["role_charter"], "-")
    ));
    out.push_str(&format!(
        "## Auto-Approval Policy\n\n- Source: `{}`\n- Enabled: `{}`\n- Worker policy: `{}`\n- Workspace policy: `{}`\n",
        s(&policy["source"], "default_disabled"),
        policy["resolved_policy"]["enabled"].as_bool().unwrap_or(false),
        policy["worker_policy_enabled"].as_bool().unwrap_or(false),
        policy["workspace_policy_enabled"].as_bool().unwrap_or(false)
    ));
    render_rosters(&mut out, policy["roster_lineage"].as_array());
    out.push_str(&format!(
        "## Budget\n\n- Today: `${:.2}`\n- This month: `${:.2}`\n- Daily cap: {}\n- Headroom today: {}\n\n",
        budget["spend_today_usd"].as_f64().unwrap_or(0.0),
        budget["spend_month_usd"].as_f64().unwrap_or(0.0),
        usd_value(&budget["daily_cap_usd_micros"]),
        budget["daily_headroom_usd"].as_f64().map(|v| format!("${v:.2}")).unwrap_or_else(|| "unlimited".into())
    ));
    out.push_str(&format!(
        "## Recent Action Stats (24h)\n\n- Completed: `{}`\n- Failed: `{}`\n- Average latency: {}\n- Average retries: {}\n\n",
        stats["completed"].as_u64().unwrap_or(0),
        stats["failed"].as_u64().unwrap_or(0),
        stats["average_latency_ms"].as_f64().map(|v| format!("{v:.0}ms")).unwrap_or_else(|| "n/a".into()),
        stats["average_retries"].as_f64().map(|v| format!("{v:.2}")).unwrap_or_else(|| "n/a".into())
    ));
    render_rows(&mut out, "Open Handoffs", "No open handoffs.", &p["open_handoffs"], render_handoff);
    render_rows(&mut out, "Recent Skill Candidates", "No recent skill candidates for this worker.", &p["skill_candidates"], render_candidate);
    render_rows(&mut out, "Anomalies (24h)", "No anomalies detected.", &p["anomalies"], render_anomaly);
    out.push_str(&format!(
        "## Circuit Breaker\n\n- State: `{}`\n\n_Generated at {}_\n",
        s(&p["circuit_breaker"]["state"], "closed"),
        s(&p["generated_at"], "-")
    ));
    out
}

fn render_rosters(out: &mut String, rows: Option<&Vec<Value>>) {
    let Some(rows) = rows.filter(|r| !r.is_empty()) else {
        out.push_str("- Rosters: none\n\n");
        return;
    };
    out.push_str("- Rosters:\n");
    for r in rows {
        out.push_str(&format!(
            "  - `{}` ({}, enabled={})\n",
            s(&r["name"], "-"),
            s(&r["id"], "-"),
            r["policy_enabled"].as_bool().unwrap_or(false)
        ));
    }
    out.push('\n');
}

fn render_rows(out: &mut String, title: &str, empty: &str, rows: &Value, f: fn(&Value) -> String) {
    out.push_str(&format!("## {title}\n\n"));
    let Some(rows) = rows.as_array().filter(|r| !r.is_empty()) else {
        out.push_str(empty);
        out.push_str("\n\n");
        return;
    };
    for row in rows {
        out.push_str(&f(row));
    }
    out.push('\n');
}

fn render_handoff(v: &Value) -> String {
    format!(
        "- `{}` from `{}`: {}\n",
        s(&v["id"], "-"),
        s(&v["from_worker_id"], "-"),
        s(&v["expected_outcome"], "-")
    )
}

fn render_candidate(v: &Value) -> String {
    format!(
        "- `{}` ({}) learned {}\n",
        s(&v["skill_name"], "-"),
        s(&v["vertical_id"], "-"),
        s(&v["learned_at"], "-")
    )
}

fn render_anomaly(v: &Value) -> String {
    format!(
        "- `{}` on `{}`: observed {:.2}, z={:+.2}\n",
        s(&v["dimension"], "-"),
        s(&v["action_id"], "-"),
        v["observed"].as_f64().unwrap_or(0.0),
        v["z_score"].as_f64().unwrap_or(0.0)
    )
}

fn json_envelope(p: &Value) -> Value {
    json!({"ok": true, "worker_portrait": p})
}

fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value).map(TenantId).map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

fn parse_worker(value: &str) -> ArgoResult<WorkerId> {
    value.parse().map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}

fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw).ok().map(|dt| dt.with_timezone(&Utc))
}

fn average(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn average_u32(values: &[u32]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64)
}

fn micros_to_usd(v: u64) -> f64 {
    v as f64 / 1_000_000.0
}

fn usd_value(v: &Value) -> String {
    v.as_u64().map(|m| format!("${:.2}", micros_to_usd(m))).unwrap_or_else(|| "unlimited".into())
}

fn s<'a>(v: &'a Value, default: &'a str) -> &'a str {
    v.as_str().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::{Persona, Role};

    fn worker() -> Worker {
        Worker::draft(TenantId::new(), "gtm-engineer", Persona::named("Maya", "op"), Role::new("Own approved marketplace follow-up."))
    }

    fn portrait() -> Value {
        let w = worker(); json!({"generated_at":"2026-05-06T00:00:00Z","identity":identity_json(&w),"policy":{"resolved_policy":{"enabled":true},"source":"worker","worker_policy_enabled":true,"workspace_policy_enabled":false,"roster_lineage":[{"id":"r1","name":"ops","policy_enabled":true}]},"budget":{"spend_today_usd":1.0,"spend_month_usd":2.0,"daily_cap_usd_micros":5000000,"daily_headroom_usd":4.0},"recent_action_stats":{"window_hours":24,"completed":1,"failed":1,"average_latency_ms":1500.0,"average_retries":0.5},"open_handoffs":[{"id":"h1","from_worker_id":"w2","expected_outcome":"finish"}],"skill_candidates":[{"skill_name":"post_listing","vertical_id":"gtm-engineer","learned_at":"now"}],"anomalies":[],"circuit_breaker":{"state":"closed"}})
    }

    #[test]
    fn markdown_renders_all_sections_without_ansi() {
        let body = render_markdown(&portrait());
        for heading in ["Identity", "Auto-Approval Policy", "Budget", "Recent Action Stats", "Open Handoffs", "Recent Skill Candidates", "Anomalies", "Circuit Breaker"] {
            assert!(body.contains(heading), "missing {heading}");
        }
        assert!(!body.contains('\u{1b}'));
    }

    #[test]
    fn markdown_gracefully_reports_no_anomalies() {
        assert!(render_markdown(&portrait()).contains("No anomalies detected"));
    }

    #[test]
    fn json_uses_universal_ok_envelope() {
        let value = json_envelope(&portrait());
        assert_eq!(value["ok"], true);
        assert_eq!(value["worker_portrait"]["identity"]["persona"], "Maya");
    }

    #[test]
    fn action_stats_count_terminal_rows_and_retry_average() {
        let w = worker().id;
        let now = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).single().unwrap();
        let rows = vec![
            action(w, "completed", now, Some(now - chrono::Duration::seconds(2)), vec![]),
            action(w, "failed", now, Some(now - chrono::Duration::seconds(1)), vec!["argo://action/old".into()]),
            action(WorkerId::new(), "failed", now, None, vec![]),
        ];
        let stats = action_stats_for(&rows, &w.to_string(), now);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.average_retries, Some(0.5));
        assert_eq!(stats.average_latency_ms, Some(1500.0));
    }

    fn action(worker_id: WorkerId, state: &str, at: DateTime<Utc>, executed_at: Option<DateTime<Utc>>, evidence: Vec<String>) -> super::super::worker_actions::ActionRecord {
        super::super::worker_actions::ActionRecord { schema_key: "worker_action".into(), action_id: Uuid::now_v7().to_string(), job_id: "job".into(), worker_id, item_id: None, action_type: "post_listing".into(), risk_level: "low".into(), requires_approval: false, state: state.into(), payload: json!({}), approved_by: None, approved_at: None, executed_at: executed_at.map(|t| t.to_rfc3339()), platform: None, result: None, evidence, blocked_reason: None, created_at: at.to_rfc3339() }
    }
}
