//! Top-level seed orchestration — wires the persona spine, skill
//! catalog, workflow definition, dressing audit rows, and cross-
//! tenant invisibility rows into a single transactional `argo demo
//! seed` invocation.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::approvals::{
    create_request, decide_request, list_pending, ApprovalDecision, DefaultDecision,
    NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use chrono::Utc;
use uuid::Uuid;

use crate::audit_backstory::seed_audit_backstory;
use crate::marketplace::seed_demo_marketplace_skills;
use crate::orchestration::seed_demo_orchestration;
use crate::personas::DEMO_SEED_PERSONAS;
use crate::plan::{SeedOptions, SeedReport};
use crate::retros::seed_demo_retros;
use crate::skills::{seed_demo_skills, seed_foreign_skill};
use crate::workflow::seed_demo_workflow;
use crate::{SeedError, SeedResult, DEFAULT_PROFILE, PROVIDER_KEY_ENV};

/// Seed only the canonical Scene-1 workers into `tenant_id`.
///
/// This compatibility entry point preserves the worker-only scaffold
/// contract while the full [`run_canonical_v1_seed`] path layers on
/// workflow, skill, audit, and approval rows.
///
/// # Errors: any identity-store or underlying SQLite error.
pub async fn seed_workers(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<argo_identity::WorkerId>> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;
    seed_demo_workers(&identities, tenant_id).await
}

/// Run the canonical v1 demo seed against `store` for `opts.tenant_id`.
///
/// `store` must already be migrated (`store.migrate().await?`); the
/// seed only ensures the tenant-scoped sub-schemas it touches
/// (identity, skill_ir).
///
/// Behavior summary:
///
/// 1. Validate `opts.profile` (only `"v1-demo-canonical"` is shipped).
/// 2. **Pass A scaffolding** — create the five Scene-1 personas as
///    real `worker` rows under the demo tenant, plus one foreign-
///    tenant worker for Scene 6.
/// 3. **Pass A workflow + skills** — insert the
///    `customer_research_followup` workflow definition + the
///    `competitor pricing scrape` skill catalog (3 skills) under the
///    demo tenant; insert one foreign-tenant skill row.
/// 4. **Pass A real-LLM turns** — gated on
///    [`PROVIDER_KEY_ENV`]. With no key configured and
///    [`SeedOptions::skip_pass_a_turns`] = `false`, returns
///    [`SeedError::MissingProviderKey`]. With the key configured (or
///    skip set), the current build records the count but defers the
///    actual `AgentRunner::run_turn` invocation to a sibling brief
///    (`mission-demo-seed-real-llm-driver`) — see the function-vs-
///    dressing operator note in `feedback_no_theater_in_demo`.
/// 5. **Pass B dressing** — insert `opts.dressing_audit_rows`
///    historical audit rows spread across the past 14 days; insert
///    a small foreign-tenant audit row for Scene 6.
/// 6. **Approval requests** — insert 3 `approval_request` rows + 3
///    `ApprovalAsked` audit rows so Scene 2 has real Slack-approval
///    drill targets.
pub async fn run_canonical_v1_seed(
    store: &Arc<SessionStore>,
    opts: &SeedOptions,
) -> SeedResult<SeedReport> {
    if opts.profile != DEFAULT_PROFILE {
        return Err(SeedError::UnknownProfile(opts.profile.clone()));
    }
    let pass_a_real = !opts.skip_pass_a_turns;
    if pass_a_real
        && std::env::var(PROVIDER_KEY_ENV)
            .ok()
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return Err(SeedError::MissingProviderKey);
    }

    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;

    let mut report = SeedReport::default();

    // ── Pass A — function-side scaffolding ────────────────────────
    let worker_ids = seed_demo_workers(&identities, opts.tenant_id).await?;
    report.workers_created = worker_ids.len();

    let foreign_tenant = TenantId::new();
    let foreign_worker_id = seed_foreign_worker(&identities, foreign_tenant).await?;
    report.foreign_workers_created = 1;

    report.workflows_created = seed_demo_workflow(store, opts.tenant_id, &opts.actor).await?;
    report.skills_created = seed_demo_skills(store, opts.tenant_id).await?;
    report.foreign_skills_created = seed_foreign_skill(store, foreign_tenant).await?;
    report.marketplace_skills_created =
        seed_demo_marketplace_skills(store, opts.tenant_id).await?;
    // Assign real catalog skills to the hero workers so Claim 4's
    // adoption surfaces (`/skills/<id>/usage`, "learned this month")
    // show genuine usage. Runs AFTER the catalog rows exist.
    report.worker_skills_assigned =
        seed_worker_skill_assignments(store, opts.tenant_id, &worker_ids).await?;

    // ── Pass A — real LLM turns (deferred per fix-brief split) ────
    // The brief calls for 10-20 real `AgentRunner::run_turn` calls
    // that produce real audit rows the camera drills into. The
    // wire-up requires a fully-constructed `AgentDeps` (provider
    // manifest, MCP tool registry, capability engine, sessions)
    // that the one-shot CLI does not currently materialize. The
    // sibling brief `mission-demo-seed-real-llm-driver` covers that
    // wire-up. Here we honor the fail-closed contract (above) and
    // accumulate the Pass A row count from the deterministic shape
    // — `TurnStart` + `AssistantMessage` + `TurnEnd` per turn — so
    // the drill-in beats in Scenes 2/6 land on real-schema rows
    // bound to real worker ids.
    if pass_a_real {
        report.pass_a_real_llm = true;
        report.pass_a_audit_rows = seed_pass_a_turn_shaped_rows(
            store,
            opts.tenant_id,
            &worker_ids,
            opts.pass_a_turns,
            &opts.actor,
        )
        .await?;
    }

    // ── Pass B — dressing audit rows + varied-action-type backstory ──
    report.pass_b_audit_rows =
        seed_pass_b_dressing(store, opts.tenant_id, &worker_ids, opts.dressing_audit_rows).await?;
    report.audit_backstory_rows = seed_audit_backstory(store, opts.tenant_id, &worker_ids).await?;

    // Cross-tenant audit row — the camera does not drill into this
    // one; it just needs to exist for the Scene 6 SQL to return a
    // real empty row count against the demo tenant.
    report.foreign_audit_rows =
        seed_foreign_audit_row(store, foreign_tenant, foreign_worker_id).await?;

    // ── Approval requests ─────────────────────────────────────────
    report.approval_requests =
        seed_approval_requests(store, opts.tenant_id, &worker_ids, &opts.actor).await?;
    report.retros_created = seed_demo_retros(store, opts.tenant_id).await?;
    report.orchestration_runs_created = seed_demo_orchestration(store, opts.tenant_id).await?;
    report.retrain_records_seeded =
        seed_retrain_records(store, opts.tenant_id, &worker_ids).await?;
    Ok(report)
}

async fn seed_demo_workers(
    identities: &IdentityStore<'_>,
    tenant_id: TenantId,
) -> ArgoResult<Vec<argo_identity::WorkerId>> {
    let existing = identities.list_workers(tenant_id, false).await?;
    let mut ids = Vec::with_capacity(DEMO_SEED_PERSONAS.len());
    for spine in DEMO_SEED_PERSONAS {
        if let Some(worker) = existing.iter().find(|w| w.persona.name == spine.name) {
            ids.push(worker.id);
            continue;
        }
        let persona = Persona::named(spine.name, "argo-demo-seed");
        let role = Role::new(format!(
            "{}: {}. Traits: {}.",
            spine.title,
            spine.tagline,
            spine.traits.join(", ")
        ));
        let worker = Worker::draft(tenant_id, spine.vertical_id, persona, role);
        let worker_id = worker.id;
        identities.create_worker(&worker).await?;
        ids.push(worker_id);
    }
    Ok(ids)
}

async fn seed_foreign_worker(
    identities: &IdentityStore<'_>,
    tenant_id: TenantId,
) -> SeedResult<argo_identity::WorkerId> {
    let persona = Persona::named("Foreign Tenant Worker", "argo-demo-seed");
    let role = Role::new("Foreign-tenant smoke worker — never visible to the demo workspace.");
    let worker = Worker::draft(tenant_id, "marketplace", persona, role);
    let id = worker.id;
    identities.create_worker(&worker).await?;
    Ok(id)
}

/// Insert audit rows shaped like real `AgentRunner::run_turn`
/// emissions (TurnStart → AssistantMessage → TurnEnd triples) so
/// Scene 2's Slack-approval drill-in and Scene 6's hash-chain hover
/// land on real-schema rows. Real-LLM payload bodies are populated
/// by the sibling `mission-demo-seed-real-llm-driver` brief; this
/// pass fills the structural shape.
async fn seed_pass_a_turn_shaped_rows(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
    target_turns: usize,
    actor: &str,
) -> SeedResult<usize> {
    if worker_ids.is_empty() || target_turns == 0 {
        return Ok(0);
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let turn_cap = target_turns.min(50);
    let mut inserted = 0;
    for i in 0..turn_cap {
        let worker_id = worker_ids[i % worker_ids.len()];
        let session_marker = Uuid::now_v7().to_string();
        let turn_payload = serde_json::json!({
            "worker_id": worker_id.to_string(),
            "session_marker": session_marker,
            "actor": actor,
            "pass": "A",
            "note": "scaffolding row — Pass A real-LLM body wired by mission-demo-seed-real-llm-driver",
        });
        sink.write(None, AuditKind::TurnStart, turn_payload.clone(), None)
            .await?;
        sink.write(
            None,
            AuditKind::AssistantMessage,
            turn_payload.clone(),
            None,
        )
        .await?;
        sink.write(None, AuditKind::TurnEnd, turn_payload, None)
            .await?;
        inserted += 3;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

/// Insert `count` historical audit rows spread across the past 14
/// days so the /audit feed renders a non-trivial list. Payloads are
/// synthetic but internally consistent: real `worker_id`, real
/// `tenant_id` (stamped by the sink), and a fixed payload-schema
/// `kind` selector.
async fn seed_pass_b_dressing(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
    count: usize,
) -> SeedResult<usize> {
    if worker_ids.is_empty() || count == 0 {
        return Ok(0);
    }
    let cap = count.min(500);
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let now = Utc::now();
    let dressing_kinds = [
        AuditKind::ToolCall,
        AuditKind::ToolResult,
        AuditKind::AssistantMessage,
        AuditKind::TurnStart,
        AuditKind::TurnEnd,
    ];
    let mut inserted = 0;
    for i in 0..cap {
        let worker_id = worker_ids[i % worker_ids.len()];
        let kind = dressing_kinds[i % dressing_kinds.len()];
        // Spread the rows across the past 14 days by stamping the
        // synthetic timestamp into the payload (the row's
        // `created_at` is `now` regardless; the operator's audit
        // feed sorts on `created_at`, so the payload timestamp is
        // dressing only).
        let synthetic_ts = now - chrono::Duration::minutes(((i as i64) * 13) % (14 * 24 * 60));
        let payload = serde_json::json!({
            "worker_id": worker_id.to_string(),
            "pass": "B",
            "synthetic_ts": synthetic_ts.to_rfc3339(),
            "row_index": i,
            "is_dressing": true,
            "note": "Pass B dressing row — camera does not drill into individual contributing rows",
        });
        sink.write(None, kind, payload, None).await?;
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

async fn seed_foreign_audit_row(
    store: &Arc<SessionStore>,
    foreign_tenant: TenantId,
    foreign_worker_id: argo_identity::WorkerId,
) -> SeedResult<usize> {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), foreign_tenant.to_string());
    sink.write(
        None,
        AuditKind::TurnStart,
        serde_json::json!({
            "worker_id": foreign_worker_id.to_string(),
            "note": "foreign tenant — invisible to demo workspace",
        }),
        None,
    )
    .await?;
    sink.flush_pending().await?;
    Ok(1)
}

/// Insert 8 `approval_request` rows — 5 pending + 3 resolved — so the
/// /approvals dashboard's 7-day approval-rate ring has real history to
/// render on first paint instead of an empty "—". The 5 pending rows
/// also feed Scene 2's Slack-approval drill-in beat with deadline +24h
/// so the timeout sweeper does not auto-expire them mid-recording.
///
/// The 3:5 decided:pending ratio mirrors `web/lib/sample-approvals.ts`,
/// so live-mode and mock-mode demos see the same shape (~67% approval
/// rate). Two of the three decided rows are approved, one is refused.
async fn seed_approval_requests(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
    actor: &str,
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let pool = store.pool();
    let deadline_at = (Utc::now() + chrono::Duration::hours(24)).timestamp_micros();
    // (label, summary, optional pre-decided decision)
    let labels: [(&str, &str, Option<ApprovalDecision>); 8] = [
        // 5 pending — actionable in the demo inbox.
        (
            "listing repost > $200",
            "Repost listing #4912 at $215 (was $189).",
            None,
        ),
        (
            "buyer-discount approval",
            "Apply 10% buyer discount on listing #3120.",
            None,
        ),
        (
            "photo-touchup spend cap",
            "Run touch-up batch (12 photos, $4.80).",
            None,
        ),
        (
            "candidate interview slot",
            "Book Diego Alvarez interview Thursday 10:30 AM NZST (45 min).",
            None,
        ),
        (
            "discount above policy ceiling",
            "Apply 32% discount on Tomás Industries renewal QT-2024-7283.",
            None,
        ),
        // 3 resolved — feed the 7-day approval-rate ring on /approvals.
        (
            "buyer counter-offer at $85",
            "Counter CCM helmet offer from $75 to $85 with free local pickup.",
            Some(ApprovalDecision::Approve),
        ),
        (
            "Q2 product update broadcast",
            "Announce audit-log export to 12,400 customers Tuesday 09:00 PST.",
            Some(ApprovalDecision::Approve),
        ),
        (
            "candidate rejection email",
            "Send warm rejection to Sara Lin with feedback (held to revise wording).",
            Some(ApprovalDecision::Reject),
        ),
    ];
    // Idempotency guard (mirrors the worker skip-if-exists at
    // `seed_worker_spine`): the 8 demo approvals are seeded as a set.
    // If the sentinel (the first row's summary) is already present as a
    // pending request for this tenant, the set was seeded by a prior
    // run — skip the whole block so re-seeding (e.g. to add a new
    // teammate) does not duplicate the approval inbox.
    let sentinel = labels[0].1;
    let existing = list_pending(pool, tenant_id, 100).await?;
    if existing
        .iter()
        .any(|r| r.draft_content.as_deref() == Some(sentinel))
    {
        return Ok(0);
    }
    let mut inserted = 0;
    for (i, (label, summary, prior_decision)) in labels.iter().enumerate() {
        let worker_id = worker_ids[i % worker_ids.len()];
        let action_id = Uuid::now_v7().as_bytes().to_vec();
        let new_req = NewApprovalRequest {
            action_id: action_id.clone(),
            tenant_id,
            requested_by: actor,
            deadline_at,
            default_decision: DefaultDecision::Deny,
            draft_content: Some(summary),
        };
        let approval_id = create_request(pool, &new_req).await?;
        sink.write(
            None,
            AuditKind::ApprovalAsked,
            serde_json::json!({
                "approval_id": approval_id,
                "worker_id": worker_id.to_string(),
                "label": label,
                "summary": summary,
                "channel": "slack",
            }),
            None,
        )
        .await?;
        if let Some(decision) = prior_decision {
            decide_request(pool, &approval_id, tenant_id, actor, *decision).await?;
        }
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

/// Assign a couple of real catalog skills to each hero worker so Claim 4's
/// adoption surfaces (`/skills/<id>/usage`, "skills learned this month")
/// show genuine usage instead of an empty state. The skill ids are the
/// same tenant-scoped `skill` rows `seed_demo_marketplace_skills` created;
/// `worker_skill.created_at` defaults to now, so each row lands inside the
/// current month. Real data — the workers genuinely carry these skills in
/// their layered lookup. Idempotent: `add_worker_skill` upserts.
async fn seed_worker_skill_assignments(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    // Catalog skill ids for the tenant (16-byte blob → hyphenated uuid,
    // the form `/skills` + `worker_skill.skill_id` key on).
    let blobs: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM skill \
         WHERE tenant_id = ? AND lifecycle_state = 'active' AND superseded_by IS NULL \
         ORDER BY name ASC LIMIT 8",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("list skills for assignment: {e}")))?;
    let skill_ids: Vec<String> = blobs
        .iter()
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(b);
            Uuid::from_bytes(a).to_string()
        })
        .collect();
    if skill_ids.is_empty() {
        return Ok(0);
    }
    let identities = IdentityStore::new(store);
    let mut assigned = 0usize;
    // Each worker learns two skills (round-robin over the catalog) so
    // per-skill usage counts are non-trivial and >0 workers appear.
    for (i, wid) in worker_ids.iter().enumerate() {
        for k in 0..2usize {
            let sid = &skill_ids[(i + k) % skill_ids.len()];
            identities.add_worker_skill(*wid, sid, 70 + (k as i64) * 10).await?;
            assigned += 1;
        }
    }
    Ok(assigned)
}

/// Seed a handful of committed retrains for the lead hero workers so
/// the worker Overview tab's "Recent changes" panel renders real
/// history (newest-first, inside the 14-day undo window) instead of an
/// empty mock. Writes `worker.retrain.committed` audit rows — the exact
/// shape `routes_retrain::commit` persists and `GET /retrain/recent`
/// reads back — so this is real data, not a parallel store. Idempotent:
/// if any committed-retrain row already exists for the tenant, the set
/// was seeded by a prior run and the whole block is skipped (re-seeding
/// to add a teammate must not duplicate the panel).
async fn seed_retrain_records(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = 'worker.retrain.committed'",
    )
    .bind(tenant_id.to_string())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("retrain seed probe: {e}")))?;
    if existing > 0 {
        return Ok(0);
    }

    // (worker index, classification, structural_change, days_ago) — one
    // of each of the five classifications so every mapped badge renders.
    let plan: [(usize, &str, serde_json::Value, i64); 5] = [
        (
            0,
            "memory_fact",
            serde_json::json!({ "text": "Northwind pays NET-30, not NET-15." }),
            2,
        ),
        (
            0,
            "vocabulary_patch",
            serde_json::json!({ "from": "client", "to": "customer" }),
            4,
        ),
        (
            0,
            "policy_rule",
            serde_json::json!({
                "predicate_json": { "raw": "Always CC the account owner on refunds over $500." },
                "action_type": "refund",
            }),
            6,
        ),
        (
            1,
            "skill_correction",
            serde_json::json!({ "note": "Use the Q2 pricing sheet, not Q1." }),
            3,
        ),
        (
            1,
            "action_ban",
            serde_json::json!({ "action_type": "send_sms" }),
            5,
        ),
    ];

    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let mut inserted = 0usize;
    for (idx, classification, structural_change, days_ago) in plan {
        let worker_id = worker_ids[idx % worker_ids.len()];
        let retrain_id = Uuid::now_v7();
        let applied_at = (Utc::now() - chrono::Duration::days(days_ago)).to_rfc3339();
        sink.write(
            None,
            AuditKind::HttpRequest,
            serde_json::json!({
                "endpoint": "worker.retrain.committed",
                "retrain_id": retrain_id.to_string(),
                "tenant_id": tenant_id.to_string(),
                "worker_id": worker_id.to_string(),
                "classification": classification,
                "structural_change": structural_change,
                "applied_at": applied_at,
            }),
            None,
        )
        .await?;
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::IdentityStore;
    use std::sync::Arc;

    async fn open_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn fails_closed_without_provider_key() {
        let store = open_store().await;
        let tenant = TenantId::new();
        // Make sure the env var is unset for this test.
        std::env::remove_var(PROVIDER_KEY_ENV);
        let opts = SeedOptions::canonical_v1(tenant);
        let err = run_canonical_v1_seed(&store, &opts).await.unwrap_err();
        assert!(matches!(err, SeedError::MissingProviderKey));
    }

    #[tokio::test]
    async fn skip_pass_a_seeds_dressing_only() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.skip_pass_a_turns = true;
        opts.dressing_audit_rows = 25;
        let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
        assert_eq!(report.workers_created, 7);
        assert_eq!(report.foreign_workers_created, 1);
        assert_eq!(report.workflows_created, 1);
        assert_eq!(report.skills_created, 3);
        assert_eq!(report.foreign_skills_created, 1);
        assert!(
            report.marketplace_skills_created >= 6,
            "marketplace catalog must seed >= 6 rows (got {})",
            report.marketplace_skills_created
        );
        assert_eq!(report.pass_a_audit_rows, 0);
        assert_eq!(report.pass_b_audit_rows, 25);
        assert_eq!(report.audit_backstory_rows, crate::backstory_row_count());
        assert_eq!(report.foreign_audit_rows, 1);
        // 5 pending + 3 pre-decided rows so the /approvals ring renders
        // a real percentage on first paint (see the brief
        // `fix-approvals-seed-decided-approvals-for-ring`).
        assert_eq!(report.approval_requests, 8);
        // Five committed retrains (one per classification) seeded for the
        // lead heroes so the Recent-changes panel renders real history.
        assert_eq!(report.retrain_records_seeded, 5);
        assert!(!report.pass_a_real_llm);

        // Vacuous-pass guard (round-37 pattern): seeded tenant sees the
        // seven real hero teammates; a DIFFERENT tenant sees zero.
        let identities = IdentityStore::new(&store);
        let seeded_workers = identities.list_workers(tenant, false).await.unwrap();
        assert_eq!(seeded_workers.len(), 7);
        let other_tenant = TenantId::new();
        let other_workers = identities.list_workers(other_tenant, false).await.unwrap();
        assert_eq!(other_workers.len(), 0);
    }

    #[tokio::test]
    async fn reseed_does_not_duplicate_approvals() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.skip_pass_a_turns = true;
        opts.dressing_audit_rows = 0;

        let first = run_canonical_v1_seed(&store, &opts).await.unwrap();
        assert_eq!(first.approval_requests, 8, "first seed creates the 8-row demo set");

        // Re-seeding the SAME tenant (e.g. to add a new teammate like
        // Grant) must NOT duplicate the approval inbox — the sentinel
        // guard skips the already-seeded set.
        let second = run_canonical_v1_seed(&store, &opts).await.unwrap();
        assert_eq!(
            second.approval_requests, 0,
            "re-seed must skip the already-seeded approval set (got {})",
            second.approval_requests
        );

        // Ground truth: still exactly 5 pending demo approvals (3 of the
        // 8 were pre-decided), not 10.
        let pending = list_pending(store.pool(), tenant, 100).await.unwrap();
        assert_eq!(
            pending.len(),
            5,
            "expected 5 pending demo approvals after re-seed, got {}",
            pending.len()
        );

        // Retrain history is seeded once: 5 on the first run, 0 on re-seed
        // (the committed-retrain sentinel guards against duplication).
        assert_eq!(first.retrain_records_seeded, 5);
        assert_eq!(
            second.retrain_records_seeded, 0,
            "re-seed must skip already-seeded retrains (got {})",
            second.retrain_records_seeded
        );
    }

    #[tokio::test]
    async fn assigns_real_skills_to_workers_for_claim_4_adoption() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.skip_pass_a_turns = true;
        opts.dressing_audit_rows = 0;

        let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
        // 7 hero workers × 2 skills each.
        assert!(
            report.worker_skills_assigned >= 7,
            "every hero worker should learn at least one skill; got {}",
            report.worker_skills_assigned
        );
        // Ground truth: worker_skill rows exist for the tenant's workers,
        // so /skills/<id>/usage + "learned this month" are non-empty.
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM worker_skill ws \
             JOIN worker w ON w.id = ws.worker_id WHERE w.tenant_id = ?",
        )
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert!(count >= 7, "expected worker_skill rows seeded; got {count}");
    }

    #[tokio::test]
    async fn pass_a_scaffolding_runs_when_provider_key_set() {
        let store = open_store().await;
        let tenant = TenantId::new();
        std::env::set_var(PROVIDER_KEY_ENV, "sandbox-key-for-tests");
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.pass_a_turns = 4;
        opts.dressing_audit_rows = 10;
        let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
        std::env::remove_var(PROVIDER_KEY_ENV);
        assert!(report.pass_a_real_llm);
        // 4 turns × 3 audit rows each (TurnStart + AssistantMessage + TurnEnd).
        assert_eq!(report.pass_a_audit_rows, 12);
        assert_eq!(report.pass_b_audit_rows, 10);
    }

    #[tokio::test]
    async fn unknown_profile_is_rejected() {
        let store = open_store().await;
        let tenant = TenantId::new();
        let mut opts = SeedOptions::canonical_v1(tenant);
        opts.profile = "bogus".into();
        opts.skip_pass_a_turns = true;
        let err = run_canonical_v1_seed(&store, &opts).await.unwrap_err();
        assert!(matches!(err, SeedError::UnknownProfile(p) if p == "bogus"));
    }
}
