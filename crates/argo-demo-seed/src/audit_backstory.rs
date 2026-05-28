//! Pass-B audit-backstory seeder — 34 varied action-type rows.
//!
//! The base Pass B dressing emits only `TurnStart` / `AssistantMessage`
//! / `ToolCall` / `ToolResult` / `TurnEnd` rows, all attributed to the
//! same five canonical workers. That reads as a homogeneous "log spam"
//! wall on `/audit` and contradicts the landing-page "audit-first AI
//! workforce" promise (UX audit F3 / P1).
//!
//! This module seeds an additional 34 rows whose displayed
//! `action_type` (resolved from `payload.action_type`) spans
//! `skill.invoked`, `approval.granted`, `approval.escalated`,
//! `delegation.assigned`, `worker.hired`, `cu.action.taken`,
//! `mcp.tool.call`, `workflow.completed`, `memory.write`,
//! `skill.installed`, and `policy.evaluated` — proving Claim 2
//! (delegation), Claim 4 (skills) and Claim 5 (computer-use + MCP
//! substrate) have on-camera evidence.
//!
//! Each row carries `memory_loaded_summary` / `skill_summary` /
//! `plan_selected_summary` / `outcome_summary` plus provenance
//! (`model_id`, `prompt_template_hash`, `policy_hash`, `signature`),
//! so `/audit` drill-in renders ≥3 non-empty decision-chain stages.
//! The `created_at` is `now` (same compromise as Pass B); the intended
//! 7-day spread is stamped into `payload.synthetic_ts` only.
//!
//! Brief: `docs/build-plan/streams/rebase-fix-audit-feed-action-kind-variety.md`.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{Map, Value};

use crate::personas::DEMO_SEED_PERSONAS;
use crate::SeedResult;

/// One backstory row: which underlying [`AuditKind`] to write, the
/// `action_type` label the `/audit` table should display, which Scene-1
/// persona owns the row, and a one-line skill / surface label that
/// drives the decision-chain `skill_summary` body.
struct BackstoryRow {
    kind: AuditKind,
    action_type: &'static str,
    worker_idx: usize,
    skill_label: &'static str,
    /// Operator-facing approver name on approval.* rows; `None` elsewhere.
    approver_name: Option<&'static str>,
    /// Source persona on delegation.* rows; `None` elsewhere.
    delegated_by_idx: Option<usize>,
    /// `cu.action.taken` surface (`Notes.app`, browser, …) or `None`.
    cu_surface: Option<&'static str>,
    /// `mcp.tool.call` connector (`zendesk`, `salesforce`, …) or `None`.
    mcp_connector: Option<&'static str>,
}

const fn base(
    kind: AuditKind,
    action_type: &'static str,
    worker_idx: usize,
    skill_label: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind,
        action_type,
        worker_idx,
        skill_label,
        approver_name: None,
        delegated_by_idx: None,
        cu_surface: None,
        mcp_connector: None,
    }
}

const fn granted(
    worker_idx: usize,
    skill_label: &'static str,
    approver: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind: AuditKind::ApprovalGranted,
        action_type: "approval.granted",
        worker_idx,
        skill_label,
        approver_name: Some(approver),
        delegated_by_idx: None,
        cu_surface: None,
        mcp_connector: None,
    }
}

const fn escalated(
    worker_idx: usize,
    skill_label: &'static str,
    approver: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind: AuditKind::EscalationRequired,
        action_type: "approval.escalated",
        worker_idx,
        skill_label,
        approver_name: Some(approver),
        delegated_by_idx: None,
        cu_surface: None,
        mcp_connector: None,
    }
}

const fn delegate(
    delegated_by_idx: usize,
    worker_idx: usize,
    skill_label: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind: AuditKind::AssistantMessage,
        action_type: "delegation.assigned",
        worker_idx,
        skill_label,
        approver_name: None,
        delegated_by_idx: Some(delegated_by_idx),
        cu_surface: None,
        mcp_connector: None,
    }
}

const fn cu(
    worker_idx: usize,
    surface: &'static str,
    skill_label: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind: AuditKind::ToolCall,
        action_type: "cu.action.taken",
        worker_idx,
        skill_label,
        approver_name: None,
        delegated_by_idx: None,
        cu_surface: Some(surface),
        mcp_connector: None,
    }
}

const fn mcp(
    worker_idx: usize,
    connector: &'static str,
    skill_label: &'static str,
) -> BackstoryRow {
    BackstoryRow {
        kind: AuditKind::ToolCall,
        action_type: "mcp.tool.call",
        worker_idx,
        skill_label,
        approver_name: None,
        delegated_by_idx: None,
        cu_surface: None,
        mcp_connector: Some(connector),
    }
}

/// The 34-row backstory recipe. Worker indices are into
/// [`DEMO_SEED_PERSONAS`] (`0=Mariah, 1=Peta, 2=Priya, 3=Jamal,
/// 4=Nori`).
#[rustfmt::skip]
const BACKSTORY: &[BackstoryRow] = &[
    // ── 8× skill.invoked (Claim 4 evidence) ───────────────────────
    base(AuditKind::ToolCall, "skill.invoked", 0, "competitor-pricing-scrape"),
    base(AuditKind::ToolCall, "skill.invoked", 0, "listing-photo-touchup"),
    base(AuditKind::ToolCall, "skill.invoked", 1, "delivery-date-pulse"),
    base(AuditKind::ToolCall, "skill.invoked", 1, "blocker-unwedge-routing"),
    base(AuditKind::ToolCall, "skill.invoked", 2, "candidate-screen-rubric"),
    base(AuditKind::ToolCall, "skill.invoked", 3, "ticket-triage-rubric"),
    base(AuditKind::ToolCall, "skill.invoked", 3, "refund-issuance-policy"),
    base(AuditKind::ToolCall, "skill.invoked", 4, "voice-conformance-check"),
    // ── 6× approval.granted ───────────────────────────────────────
    granted(0, "listing-repost-cap",          "Mark Hallam"),
    granted(0, "buyer-discount-10pct",        "Mark Hallam"),
    granted(2, "interview-slot-confirm",      "Mark Hallam"),
    granted(3, "refund-29-99-cap",            "Mark Hallam"),
    granted(4, "comms-broadcast-queue",       "Mark Hallam"),
    granted(1, "delivery-deadline-pull-in",   "Mark Hallam"),
    // ── 2× approval.escalated (high-severity → manager) ───────────
    escalated(0, "discount-above-policy-ceiling", "Ops Manager"),
    escalated(3, "refund-above-rubric-cap",       "Ops Manager"),
    // ── 4× delegation.assigned (Claim 2 evidence) ─────────────────
    // delegated_by_idx → worker_idx; chosen so by ≠ worker.
    delegate(0, 2, "candidate-shortlist-research"),
    delegate(1, 0, "status-rollup-prep"),
    delegate(3, 2, "knowledge-base-curation"),
    delegate(4, 0, "competitor-pricing-narrative"),
    // ── 3× worker.hired ───────────────────────────────────────────
    base(AuditKind::WorkerHired, "worker.hired", 1, "project-coordination-onboard"),
    base(AuditKind::WorkerHired, "worker.hired", 2, "recruiting-onboard"),
    base(AuditKind::WorkerHired, "worker.hired", 3, "support-tier1-onboard"),
    // ── 4× cu.action.taken (Claim 5 substrate) ────────────────────
    cu(0, "Notes.app",  "draft-listing-notes"),
    cu(0, "Safari",     "capture-competitor-screenshots"),
    cu(2, "Slack",      "post-candidate-confirmation"),
    cu(3, "Mail.app",   "send-refund-receipt"),
    // ── 3× mcp.tool.call (Claim 5 substrate) ──────────────────────
    mcp(3, "zendesk",    "ticket-fetch"),
    mcp(3, "salesforce", "account-search"),
    mcp(0, "stripe",     "refund-status-check"),
    // ── 4× filler (workflow.completed / memory.write / skill.installed / policy.evaluated) ──
    base(AuditKind::VerticalRunCompleted, "workflow.completed", 0, "customer-research-followup"),
    base(AuditKind::AssistantMessage,     "memory.write",       1, "delivery-pulse-snapshot"),
    base(AuditKind::VerticalInstalled,    "skill.installed",    4, "voice-conformance-check"),
    base(AuditKind::AssistantMessage,     "policy.evaluated",   3, "refund-policy-v3"),
];

/// Total row count this module will emit on a successful call.
#[must_use]
pub const fn backstory_row_count() -> usize {
    BACKSTORY.len()
}

/// Insert the 34 backstory audit rows under `tenant_id`, attributing
/// each to one of the five canonical [`DEMO_SEED_PERSONAS`] workers.
///
/// `worker_ids` must be the in-order ids returned by
/// `seed_demo_workers` (0=Mariah, 1=Peta, 2=Priya, 3=Jamal, 4=Nori) —
/// the row indices are positional into that slice.
///
/// Returns the number of rows inserted; `0` when `worker_ids` is
/// empty (the audit feed has nobody to attribute rows to).
///
/// # Errors
///
/// Returns any underlying audit-sink / SQLite error.
pub async fn seed_audit_backstory(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let now = Utc::now();
    let mut inserted = 0;
    for (i, spec) in BACKSTORY.iter().enumerate() {
        let worker_id = worker_ids[spec.worker_idx % worker_ids.len()];
        let worker_name = DEMO_SEED_PERSONAS[spec.worker_idx % DEMO_SEED_PERSONAS.len()].name;
        // Spread the synthetic timestamps across the past 7 days; the
        // displayed timestamp on /audit is still `created_at` (now) per
        // the existing Pass B compromise, but the synthetic_ts is
        // surfaced in payload + detail view.
        let minute_offset = ((i as i64) * 313) % (7 * 24 * 60);
        let synthetic_ts = now - chrono::Duration::minutes(minute_offset);
        let payload = build_payload(spec, worker_id, worker_name, i, synthetic_ts);
        sink.write(None, spec.kind, payload, None).await?;
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

fn build_payload(
    spec: &BackstoryRow,
    worker_id: WorkerId,
    worker_name: &'static str,
    row_idx: usize,
    synthetic_ts: chrono::DateTime<Utc>,
) -> Value {
    let mut obj = Map::new();
    obj.insert("worker_id".into(), Value::String(worker_id.to_string()));
    obj.insert("worker_name".into(), Value::String(worker_name.into()));
    obj.insert("action_type".into(), Value::String(spec.action_type.into()));
    obj.insert("verb".into(), Value::String(spec.action_type.into()));
    obj.insert("channel".into(), Value::String(channel_for(spec).into()));
    obj.insert("outcome_status".into(), Value::String("succeeded".into()));
    obj.insert("synthetic_ts".into(), Value::String(synthetic_ts.to_rfc3339()));
    obj.insert("is_dressing".into(), Value::Bool(true));
    obj.insert("pass".into(), Value::String("B".into()));
    obj.insert("row_index".into(), Value::Number(row_idx.into()));
    // Decision-chain bodies (≥3 non-empty stages → drill-in pass).
    obj.insert(
        "memory_loaded_summary".into(),
        Value::String(format!(
            "loaded {} entries from {worker_name}'s working memory",
            3 + (row_idx % 4)
        )),
    );
    obj.insert(
        "skill_summary".into(),
        Value::String(format!("{} skill (v1)", spec.skill_label)),
    );
    obj.insert(
        "plan_selected_summary".into(),
        Value::String(format!(
            "chose {} from 3 candidates (confidence 0.{:02})",
            spec.skill_label,
            70 + (row_idx % 25)
        )),
    );
    obj.insert(
        "outcome_summary".into(),
        Value::String(format!(
            "{} completed for {worker_name} via {}",
            spec.action_type,
            channel_for(spec)
        )),
    );
    // Provenance fields read by `routes_audit_detail_render::build_provenance`.
    obj.insert("model_id".into(), Value::String("anthropic-claude-sonnet-4-5".into()));
    obj.insert("model_version".into(), Value::String("2026-01-01".into()));
    obj.insert("prompt_template_id".into(), Value::String("demo-backstory".into()));
    obj.insert("prompt_template_hash".into(), Value::String(fake_hex(row_idx, "tmpl")));
    obj.insert("skill_id".into(), Value::String(spec.skill_label.into()));
    obj.insert("skill_version".into(), Value::String("1".into()));
    obj.insert("policy_id".into(), Value::String("default-v1".into()));
    obj.insert("policy_version".into(), Value::String("1".into()));
    obj.insert("policy_hash".into(), Value::String(fake_hex(row_idx, "pol")));
    obj.insert("signature".into(), Value::String(fake_hex(row_idx, "sig")));
    if let Some(approver) = spec.approver_name {
        obj.insert(
            "approver_id".into(),
            Value::String(format!("operator-{}", slugify(approver))),
        );
        obj.insert("approver_name".into(), Value::String(approver.into()));
    }
    if let Some(idx) = spec.delegated_by_idx {
        let by = DEMO_SEED_PERSONAS[idx % DEMO_SEED_PERSONAS.len()].name;
        obj.insert("delegated_by".into(), Value::String(by.into()));
    }
    if let Some(surface) = spec.cu_surface {
        obj.insert("executor_kind".into(), Value::String("computer-use".into()));
        let mut meta = Map::new();
        meta.insert("surface".into(), Value::String(surface.into()));
        meta.insert("os".into(), Value::String("macOS".into()));
        obj.insert("executor_meta".into(), Value::Object(meta));
    }
    if let Some(connector) = spec.mcp_connector {
        obj.insert("executor_kind".into(), Value::String("mcp".into()));
        let mut meta = Map::new();
        meta.insert("connector".into(), Value::String(connector.into()));
        obj.insert("executor_meta".into(), Value::Object(meta));
    }
    Value::Object(obj)
}

fn channel_for(spec: &BackstoryRow) -> &'static str {
    match spec.action_type {
        "approval.granted" | "approval.escalated" => "slack",
        "cu.action.taken" => "computer-use",
        "mcp.tool.call" => "mcp",
        "worker.hired" => "web",
        _ => "agent",
    }
}

fn fake_hex(row_idx: usize, salt: &str) -> String {
    // Stable, deterministic 64-char hex per (row_idx, salt) without
    // pulling sha2 into this crate — repeats a seeded nibble pattern so
    // `audit_backstory` rows look chain-anchored to a human scanner but
    // the real chain anchor still comes from the sink.
    let seed = (row_idx as u64).wrapping_add(salt.bytes().map(u64::from).sum::<u64>());
    let mut out = String::with_capacity(64);
    let mut v = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for _ in 0..16 {
        out.push_str(&format!("{:04x}", v as u16));
        v = v.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0xA5);
    }
    out
}

fn slugify(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::AuditKind;
    use std::collections::HashSet;

    #[test]
    fn backstory_emits_thirty_four_rows() {
        assert_eq!(BACKSTORY.len(), 34);
        assert_eq!(backstory_row_count(), 34);
    }

    #[test]
    fn recipe_action_type_distribution_matches_brief() {
        let count = |label: &str| {
            BACKSTORY
                .iter()
                .filter(|r| r.action_type == label)
                .count()
        };
        assert_eq!(count("skill.invoked"), 8);
        assert_eq!(count("approval.granted"), 6);
        assert_eq!(count("approval.escalated"), 2);
        assert_eq!(count("delegation.assigned"), 4);
        assert_eq!(count("worker.hired"), 3);
        assert_eq!(count("cu.action.taken"), 4);
        assert_eq!(count("mcp.tool.call"), 3);
    }

    #[test]
    fn at_least_five_distinct_action_types() {
        let distinct: HashSet<_> = BACKSTORY.iter().map(|r| r.action_type).collect();
        assert!(
            distinct.len() >= 5,
            "expected ≥5 distinct action_type values, got {distinct:?}"
        );
    }

    #[test]
    fn no_metadata_verbs_in_recipe() {
        for spec in BACKSTORY {
            assert!(
                !argo_audit::is_metadata_verb(spec.action_type),
                "{} is a metadata verb — would be hidden from default /audit",
                spec.action_type
            );
            assert_ne!(spec.action_type, "http.request");
        }
    }

    #[test]
    fn approvals_carry_approver_and_escalations_route_to_manager() {
        for spec in BACKSTORY {
            if spec.action_type.starts_with("approval.") {
                assert!(spec.approver_name.is_some(), "{} missing approver", spec.action_type);
            }
            if spec.action_type == "approval.escalated" {
                assert!(matches!(spec.kind, AuditKind::EscalationRequired));
            }
            if spec.action_type == "approval.granted" {
                assert!(matches!(spec.kind, AuditKind::ApprovalGranted));
            }
        }
    }

    #[test]
    fn delegations_populate_delegated_by_and_differ_from_actor() {
        let mut delegations = 0;
        for spec in BACKSTORY {
            if spec.action_type == "delegation.assigned" {
                let by = spec
                    .delegated_by_idx
                    .expect("delegation row missing delegated_by_idx");
                assert_ne!(by, spec.worker_idx, "delegated_by must differ from actor");
                delegations += 1;
            }
        }
        assert_eq!(delegations, 4);
    }

    #[test]
    fn worker_indices_in_range() {
        for spec in BACKSTORY {
            assert!(spec.worker_idx < DEMO_SEED_PERSONAS.len());
            if let Some(by) = spec.delegated_by_idx {
                assert!(by < DEMO_SEED_PERSONAS.len());
            }
        }
    }
}
