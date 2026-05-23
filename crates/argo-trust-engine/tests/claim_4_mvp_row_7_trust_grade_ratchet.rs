//! # Mission Claim 4 MVP row 7 — trust grade ratchets across the 5 signals
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > Trust grade ratchets after successful invocations using 5 signals
//! > (approvals granted/rejected, escalations, RBAC denies, quality-avg).
//!
//! Cite: `argo-trust-engine::scoring::compute_score` +
//! `ratchet::ratchet_grade`. Substrate found round 20; this round
//! flips the row 🟢 → 🟢🟢 with a falsifiable LIVE regression that
//! composes BOTH halves together.
//!
//! ## What this test proves
//!
//! 1. **End-to-end ratchet UP** — seed real audit events of the 5
//!    documented signal kinds (`approval.granted`, `approval.replied`
//!    with `reply_kind=reject`, `escalation.required`, `rbac.deny`, +
//!    `trust_observation` quality rows). Run `compute_score`. Feed the
//!    total into `ratchet_grade(LearnedUnreviewed, total)`. With enough
//!    positive signals, the decision MUST be `Promote` from
//!    `LearnedUnreviewed` → `PublishedCommunity`.
//! 2. **Each subscore reflects its signal** — the breakdown's
//!    `approval_score / escalation_score / rbac_score / quality_score /
//!    streak_score` each match the documented weight × count. This
//!    pins the contract that `TrustGradeChanged` audit-payload
//!    consumers (the SPA's worker-detail Trust panel) depend on.
//! 3. **End-to-end ratchet DOWN** — add a single rejected reply on top
//!    of an otherwise-positive history. The total drops below the
//!    promote line. Compose with a `Tenant`-grade starting position →
//!    ratchet MUST decide `Demote` from `Tenant` →
//!    `PublishedCommunity`.
//! 4. **One rung per call** — even with a huge positive score swing,
//!    a single ratchet call moves at most one rung. Pins the brief's
//!    "audit chain shows the journey" invariant.
//! 5. **Cross-tenant isolation at the FULL-LOOP scope** — tenant_b's
//!    audit events do not affect tenant_a's grade decision. Existing
//!    `cross_tenant_isolation.rs` proves this for `compute_score`
//!    alone; here we re-check it through the full
//!    compute → ratchet pipeline.
//!
//! ## Why this is the MVP row 7 e2e pin
//!
//! The existing unit tests prove `compute_score` and `ratchet_grade`
//! work in ISOLATION. This test pins the LOOP — the operator's
//! actual promise that "trust grade ratchets after successful
//! invocations." Without it, a future refactor could keep both halves
//! green but break the meaningful integration (e.g., the score signs
//! flip but the ratchet thresholds keep their old direction, or vice
//! versa).
//!
//! ## Authored
//!
//! Evaluator round 28, 2026-05-23.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_trust_engine::{compute_score, ratchet_grade, RatchetDecision};
use argo_types::TenantId;
use chrono::Utc;
use serde_json::json;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn create_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    persona_name: &str,
) -> WorkerId {
    let worker = Worker::draft(
        tenant,
        "marketplace-seller",
        Persona::named(persona_name, "human:operator-mark"),
        Role::new("Claim-4 MVP row 7 trust ratchet test"),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create_worker");
    worker.id
}

async fn emit(
    sink: &AuditSink,
    tenant: &TenantId,
    kind: AuditKind,
    worker_id: &WorkerId,
    extra: serde_json::Value,
) {
    let mut payload = json!({
        "worker_id": worker_id.to_string(),
    });
    if let Some(obj) = extra.as_object() {
        for (k, v) in obj {
            payload[k] = v.clone();
        }
    }
    sink.write_with_tenant(None, kind, payload, None, &tenant.to_string())
        .await
        .expect("audit write");
    sink.flush_pending().await.expect("flush");
}

/// Insert a `trust_observation` row directly — the SQL schema is the
/// only writer surface today (the operator-locked recompute path
/// uses raw inserts in production). Quality value is clamped 0.0-1.0
/// by the scoring function so 0.9 is meaningful.
async fn insert_quality_observation(
    store: &Arc<SessionStore>,
    source_tenant: TenantId,
    quality: f32,
) {
    use uuid::Uuid;
    let id_blob = Uuid::now_v7().as_bytes().to_vec();
    let consumer_blob = TenantId::new().0.as_bytes().to_vec();
    let skill_blob = Uuid::now_v7().as_bytes().to_vec();
    let now_micros = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO trust_observation \
         (id, source_tenant_id, consumer_tenant_id, skill_id, quality_score, observed_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id_blob)
    .bind(source_tenant.0.as_bytes().to_vec())
    .bind(consumer_blob)
    .bind(skill_blob)
    .bind(quality)
    .bind(now_micros)
    .execute(store.pool())
    .await
    .expect("insert trust_observation");
}

#[tokio::test]
async fn claim_4_mvp_row_7_full_loop_ratchets_up_from_unreviewed() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Marketplace").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // ── Phase 1: seed a positive-leaning signal mix.
    // Operator framing: a worker that's been getting approved
    // consistently, with a couple of escalations (allowable noise)
    // and good cross-tenant quality observations.

    // 5 consecutive granted approvals (streak = 5).
    for _ in 0..5 {
        emit(
            &sink,
            &tenant,
            AuditKind::ApprovalGranted,
            &worker_id,
            json!({}),
        )
        .await;
    }
    // 1 escalation — small negative (-1.0 weight).
    emit(
        &sink,
        &tenant,
        AuditKind::EscalationRequired,
        &worker_id,
        json!({ "tool": "fb_marketplace.post" }),
    )
    .await;
    // 3 trust_observation rows at quality 0.9 → mean = 0.9 → score = 4.5.
    for _ in 0..3 {
        insert_quality_observation(&store, tenant, 0.9).await;
    }

    // ── Phase 2: compute the score.
    let score = compute_score(&store, &tenant, &worker_id.to_string())
        .await
        .expect("compute_score");

    // ── Phase 3: each subscore matches its weighted signal count.
    // Approval-granted weight = +1.0 each × 5 grants = +5.0.
    assert!(
        (score.approval_score - 5.0).abs() < 0.01,
        "claim-4 MVP row 7: approval_score must be 5.0 (5 grants × 1.0); got {}",
        score.approval_score
    );
    // Streak weight = +0.5 × 5 consecutive grants = +2.5.
    assert!(
        (score.streak_score - 2.5).abs() < 0.01,
        "claim-4 MVP row 7: streak_score must be 2.5 (longest-streak 5 × 0.5); got {}",
        score.streak_score
    );
    // Escalation weight = −1.0 × 1 = −1.0.
    assert!(
        (score.escalation_score - (-1.0)).abs() < 0.01,
        "claim-4 MVP row 7: escalation_score must be -1.0 (1 × -1.0); got {}",
        score.escalation_score
    );
    // RBAC deny = 0 (none emitted).
    assert_eq!(
        score.rbac_score, 0.0,
        "claim-4 MVP row 7: rbac_score must be 0 when no RBAC denies seeded"
    );
    // Quality = mean 0.9 × +5.0 = +4.5.
    assert!(
        (score.quality_score - 4.5).abs() < 0.01,
        "claim-4 MVP row 7: quality_score must be 4.5 (0.9 mean × 5.0 weight); got {}",
        score.quality_score
    );

    // Total = 5.0 + 2.5 − 1.0 + 0.0 + 4.5 = 11.0
    assert!(
        (score.total - 11.0).abs() < 0.01,
        "claim-4 MVP row 7: total must be 11.0; got {}",
        score.total
    );

    // ── Phase 4: feed score into ratchet — MUST promote one rung.
    let decision = ratchet_grade(TrustGrade::LearnedUnreviewed, score.total);
    assert_eq!(
        decision,
        RatchetDecision::Promote {
            from: TrustGrade::LearnedUnreviewed,
            to: TrustGrade::PublishedCommunity,
        },
        "claim-4 MVP row 7: total={} from LearnedUnreviewed must Promote → PublishedCommunity; got {:?}",
        score.total,
        decision
    );

    // ── Phase 5: even with the strong total (~11.0), still only one
    // rung per call — promote does not jump multiple grades.
    // (Existing ratchet.rs has a unit test for this; we re-pin it
    // through the full loop.)
    let decision_starting_at_unreviewed_huge_score =
        ratchet_grade(TrustGrade::LearnedUnreviewed, 999.0);
    if let RatchetDecision::Promote { to, .. } = decision_starting_at_unreviewed_huge_score {
        assert_eq!(
            to,
            TrustGrade::PublishedCommunity,
            "claim-4 MVP row 7: even huge score must promote at most one rung; got to={to:?}"
        );
    } else {
        panic!("claim-4 MVP row 7: huge score must Promote; got {decision_starting_at_unreviewed_huge_score:?}");
    }
}

#[tokio::test]
async fn claim_4_mvp_row_7_full_loop_ratchets_down_on_rejections() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Slipping").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // ── Phase 1: seed signals that produce a low-positive total —
    // strong enough to hold at PublishedCommunity but weak enough
    // that demote-line crossing flips it.
    // 2 approvals → +2.0, streak 2 → +1.0.
    for _ in 0..2 {
        emit(
            &sink,
            &tenant,
            AuditKind::ApprovalGranted,
            &worker_id,
            json!({}),
        )
        .await;
    }
    // 1 rejection (resets streak; subtracts 2.0).
    emit(
        &sink,
        &tenant,
        AuditKind::ApprovalReplied,
        &worker_id,
        json!({ "reply_kind": "reject" }),
    )
    .await;

    let score = compute_score(&store, &tenant, &worker_id.to_string())
        .await
        .expect("compute_score");
    // 2 grants × +1.0 + 1 reject × −2.0 = 0.0
    assert!(
        (score.approval_score - 0.0).abs() < 0.01,
        "claim-4 MVP row 7: 2 grants - 1 reject must = 0 approval_score; got {}",
        score.approval_score
    );
    // Streak: grant, grant, reject → longest = 2 → +1.0.
    assert!(
        (score.streak_score - 1.0).abs() < 0.01,
        "claim-4 MVP row 7: longest streak was 2 (before reject); got {}",
        score.streak_score
    );
    // Total = 0 + 1.0 = 1.0
    assert!(
        (score.total - 1.0).abs() < 0.01,
        "claim-4 MVP row 7: total = 1.0; got {}",
        score.total
    );

    // ── Phase 2: from Tenant grade, total=1.0 is below the
    // Tenant-demote line (3.5) → MUST Demote → PublishedCommunity.
    let decision = ratchet_grade(TrustGrade::Tenant, score.total);
    assert_eq!(
        decision,
        RatchetDecision::Demote {
            from: TrustGrade::Tenant,
            to: TrustGrade::PublishedCommunity,
        },
        "claim-4 MVP row 7: total={} from Tenant must Demote → PublishedCommunity; got {:?}",
        score.total,
        decision
    );

    // ── Phase 3: from PublishedCommunity, total=1.0 is at the
    // demote-line (1.0 is NOT < 1.0) → Hold (hysteresis).
    let decision_pc = ratchet_grade(TrustGrade::PublishedCommunity, score.total);
    assert_eq!(
        decision_pc,
        RatchetDecision::Hold,
        "claim-4 MVP row 7: at PublishedCommunity with total=1.0 (== demote line) must Hold; got {:?}",
        decision_pc
    );
}

#[tokio::test]
async fn claim_4_mvp_row_7_rbac_denies_drag_grade_down() {
    // Defensive — RBAC denies are the "trust eroder" signal. A
    // worker hitting capability boundaries repeatedly should NOT
    // promote even with positive approval signals.
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Misbehaving").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // 2 approvals = +2.0, streak 2 = +1.0
    for _ in 0..2 {
        emit(
            &sink,
            &tenant,
            AuditKind::ApprovalGranted,
            &worker_id,
            json!({}),
        )
        .await;
    }
    // 6 RBAC denies = -3.0 (each × -0.5)
    for _ in 0..6 {
        emit(
            &sink,
            &tenant,
            AuditKind::RbacDeny,
            &worker_id,
            json!({ "tool": "internal_admin.list_users" }),
        )
        .await;
    }

    let score = compute_score(&store, &tenant, &worker_id.to_string())
        .await
        .expect("compute_score");
    assert!(
        (score.rbac_score - (-3.0)).abs() < 0.01,
        "claim-4 MVP row 7: rbac_score must be -3.0 (6 × -0.5); got {}",
        score.rbac_score
    );
    // 2.0 + 1.0 − 3.0 = 0.0
    assert!(
        (score.total - 0.0).abs() < 0.01,
        "claim-4 MVP row 7: rbac-dragged total must be 0.0; got {}",
        score.total
    );

    // From LearnedUnreviewed, total=0.0 → MUST Hold (below promote
    // line 2.0).
    let decision = ratchet_grade(TrustGrade::LearnedUnreviewed, score.total);
    assert_eq!(
        decision,
        RatchetDecision::Hold,
        "claim-4 MVP row 7: 6 RBAC denies must drag total below promote line; got {:?}",
        decision
    );
}

#[tokio::test]
async fn claim_4_mvp_row_7_cross_tenant_isolation_at_full_loop_scope() {
    // Re-checks cross-tenant isolation at the full compute → ratchet
    // pipeline (not just compute_score, which is covered by the
    // existing tests/cross_tenant_isolation.rs).
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = create_worker(&store, tenant_a, "Mariah-A").await;
    let worker_b = create_worker(&store, tenant_b, "Mariah-B").await;
    let sink_a = AuditSink::new_with_tenant(Arc::clone(&store), tenant_a.to_string());
    let sink_b = AuditSink::new_with_tenant(Arc::clone(&store), tenant_b.to_string());

    // Tenant_b fires a NEGATIVE storm (10 RBAC denies + 3 rejections).
    for _ in 0..10 {
        emit(
            &sink_b,
            &tenant_b,
            AuditKind::RbacDeny,
            &worker_b,
            json!({}),
        )
        .await;
    }
    for _ in 0..3 {
        emit(
            &sink_b,
            &tenant_b,
            AuditKind::ApprovalReplied,
            &worker_b,
            json!({ "reply_kind": "reject" }),
        )
        .await;
    }

    // Tenant_a fires a POSITIVE storm (5 grants + 3 high-quality observations).
    for _ in 0..5 {
        emit(
            &sink_a,
            &tenant_a,
            AuditKind::ApprovalGranted,
            &worker_a,
            json!({}),
        )
        .await;
    }
    for _ in 0..3 {
        insert_quality_observation(&store, tenant_a, 0.95).await;
    }

    // Tenant_a's score must reflect ONLY tenant_a's events.
    let score_a = compute_score(&store, &tenant_a, &worker_a.to_string())
        .await
        .expect("compute_score A");
    assert!(
        (score_a.approval_score - 5.0).abs() < 0.01,
        "claim-4 MVP row 7: tenant_a approval_score must be 5.0 (UNAFFECTED by tenant_b rejections); got {}",
        score_a.approval_score
    );
    assert_eq!(
        score_a.rbac_score, 0.0,
        "claim-4 MVP row 7: tenant_a rbac_score must be 0.0 (UNAFFECTED by tenant_b's 10 RBAC denies)"
    );
    assert!(
        score_a.total > 5.0,
        "claim-4 MVP row 7: tenant_a's positive total must hold; got {}",
        score_a.total
    );

    // Tenant_a's grade decision MUST be Promote regardless of
    // tenant_b's noise.
    let decision_a = ratchet_grade(TrustGrade::LearnedUnreviewed, score_a.total);
    assert!(
        matches!(decision_a, RatchetDecision::Promote { .. }),
        "claim-4 MVP row 7: tenant_a's grade must promote despite tenant_b's noise; got {decision_a:?}"
    );

    // Symmetric — tenant_b's grade must reflect ONLY tenant_b's events.
    let score_b = compute_score(&store, &tenant_b, &worker_b.to_string())
        .await
        .expect("compute_score B");
    assert!(
        score_b.total < 0.0,
        "claim-4 MVP row 7: tenant_b's total must be negative; got {}",
        score_b.total
    );
    let decision_b = ratchet_grade(TrustGrade::LearnedUnreviewed, score_b.total);
    assert_eq!(
        decision_b,
        RatchetDecision::Hold,
        "claim-4 MVP row 7: tenant_b's grade MUST NOT promote (already at floor); got {decision_b:?}"
    );
}
