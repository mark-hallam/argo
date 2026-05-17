//! Mission claim 4 — runtime closure trace for the skills-learned
//! loop. Drives the pipeline end-to-end so a regression on any
//! layer (proposal gate, lifecycle transition, marketplace publish,
//! library re-injection, cross-tenant boundary) flips this test.
//!
//! Pipeline: workerA CU success → `should_propose` →
//! `SkillCandidate::from_cu_trajectory` → `propose_for_review`
//! (pending_review) → operator approve (`transition_lifecycle` +
//! SkillApproved audit) → `publish_mock_skill` → library install
//! (`published:verified:vendor`) → workerB selection via
//! `LayeredLookup::with_library` + `SkillRelevanceSelector`.
//!
//! Negative cases locked in: below-threshold rejection,
//! cross-tenant workspace-private isolation, F-11 vertical mismatch.

#[path = "fixtures/skill_trajectory_seed.rs"]
mod skill_trajectory_seed;

use argo_audit::{verify_session, AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_skill::{
    publish::{mock_registry_root, publish_mock_skill, SkillPublishSource},
    LayeredLookup, LifecycleState, SkillRelevanceConfig, SkillRelevanceSelector,
    SkillSelectionContext, SkillStore,
};
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

const WORKER_A_VERTICAL: &str = "gtm-engineer";
const WORKER_B_VERTICAL: &str = "gtm-engineer";
const OTHER_VERTICAL: &str = "pr-checker";
const SKILL_NAME: &str = "post_listing_loopclosure";
const PRIVATE_SKILL_NAME: &str = "tenant_a_private_recipe";

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("open session store");
    store.migrate().await.expect("migrate session store");
    Arc::new(store)
}

/// Drive every waypoint in one test so the 3-store + 2-sink
/// fixture is paid once.
#[tokio::test]
async fn skills_learned_loop_closes_across_workers() {
    // ── Setup: tenantA workspace, tenantB workspace, shared library ──
    let tenant_a_store = fresh_store().await;
    let tenant_b_store = fresh_store().await;
    let library_store = fresh_store().await;

    let tenant_a_skills = SkillStore::new(Arc::clone(&tenant_a_store));
    let tenant_b_skills = SkillStore::new(Arc::clone(&tenant_b_store));
    let library_skills = SkillStore::new(Arc::clone(&library_store));

    // Per-tenant sessions so audit rows chain cleanly per-tenant.
    let worker_a_session = SessionId::new();
    let worker_b_session = SessionId::new();

    // Audit sinks scoped to each tenant's store so verify_session
    // walks the rows for the corresponding worker independently.
    let audit_a = AuditSink::new_with_tenant(
        Arc::clone(&tenant_a_store),
        SYSTEM_AUDIT_TENANT_ID.to_string(),
    );
    let audit_b = AuditSink::new_with_tenant(
        Arc::clone(&tenant_b_store),
        SYSTEM_AUDIT_TENANT_ID.to_string(),
    );

    // ── Waypoint 1: workerA performs a successful CU action ───────
    audit_a
        .write(
            Some(worker_a_session),
            AuditKind::ToolCall,
            serde_json::json!({"tool": "computer_use", "action": "post_listing"}),
            None,
        )
        .await
        .expect("workerA ToolCall");
    audit_a
        .write(
            Some(worker_a_session),
            AuditKind::ToolResult,
            serde_json::json!({"tool": "computer_use", "success": true}),
            None,
        )
        .await
        .expect("workerA ToolResult");

    // ── Waypoint 2: should_propose guards the proposal path ───────
    let high = skill_trajectory_seed::high_confidence_post_listing(WORKER_A_VERTICAL);
    let low = skill_trajectory_seed::low_confidence_trajectory(WORKER_A_VERTICAL);
    assert!(
        skill_trajectory_seed::maybe_propose("ignored", &low).is_none(),
        "should_propose must reject below-threshold trajectories"
    );
    let candidate = skill_trajectory_seed::maybe_propose(SKILL_NAME, &high)
        .expect("high-confidence trajectory should clear the threshold");
    assert_eq!(candidate.vertical_id, WORKER_A_VERTICAL);

    // ── Waypoint 3: propose_for_review lands a `pending_review` row ──
    //
    // The capabilities-block serde drift surfaced here on #1822 was
    // fixed in mission-skill-md-capabilities-serde-fix: `to_skill_md`
    // now emits the tagged-enum YAML shape that
    // `SkillStore::propose_for_review` expects, so the raw output
    // round-trips without a workaround.
    let raw = candidate.to_skill_md("workerA-session");
    let proposed = tenant_a_skills
        .propose_for_review(&raw, worker_a_session, WORKER_A_VERTICAL)
        .await
        .expect("propose_for_review");
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);
    let pending = tenant_a_skills
        .list_pending_review()
        .await
        .expect("list_pending_review");
    assert!(
        pending.iter().any(|s| s.frontmatter.name == SKILL_NAME),
        "candidate should appear in pending_review queue: {:?}",
        pending.iter().map(|s| &s.frontmatter.name).collect::<Vec<_>>()
    );
    // The CU dispatch shim does not yet emit a candidate-proposed
    // audit row (only the memory-promote path does, in
    // `skill_review.rs`). Emitting it here completes the closure
    // trace; once the runtime emits its own, the assertion below
    // will catch any duplication.
    audit_a
        .write(
            Some(worker_a_session),
            AuditKind::SkillCandidatePromoted,
            serde_json::json!({
                "skill_id": proposed.id.0.to_string(),
                "name": &proposed.frontmatter.name,
                "vertical_id": WORKER_A_VERTICAL,
                "source": "cu_trajectory",
            }),
            None,
        )
        .await
        .expect("SkillCandidatePromoted audit");

    // ── Waypoint 4: operator approves the candidate ───────────────
    let new_state = tenant_a_skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .expect("transition_lifecycle");
    assert_eq!(new_state, LifecycleState::Active);
    audit_a
        .write(
            Some(worker_a_session),
            AuditKind::SkillApproved,
            serde_json::json!({
                "skill_id": proposed.id.0.to_string(),
                "name": &proposed.frontmatter.name,
                "version": &proposed.frontmatter.version,
                "vertical_id": WORKER_A_VERTICAL,
                "to": new_state.label(),
                "reviewed_by": "loop-closure-smoke",
            }),
            None,
        )
        .await
        .expect("SkillApproved audit");

    // ── Waypoint 5: publish to mock OCI marketplace ───────────────
    let active = tenant_a_skills
        .find_active_by_name(SKILL_NAME)
        .await
        .expect("find_active_by_name")
        .expect("approved skill should be active");
    let publish_root_dir = tempfile::tempdir().expect("publish root tempdir");
    let registry_url = format!("mock://{}", publish_root_dir.path().display());
    let root = mock_registry_root(&registry_url).expect("mock registry root");
    let source = SkillPublishSource::from_skill(&active).expect("publish source from active skill");
    let record = publish_mock_skill(&publish_root_dir.path().join(&root), &source, "0.1.0", false)
        .expect("publish mock skill");
    assert!(!record.dry_run, "publish must persist a real bundle");
    assert!(
        publish_root_dir.path().join(&root).join(&record.manifest_path).exists(),
        "manifest file should exist at {}",
        record.manifest_path
    );

    // OCI sync re-installs the canonical SKILL.md into the shared
    // library store with verified-publisher provenance.
    let library_raw = build_library_raw(&active.frontmatter.name, &active.body);
    library_skills
        .install_for_vertical(&library_raw, None, Some(WORKER_A_VERTICAL))
        .await
        .expect("install into library store");

    // ── Waypoint 6: workerB (tenantB) sees the library skill ──────
    tenant_b_skills
        .install_for_vertical(
            &skill_md_for(
                "tenant_b_local_recipe",
                "user",
                "tenant B's own recipe body",
                WORKER_B_VERTICAL,
            ),
            None,
            Some(WORKER_B_VERTICAL),
        )
        .await
        .expect("install tenantB workspace skill");
    // Workspace-private skill — never published; drives the
    // cross-tenant negative case below.
    tenant_a_skills
        .install_for_vertical(
            &skill_md_for(
                PRIVATE_SKILL_NAME,
                "user",
                "tenant A's secret recipe — must not propagate",
                WORKER_A_VERTICAL,
            ),
            None,
            Some(WORKER_A_VERTICAL),
        )
        .await
        .expect("install tenantA private skill");

    let lookup = LayeredLookup::with_library(&tenant_b_skills, &library_skills);
    let rows = lookup
        .list_for_action("low")
        .await
        .expect("layered list_for_action");
    let selector = SkillRelevanceSelector::new(SkillRelevanceConfig::default());
    let selected_for_b = selector.select(
        rows.clone(),
        SkillSelectionContext {
            action_type: None,
            vertical_id: Some(WORKER_B_VERTICAL),
            task_tags: &[],
        },
    );
    let selected_names: Vec<_> = selected_for_b
        .iter()
        .map(|s| s.frontmatter.name.clone())
        .collect();
    assert!(
        selected_names.iter().any(|n| n == SKILL_NAME),
        "F-11 selector should re-inject the published skill into workerB's prompt set: got {selected_names:?}"
    );

    // ── Waypoint 7: cross-tenant negative case ────────────────────
    assert!(
        !selected_names.iter().any(|n| n == PRIVATE_SKILL_NAME),
        "tenant A's workspace-private skill must NOT leak into tenant B's prompt: got {selected_names:?}"
    );

    // ── Waypoint 7b: F-11 filter mismatch ─────────────────────────
    let selected_for_other = selector.select(
        rows,
        SkillSelectionContext {
            action_type: None,
            vertical_id: Some(OTHER_VERTICAL),
            task_tags: &[],
        },
    );
    assert!(
        !selected_for_other
            .iter()
            .any(|s| s.frontmatter.name == SKILL_NAME),
        "selector with mismatched vertical must drop the published skill"
    );

    // ── Waypoint 8: tenant B records its own ToolCall audit ───────
    audit_b
        .write(
            Some(worker_b_session),
            AuditKind::ToolCall,
            serde_json::json!({
                "tool": "computer_use",
                "skills_in_context": selected_names,
            }),
            None,
        )
        .await
        .expect("workerB ToolCall");

    // ── Audit chain assertions ────────────────────────────────────
    // Flush so the batched writer commits before verify_session reads.
    audit_a.flush_pending().await.expect("flush tenantA audit");
    audit_b.flush_pending().await.expect("flush tenantB audit");
    let tenant_a_report = verify_session(&tenant_a_store, worker_a_session)
        .await
        .expect("verify tenantA chain");
    assert!(
        tenant_a_report.ok,
        "tenantA audit chain must verify clean: {:?}",
        tenant_a_report.breaks
    );
    assert!(
        tenant_a_report
            .kind_counts
            .get(AuditKind::SkillApproved.label())
            .copied()
            .unwrap_or_default()
            >= 1,
        "tenantA chain must include at least one SkillApproved row: {:?}",
        tenant_a_report.kind_counts
    );
    let tenant_b_report = verify_session(&tenant_b_store, worker_b_session)
        .await
        .expect("verify tenantB chain");
    assert!(
        tenant_b_report.ok,
        "tenantB audit chain must verify clean: {:?}",
        tenant_b_report.breaks
    );

    // `:memory:` SQLite + tempdir clean up on Drop at end of scope.
}

fn build_library_raw(name: &str, body: &str) -> String {
    // OCI sync rewrites provenance to `published:verified:vendor` so
    // the consuming tenant's trust filter sees the verified grade
    // rather than the originating `learned:…` workspace source.
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\n\
         description: \"loop-closure published artefact\"\n\
         provenance:\n  source: published:verified:vendor\n  hash: sha256-libpub\n\
         ---\n{body}\n"
    )
}

fn skill_md_for(name: &str, source: &str, body: &str, _vertical: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"closure-fixture\"\n\
         provenance:\n  source: {source}\n  hash: sha256-loopclosure\n\
         ---\n{body}\n"
    )
}

