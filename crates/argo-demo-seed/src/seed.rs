//! Top-level seed orchestration — wires the persona spine, skill
//! catalog, workflow definition, dressing audit rows, and cross-
//! tenant invisibility rows into a single transactional `argo demo
//! seed` invocation.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::approvals::{
    create_request, decide_request, list_pending, DefaultDecision, NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use chrono::Utc;
use uuid::Uuid;

use crate::audit_backstory::seed_audit_backstory;
use crate::marketplace::seed_demo_marketplace_skills;
use crate::orchestration::seed_demo_orchestration;
use crate::personas::DEMO_SEED_PERSONAS;
use crate::seed_approvals::{demo_approval_rows, worker_id_for_persona};
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

    // Record-readiness: prune the demo tenant to EXACTLY the canonical
    // roster (archive stray workers like Genesis-hired "Maya"). DEMO PATH
    // ONLY, canonical-protected, tenant-scoped, reversible — see seed_prune.
    let canonical_names: Vec<&str> = DEMO_SEED_PERSONAS.iter().map(|p| p.name).collect();
    report.workers_pruned =
        crate::seed_prune::prune_demo_tenant_to_canonical(store, opts.tenant_id, &canonical_names)
            .await?;

    let foreign_tenant = TenantId::new();
    let foreign_worker_id = seed_foreign_worker(&identities, foreign_tenant).await?;
    report.foreign_workers_created = 1;

    report.workflows_created = seed_demo_workflow(store, opts.tenant_id, &opts.actor).await?;
    report.skills_created = seed_demo_skills(store, opts.tenant_id).await?;
    report.foreign_skills_created = seed_foreign_skill(store, foreign_tenant).await?;
    report.marketplace_skills_created =
        seed_demo_marketplace_skills(store, opts.tenant_id).await?;
    // Assign real catalog skills to the hero workers (Claim 4 adoption:
    // `/skills/<id>/usage` + "learned this month"). After catalog rows.
    report.worker_skills_assigned =
        crate::seed_worker_overlays::seed_worker_skill_assignments(
            store,
            opts.tenant_id,
            &worker_ids,
        )
        .await?;
    // Real `role_duty` rows for the hero workers so each worker
    // overview's Responsibilities / Schedules card renders genuine
    // duties (Scene 2 lands on Mariah's four). Positional-aligned with
    // DEMO_SEED_PERSONAS, the same spine `seed_demo_workers` walked.
    report.worker_duties_seeded = crate::seed_worker_duties::seed_worker_duties(
        store,
        opts.tenant_id,
        DEMO_SEED_PERSONAS,
        &worker_ids,
    )
    .await?;

    // ── Pass A — real LLM turns (deferred per fix-brief split) ────
    // Real `AgentRunner::run_turn` wire-up needs a full `AgentDeps` the
    // one-shot CLI does not materialize (sibling brief
    // `mission-demo-seed-real-llm-driver`). Here we honor the fail-closed
    // contract and seed the deterministic `TurnStart`/`AssistantMessage`/
    // `TurnEnd` shape so Scenes 2/6 drill into real-schema rows.
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

    // Cross-tenant audit row — exists so Scene 6's SQL returns a real
    // empty count against the demo tenant (camera doesn't drill in).
    report.foreign_audit_rows =
        seed_foreign_audit_row(store, foreign_tenant, foreign_worker_id).await?;

    // ── Approval requests ─────────────────────────────────────────
    report.approval_requests =
        seed_approval_requests(store, opts.tenant_id, &worker_ids, &opts.actor).await?;
    report.retros_created = seed_demo_retros(store, opts.tenant_id).await?;
    report.orchestration_runs_created = seed_demo_orchestration(store, opts.tenant_id).await?;
    report.retrain_records_seeded =
        crate::seed_worker_overlays::seed_retrain_records(store, opts.tenant_id, &worker_ids)
            .await?;
    // Skill IR + reeval history — keyed on the marketplace catalog ids,
    // so they run AFTER the catalog + worker-skill assignments. Detail:
    // crate::skill_ir_eval.
    report.skill_ir_seeded = crate::seed_skill_ir(store, opts.tenant_id).await?;
    report.skill_reeval_rows_seeded =
        crate::seed_skill_reeval_history(store, opts.tenant_id).await?;
    // Clean v1→v2→v3 version chain per catalog skill (active head
    // relabelled IN PLACE to v3 + two superseded history rows behind it)
    // so the /skills "Version History" panel reads a coherent progression
    // aligned with the v1/v2/v3 Eval Re-run History above it — not a lone
    // 0.1.0 row that joins no eval runs. Runs AFTER reeval so the version
    // labels match the seeded `skill_reeval_history.skill_version` values;
    // the head id is preserved so worker_skill / skill_ir stay valid.
    report.skill_version_chains_built =
        crate::seed_skill_version_chain(store, opts.tenant_id).await?;

    // ── Seed-enrichment: REAL-but-empty surfaces ──────────────────
    // Closes NO-THEATER gaps where a real, correctly-gated endpoint
    // exists but the seed left it empty. Runs last so every referenced
    // entity (approvals, workers, workflow, skills) already exists.
    seed_enrichment_surfaces(store, opts.tenant_id, &worker_ids, &mut report).await?;

    Ok(report)
}

/// Seed the REAL-but-empty surfaces — bell notifications, workflow run
/// history, knowledge handbook, trust grades, API-key roster, and worker
/// memory (facts + conversation threads) — onto `report`. Each writes real
/// rows the live endpoint serves; idempotent.
async fn seed_enrichment_surfaces(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
    report: &mut SeedReport,
) -> SeedResult<()> {
    // Broadcast bell notifications so the bell shows real rows.
    report.notifications_seeded =
        crate::seed_notifications::seed_notifications(store, tenant_id).await?;

    // Run history for the Scene-5 workflow (`workflow_run` is NOT
    // tenant-scoped, mig 18; rows match the `{vertical}/{name}` key).
    report.workflow_runs_seeded = crate::seed_workflow_runs::seed_workflow_runs(store).await?;

    // Workplace-knowledge handbook docs so /settings/knowledge lists pages.
    report.knowledge_docs_seeded =
        crate::seed_knowledge::seed_knowledge(store, tenant_id).await?;

    // Honest per-worker trust grades so the Trust tab shows a real grade
    // (positional-aligned with DEMO_SEED_PERSONAS).
    report.trust_grades_set =
        crate::seed_trust_grades::seed_trust_grades(store, tenant_id, DEMO_SEED_PERSONAS, worker_ids)
            .await?;

    // Metadata-only API-key roster (throwaway hashes, no plaintext
    // secret) so /settings/api-keys shows real cards (Frontier-B SPEC 2).
    report.api_keys_seeded = crate::seed_api_keys::seed_api_keys(store, tenant_id).await?;

    // Real worker-memory facts (live append path) + open conversation
    // threads (v87 store) so the Memory tab renders real persona-tied data.
    crate::seed_worker_memory::seed_worker_memory(store, tenant_id, worker_ids, report).await?;

    Ok(())
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

/// Insert 14 `approval_request` rows — 5 pending + 9 resolved — so the
/// /approvals dashboard's 7-day approval-rate ring has real history to
/// render on first paint instead of an empty "—". The 5 pending rows
/// also feed Scene 2's Slack-approval drill-in beat with deadline +24h
/// so the timeout sweeper does not auto-expire them mid-recording.
///
/// Each row names its **owner persona** so the `approval.asked` audit
/// row carries that worker's id — and the Memory tab's "Recent
/// decisions" (which filters by `worker_id`) shows only role-appropriate
/// decisions. The prior blind `worker_ids[i % len]` round-robin
/// mis-attributed rows (e.g. a *recruiting* "candidate rejection email"
/// landed on Marcus in Marketing); owner-by-name fixes that.
///
/// Variety + a trusted-AI ratio: the banner promises "refunds, hires,
/// contracts," so the set spans a refund (Jamal/Support), a hire
/// (Priya/Recruiting), a contract (Grant/GTM), marketing drafts
/// (Marcus), and marketplace/comms actions — not an all-marketing queue.
/// Of the decided rows, the approve/refuse split targets a healthy
/// ~70-85% approval rate (the refusals are the exceptions that prove the
/// human-oversight loop works), so the /approvals rate ring reads as a
/// trusted workforce rather than a 24%-distrust signal.
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
    let rows = demo_approval_rows();
    // Idempotency guard (mirrors the worker skip-if-exists at
    // `seed_worker_spine`): the demo approvals are seeded as a set. If the
    // sentinel (the first row's summary) is already present as a pending
    // request for this tenant, the set was seeded by a prior run — skip
    // the whole block so re-seeding (e.g. to add a new teammate) does not
    // duplicate the approval inbox.
    let sentinel = rows[0].summary;
    let existing = list_pending(pool, tenant_id, 100).await?;
    if existing
        .iter()
        .any(|r| r.draft_content.as_deref() == Some(sentinel))
    {
        return Ok(0);
    }
    let mut inserted = 0;
    for row in &rows {
        // Resolve the owning worker by persona name; fall back to the
        // positional id only if the name is somehow absent from the spine
        // (it never is for the canonical roster).
        let worker_id = worker_id_for_persona(row.owner, worker_ids).unwrap_or(worker_ids[0]);
        let action_id = Uuid::now_v7().as_bytes().to_vec();
        let new_req = NewApprovalRequest {
            action_id: action_id.clone(),
            tenant_id,
            requested_by: row.owner,
            deadline_at,
            default_decision: DefaultDecision::Deny,
            draft_content: Some(row.summary),
        };
        let approval_id = create_request(pool, &new_req).await?;
        sink.write(
            None,
            AuditKind::ApprovalAsked,
            serde_json::json!({
                "approval_id": approval_id,
                "worker_id": worker_id.to_string(),
                "label": row.label,
                "summary": row.summary,
                "channel": "slack",
            }),
            None,
        )
        .await?;
        if let Some(decision) = row.decision {
            decide_request(pool, &approval_id, tenant_id, actor, decision).await?;
        }
        inserted += 1;
    }
    sink.flush_pending().await?;
    Ok(inserted)
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
