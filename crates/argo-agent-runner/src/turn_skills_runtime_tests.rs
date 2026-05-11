//! Unit tests for `turn_skills_runtime`. Sibling file so the main
//! module stays under the 500-LOC HARD RULE 6 ceiling.
use super::*;
use argo_skill::{
    create_run, ensure_ab_schema, variant_counts, LifecycleState, Provenance, Skill,
    SkillFrontmatter, Variant,
};
use argo_store::SessionStore;
use argo_types::{CapabilitySet, Lock, SkillId};
use chrono::Utc;
use uuid::Uuid;

fn skill(name: &str, version: &str, body: &str) -> Skill {
    Skill {
        id: SkillId(Uuid::now_v7()),
        frontmatter: SkillFrontmatter {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("{name} {version}"),
            capabilities: CapabilitySet::default(),
            provenance: Provenance {
                source: "user".into(),
                hash: "sha256-x".into(),
            },
            lock: Lock::None,
            evals: Vec::new(),
        },
        body: body.to_string(),
        hash: [0u8; 32],
        learned_in: None,
        superseded_by: None,
        created_at: Utc::now(),
        vertical_id: None,
        lifecycle_state: LifecycleState::Active,
    }
}

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Seed minimum-viable rows in `skill` so `create_run`'s
/// `find_version` checks succeed for `(name, vA)` + `(name, vB)`.
async fn insert_skill_row(store: &SessionStore, name: &str, version: &str) {
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
                            learned_in, superseded_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().to_vec())
    .bind(name)
    .bind(version)
    .bind("body")
    .bind("{}")
    .bind("user")
    .bind(vec![0u8; 32])
    .bind(Option::<&[u8]>::None)
    .bind(Option::<&[u8]>::None)
    .bind(0_i64)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn single_variant_skips_select_variant_and_emits_no_pending() {
    let store = open_store().await;
    let active = vec![skill("alpha", "v1", "body-a")];
    let (out, pending) =
        resolve_variants_for_turn(&store, "inv-1", active).await;
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].frontmatter.name, "alpha");
    assert!(pending.is_empty(), "single-variant case must skip select_variant");
}

#[tokio::test]
async fn two_variants_deterministic_selection_via_ab_run() {
    let store = open_store().await;
    ensure_ab_schema(&store).await.unwrap();
    insert_skill_row(&store, "alpha", "v1").await;
    insert_skill_row(&store, "alpha", "v2").await;
    let run_id = create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();

    let active = vec![
        skill("alpha", "v1", "body-v1"),
        skill("alpha", "v2", "body-v2"),
    ];
    let invocation_id = "inv-fixed-1";
    let (out, pending) =
        resolve_variants_for_turn(&store, invocation_id, active.clone()).await;

    assert_eq!(out.len(), 1, "ab run should collapse two variants to one");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].run_id, run_id);
    assert_eq!(pending[0].invocation_id, invocation_id);

    // Determinism: same (run_id, invocation_id) ⇒ same variant.
    let (out2, pending2) =
        resolve_variants_for_turn(&store, invocation_id, active).await;
    assert_eq!(out[0].frontmatter.version, out2[0].frontmatter.version);
    assert_eq!(pending[0].variant, pending2[0].variant);
}

#[tokio::test]
async fn two_variants_without_ab_run_emits_no_pending() {
    let store = open_store().await;
    let active = vec![
        skill("alpha", "v1", "body-v1"),
        skill("alpha", "v2", "body-v2"),
    ];
    let (out, pending) =
        resolve_variants_for_turn(&store, "inv-1", active).await;
    // No active run found, schema absent → fall through, keep both.
    assert_eq!(out.len(), 2, "without an active ab run, all variants pass through");
    assert!(pending.is_empty());
}

#[tokio::test]
async fn record_outcome_success_writes_a_row() {
    let store = open_store().await;
    ensure_ab_schema(&store).await.unwrap();
    insert_skill_row(&store, "alpha", "v1").await;
    insert_skill_row(&store, "alpha", "v2").await;
    let run_id = create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();
    let pending = vec![PendingAbOutcome {
        run_id: run_id.clone(),
        invocation_id: "inv-success".into(),
        variant: Variant::A,
    }];
    record_pending_outcomes(&store, &pending, true, Some(42)).await;
    let (a, _b) = variant_counts(&store, &run_id).await.unwrap();
    assert_eq!(a.trials, 1);
    assert_eq!(a.successes, 1);
}

#[tokio::test]
async fn record_outcome_failure_writes_a_row_with_zero_successes() {
    let store = open_store().await;
    ensure_ab_schema(&store).await.unwrap();
    insert_skill_row(&store, "alpha", "v1").await;
    insert_skill_row(&store, "alpha", "v2").await;
    let run_id = create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();
    let pending = vec![PendingAbOutcome {
        run_id: run_id.clone(),
        invocation_id: "inv-fail".into(),
        variant: Variant::B,
    }];
    record_pending_outcomes(&store, &pending, false, None).await;
    let (_a, b) = variant_counts(&store, &run_id).await.unwrap();
    assert_eq!(b.trials, 1);
    assert_eq!(b.successes, 0);
}

#[tokio::test]
async fn select_error_falls_back_to_first_variant_when_target_version_missing() {
    let store = open_store().await;
    ensure_ab_schema(&store).await.unwrap();
    insert_skill_row(&store, "alpha", "v1").await;
    insert_skill_row(&store, "alpha", "v2").await;
    create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();

    // Active list only has v3/v4 — neither matches the run's
    // declared variants. select_variant picks A or B; either way
    // the target version isn't in the list, so we fall back.
    let active = vec![
        skill("alpha", "v3", "body-v3"),
        skill("alpha", "v4", "body-v4"),
    ];
    let (out, pending) =
        resolve_variants_for_turn(&store, "inv-mismatch", active).await;
    assert_eq!(out.len(), 1, "fall back to a single variant");
    // Fallback picks the first by version (lexicographic).
    assert_eq!(out[0].frontmatter.version, "v3");
    assert!(
        pending.is_empty(),
        "fallback path must not queue a pending outcome — there's no faithful variant choice to record"
    );
}

#[tokio::test]
async fn record_error_logs_but_doesnt_panic() {
    let store = open_store().await;
    // `skill_ab_test_outcome` table not migrated → record_outcome
    // returns an error, which the helper must swallow.
    let pending = vec![PendingAbOutcome {
        run_id: "bogus-run".into(),
        invocation_id: "inv-x".into(),
        variant: Variant::A,
    }];
    record_pending_outcomes(&store, &pending, true, None).await;
    // If we got here, the helper swallowed the error. That's the
    // whole acceptance criterion for this test.
}

#[tokio::test]
async fn record_outcome_is_idempotent_via_invocation_id() {
    let store = open_store().await;
    ensure_ab_schema(&store).await.unwrap();
    insert_skill_row(&store, "alpha", "v1").await;
    insert_skill_row(&store, "alpha", "v2").await;
    let run_id = create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();
    let pending = vec![PendingAbOutcome {
        run_id: run_id.clone(),
        invocation_id: "inv-dedup".into(),
        variant: Variant::A,
    }];
    record_pending_outcomes(&store, &pending, true, None).await;
    record_pending_outcomes(&store, &pending, false, None).await;
    let (a, _b) = variant_counts(&store, &run_id).await.unwrap();
    assert_eq!(a.trials, 1, "INSERT OR IGNORE keeps the first outcome");
    assert_eq!(a.successes, 1);
}
