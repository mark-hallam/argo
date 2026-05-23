//! # eval-trust-engine-ratchet-from-zero-to-grade-A
//!
//! Multi-tick zero-to-top trust-grade journey driven through the
//! [`recompute_worker`] writer surface — the production code path
//! `tick_all_workers` walks every tick of the cron loop. Existing
//! in-crate tests cover [`compute_score`] and [`ratchet_grade`] in
//! isolation; this file pins the multi-tick LOOP and is the eval
//! pin for `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! row 11:
//!
//! > argo-trust-engine | NOT AUDITED | Trust grade ratchet. Gap:
//! > which signals advance the grade? Does it actually fire when a
//! > worker completes successful executions without RBAC denies?
//! > `eval-trust-engine-ratchet-from-zero-to-grade-A` — simulate 10
//! > successful tool_call executions → verify trust grade advances
//! > → simulate 1 RBAC deny → grade pauses.
//!
//! Pinned here, in order:
//!
//! 1. **Zero-to-top across 4 ticks.** Hire at `LearnedUnreviewed`;
//!    each tick accumulates more positive signals and the persisted
//!    grade climbs one rung per call (re-reading the grade from
//!    `IdentityStore` between ticks, the way `tick_all_workers` does).
//! 2. **Audit chain shows the journey.** One `trust_grade_changed`
//!    row per rung, `from`/`to` payload in order — the contract the
//!    SPA worker-detail Trust panel consumes.
//! 3. **RBAC denies pause promotion.** A signal mix that would
//!    promote `PublishedCommunity → Tenant` is held by a burst of
//!    `rbac.deny` rows; counterfactual twin without the denies
//!    promotes.
//! 4. **Flap-resistance under [`recompute_worker`].** 20 recompute
//!    ticks inside the hysteresis band leave the persisted grade
//!    unchanged and emit zero `trust_grade_changed` rows.
//!
//! Brief authored against origin/main `d0ff84a52`, 2026-05-23.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_trust_engine::{
    ratchet::{grade_to_u8, u8_to_grade},
    recompute_worker, RatchetDecision,
};
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

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
        Role::new("eval-trust-engine-ratchet-from-zero-to-grade-A"),
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
    extra: Value,
) {
    let mut payload = json!({ "worker_id": worker_id.to_string() });
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

async fn insert_quality_observation(
    store: &Arc<SessionStore>,
    source_tenant: TenantId,
    quality: f32,
) {
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

/// Read the persisted `worker.trust_grade` u8 and map it back through
/// [`u8_to_grade`] — the exact path `tick_all_workers` takes.
async fn persisted_grade(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> TrustGrade {
    let worker = IdentityStore::new(store)
        .get_worker(tenant, worker_id)
        .await
        .expect("get_worker")
        .expect("worker present");
    u8_to_grade(worker.trust_grade).expect("known rung")
}

/// Count `trust_grade_changed` audit rows for a (tenant, worker)
/// pair, in created-at-order, returning the (`from_grade`,
/// `to_grade`) pair extracted from each payload. Used to assert the
/// audit chain records the journey.
async fn trust_grade_changed_chain(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: &WorkerId,
) -> Vec<(String, String)> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind = 'trust_grade_changed' \
         ORDER BY created_at ASC, seq ASC",
    )
    .bind(tenant.to_string())
    .bind(worker_id.to_string())
    .fetch_all(store.pool())
    .await
    .expect("trust_grade_changed query");
    rows.into_iter()
        .filter_map(|(payload,)| {
            let value: Value = serde_json::from_str(payload.as_deref()?).ok()?;
            let from = value.get("from_grade")?.as_str()?.to_string();
            let to = value.get("to_grade")?.as_str()?.to_string();
            Some((from, to))
        })
        .collect()
}

#[tokio::test]
async fn zero_to_top_journey_promotes_one_rung_per_recompute_tick() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Climber").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // ── Tick 0 (sanity): a fresh hire is at LearnedUnreviewed
    //    (`trust_grade` u8 = 0) with no signals — recompute MUST
    //    Hold. Pins "hire at trust=None" in the brief.
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::LearnedUnreviewed,
        "fresh hire must start at LearnedUnreviewed (u8=0)"
    );
    let decision = recompute_worker(
        &store,
        &sink,
        &tenant,
        &worker_id.to_string(),
        TrustGrade::LearnedUnreviewed,
    )
    .await
    .expect("tick 0 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Hold,
        "tick 0 (no signals) must Hold; got {decision:?}"
    );

    // ── Tick 1 — LearnedUnreviewed → PublishedCommunity.
    // Promote line for PublishedCommunity = 2.0.
    // Seed: 2 grants → +2.0 approval, streak 2 → +1.0 → total +3.0.
    for _ in 0..2 {
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
    }
    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("tick 1 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Promote {
            from: TrustGrade::LearnedUnreviewed,
            to: TrustGrade::PublishedCommunity,
        },
        "tick 1 must Promote to PublishedCommunity; got {decision:?}"
    );
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::PublishedCommunity,
        "worker.trust_grade column must be 1 after tick 1"
    );

    // ── Tick 2 — PublishedCommunity → Tenant.
    // Promote line for Tenant = 5.0.
    // Seed: +3 more grants (5 cumulative) → +5.0 approval, +2.5
    // streak → total +7.5.
    for _ in 0..3 {
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
    }
    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("tick 2 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Promote {
            from: TrustGrade::PublishedCommunity,
            to: TrustGrade::Tenant,
        },
        "tick 2 must Promote to Tenant; got {decision:?}"
    );
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::Tenant,
    );

    // ── Tick 3 — Tenant → PublishedVerified.
    // Promote line for PublishedVerified = 10.0.
    // Seed: 3 quality observations at 0.9 → +0.9 × 5.0 = +4.5.
    // Cumulative total ≈ 7.5 + 4.5 = 12.0.
    for _ in 0..3 {
        insert_quality_observation(&store, tenant, 0.9).await;
    }
    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("tick 3 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Promote {
            from: TrustGrade::Tenant,
            to: TrustGrade::PublishedVerified,
        },
        "tick 3 must Promote to PublishedVerified; got {decision:?}"
    );
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::PublishedVerified,
    );

    // ── Tick 4 — PublishedVerified → Builtin.
    // Promote line for Builtin = 20.0.
    // Seed: 10 more grants (15 cumulative) → +15.0 approval,
    // streak now 15 → +7.5 streak, +4.5 quality = +27.0.
    for _ in 0..10 {
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
    }
    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("tick 4 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Promote {
            from: TrustGrade::PublishedVerified,
            to: TrustGrade::Builtin,
        },
        "tick 4 must Promote to Builtin; got {decision:?}"
    );
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::Builtin,
    );
    assert_eq!(
        grade_to_u8(persisted_grade(&store, tenant, worker_id).await),
        4,
        "Builtin must persist as u8=4"
    );

    // ── Tick 5 — already at the top; recompute MUST Hold.
    // Pins "does not promote above top grade" through the writer.
    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("tick 5 recompute");
    assert_eq!(
        decision,
        RatchetDecision::Hold,
        "tick 5 at Builtin must Hold; got {decision:?}"
    );

    // ── Audit chain: 4 promotions, in order.
    // Pins the `trust_grade_changed` payload contract the SPA
    // worker-detail Trust panel consumes.
    let chain = trust_grade_changed_chain(&store, tenant, &worker_id).await;
    assert_eq!(
        chain.len(),
        4,
        "audit chain must record one row per rung climbed; got {chain:?}"
    );
    let expected = [
        ("LearnedUnreviewed", "PublishedCommunity"),
        ("PublishedCommunity", "Tenant"),
        ("Tenant", "PublishedVerified"),
        ("PublishedVerified", "Builtin"),
    ];
    for (idx, ((got_from, got_to), (want_from, want_to))) in
        chain.iter().zip(expected.iter()).enumerate()
    {
        assert_eq!(
            (got_from.as_str(), got_to.as_str()),
            (*want_from, *want_to),
            "audit chain rung {idx} must record {want_from} → {want_to}; got {got_from} → {got_to}"
        );
    }
}

#[tokio::test]
async fn rbac_deny_burst_pauses_promotion_at_published_community() {
    // From the gap manifest: "simulate 10 successful tool_call
    // executions → verify trust grade advances → simulate 1 RBAC
    // deny → grade pauses."
    //
    // We model the pause at the `PublishedCommunity → Tenant`
    // threshold (5.0). 5 grants alone give +5.0 approval + 2.5
    // streak = +7.5 → would promote. With 6 `rbac.deny` rows
    // (-0.5 each = -3.0) the score lands at +4.5, below the
    // promote-to-Tenant line of 5.0 but above the demote-below-
    // PublishedCommunity line of 1.0 → Hold (pause).
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Paused").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // Manually advance to PublishedCommunity so the test starts at
    // the rung the brief's pause-scenario describes.
    IdentityStore::new(&store)
        .set_worker_trust_grade(tenant, worker_id, grade_to_u8(TrustGrade::PublishedCommunity))
        .await
        .expect("seed PublishedCommunity");
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::PublishedCommunity,
    );

    // 5 successful "tool_call executions" → modeled as approvals.
    for _ in 0..5 {
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
    }
    // 6 RBAC denies — the "1 RBAC deny" beat from the gap manifest
    // is scaled up to 6 here so the score crosses the threshold by
    // a clear margin. A single deny (-0.5) would leave score at
    // +7.0, still > 5.0, and a future ratchet-weight tune would
    // accidentally pass. 6 denies drag the score below the line by
    // 0.5 in absolute terms — wider safety margin without losing the
    // brief's intent ("RBAC denies hold the grade").
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

    let current = persisted_grade(&store, tenant, worker_id).await;
    let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
        .await
        .expect("paused recompute");
    assert_eq!(
        decision,
        RatchetDecision::Hold,
        "RBAC-deny burst must pause grade at PublishedCommunity; got {decision:?}"
    );
    assert_eq!(
        persisted_grade(&store, tenant, worker_id).await,
        TrustGrade::PublishedCommunity,
        "worker.trust_grade column must NOT advance"
    );

    // Negative half: same signal mix WITHOUT the RBAC denies must
    // promote. Pin the counterfactual so a future regression that
    // makes RBAC denies ineffective shows up as a false-promote
    // here. We use a SECOND tenant + worker so the audit-row
    // accumulation from the paused worker doesn't bleed in.
    let tenant2 = TenantId::new();
    let twin = create_worker(&store, tenant2, "Mariah-Twin").await;
    let sink2 = AuditSink::new_with_tenant(Arc::clone(&store), tenant2.to_string());
    IdentityStore::new(&store)
        .set_worker_trust_grade(tenant2, twin, grade_to_u8(TrustGrade::PublishedCommunity))
        .await
        .expect("seed PublishedCommunity twin");
    for _ in 0..5 {
        emit(&sink2, &tenant2, AuditKind::ApprovalGranted, &twin, json!({})).await;
    }
    let twin_decision = recompute_worker(
        &store,
        &sink2,
        &tenant2,
        &twin.to_string(),
        TrustGrade::PublishedCommunity,
    )
    .await
    .expect("twin recompute");
    assert_eq!(
        twin_decision,
        RatchetDecision::Promote {
            from: TrustGrade::PublishedCommunity,
            to: TrustGrade::Tenant,
        },
        "without RBAC denies the same signals must promote; got {twin_decision:?}"
    );
}

#[tokio::test]
async fn flap_resistance_across_recompute_ticks_holds_grade() {
    // Pin flap-resistance through the full writer pipeline (the
    // pure-ratchet version lives in `tests/flap_resistance_tests.rs`).
    //
    // Seed signals that produce a score inside the
    // `PublishedCommunity` hysteresis band (demote_below=1.0,
    // promote_to_tenant=5.0). 2 grants + 2 escalations = +2.0
    // approval + +1.0 streak - 2.0 escalation = +1.0, sitting right
    // at the demote-line edge (1.0 is not strictly less than 1.0
    // → Hold). 20 recompute ticks against the same DB state must
    // NEVER move the persisted grade. A future score-quantization
    // bug or read/write inversion would surface as a flap here.
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let worker_id = create_worker(&store, tenant, "Mariah-Steady").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    IdentityStore::new(&store)
        .set_worker_trust_grade(tenant, worker_id, grade_to_u8(TrustGrade::PublishedCommunity))
        .await
        .expect("seed PublishedCommunity");

    for _ in 0..2 {
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
    }
    for _ in 0..2 {
        emit(
            &sink,
            &tenant,
            AuditKind::EscalationRequired,
            &worker_id,
            json!({ "tool": "fb_marketplace.post" }),
        )
        .await;
    }

    let baseline = persisted_grade(&store, tenant, worker_id).await;
    assert_eq!(baseline, TrustGrade::PublishedCommunity);

    for tick in 0..20 {
        let current = persisted_grade(&store, tenant, worker_id).await;
        let decision = recompute_worker(&store, &sink, &tenant, &worker_id.to_string(), current)
            .await
            .unwrap_or_else(|e| panic!("tick {tick} recompute: {e}"));
        assert_eq!(
            decision,
            RatchetDecision::Hold,
            "tick {tick}: recompute_worker must Hold inside hysteresis band; got {decision:?}"
        );
        assert_eq!(
            persisted_grade(&store, tenant, worker_id).await,
            baseline,
            "tick {tick}: worker.trust_grade column must not move"
        );
    }

    // Belt-and-braces: no `trust_grade_changed` audit row was
    // emitted across the 20 ticks. A Hold decision short-circuits
    // before the audit write in `recompute_worker`, so the row
    // count must stay at zero.
    let chain = trust_grade_changed_chain(&store, tenant, &worker_id).await;
    assert!(
        chain.is_empty(),
        "Hold decisions must NOT emit trust_grade_changed; got {chain:?}"
    );
}
