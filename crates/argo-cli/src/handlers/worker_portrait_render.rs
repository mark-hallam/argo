//! Presentation layer for `argo worker portrait` — JSON projections
//! and markdown rendering. Split out of `worker_portrait.rs` to keep
//! both files comfortably under the 500-LOC file invariant
//! (claude-cli-handlers-worker-portrait-split). Pure refactor; no
//! behaviour change.

use argo_identity::{AutoApprovalPolicy, Roster, Worker};
use argo_orchestrator::Handoff;
use argo_store::anomaly;
use serde_json::{json, Value};

pub(crate) fn identity_json(worker: &Worker) -> Value {
    json!({
        "worker_id": worker.id.to_string(),
        "tenant_id": worker.tenant_id.to_string(),
        "vertical_id": worker.vertical_id,
        "persona": worker.persona.name,
        "role_charter": worker.role.charter,
    })
}

pub(crate) fn policy_json(
    worker: &Worker,
    rosters: &[Roster],
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

pub(crate) fn handoff_json(h: Handoff) -> Value {
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

pub(crate) fn anomaly_json(e: anomaly::AnomalyEvent) -> Value {
    json!({
        "dimension": e.dimension,
        "action_id": e.action_id,
        "observed": e.observed,
        "z_score": e.z_score,
        "observed_at": e.observed_at.to_rfc3339(),
    })
}

pub(crate) fn json_envelope(p: &Value) -> Value {
    json!({"ok": true, "worker_portrait": p})
}

pub(crate) fn render_markdown(p: &Value) -> String {
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

pub(crate) fn micros_to_usd(v: u64) -> f64 {
    v as f64 / 1_000_000.0
}

fn usd_value(v: &Value) -> String {
    v.as_u64().map(|m| format!("${:.2}", micros_to_usd(m))).unwrap_or_else(|| "unlimited".into())
}

fn s<'a>(v: &'a Value, default: &'a str) -> &'a str {
    v.as_str().unwrap_or(default)
}
