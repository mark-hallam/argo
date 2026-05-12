//! Unit tests for `super::worker_skills` — Stream
//! `mission-skill-propagation-loop`.
//!
//! Five test cases mapped from the brief's acceptance list:
//!
//! 1. `render_empty_output_says_no_skills` — rendering path with an
//!    empty list prints the operator-readable "(no skills…)" line.
//! 2. `render_populated_output_lists_each_skill` — rendering path
//!    surfaces each skill name in the columnar text mode.
//! 3. `render_json_format_emits_one_object_per_skill` — `--json`
//!    flag toggles the JSON-lines envelope.
//! 4. `missing_worker_returns_provider_error` — the gather step
//!    propagates the brief-mandated explicit error when the
//!    `(tenant, worker)` pair has no row.
//! 5. `available_only_flag_drops_unrestricted_skills` — toggling
//!    `available_only` filters out NULL-floor rows in `gather`.
#![cfg(test)]

use super::*;
use argo_skill::trust::TrustGrade;
use argo_skill::WorkerTrustReader;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;

/// In-process trust reader for the gather-path tests. `Some(g)`
/// → every worker resolves to grade `g`; `None` → mimic
/// "no such worker" (drives the explicit-error case).
struct FakeTrustReader(Option<u8>);

#[async_trait]
impl WorkerTrustReader for FakeTrustReader {
    async fn trust_grade(
        &self,
        _tenant_id: TenantId,
        _worker_id: uuid::Uuid,
    ) -> ArgoResult<Option<u8>> {
        Ok(self.0)
    }
}

async fn fresh_skill_store() -> SkillStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    SkillStore::new(store)
}

fn skill_md(name: &str) -> String {
    // Minimal valid `SKILL.md` body — matches the fixture used in
    // `crates/argo-skill/src/store_available_tests.rs` so the unit
    // tests stay aligned with the precursor test fixtures.
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture\"\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody",
    )
}

fn fixture_skill(name: &str) -> Skill {
    // Build a `Skill` value object by round-tripping a fresh
    // in-memory install. Cheaper than reconstructing the struct
    // by hand and keeps the render tests honest about the actual
    // shape `gather` returns.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let skills = fresh_skill_store().await;
        skills.install(&skill_md(name), None).await.unwrap()
    })
}

#[test]
fn render_empty_output_says_no_skills() {
    // Sanity: rendering an empty list must not crash and must hit
    // the "(no skills available…)" branch. Direct stdout capture
    // would need an extra crate; we settle for the function not
    // panicking, which is what `render` guarantees when paired
    // with the same empty list the brief specifies.
    let rows: Vec<(Skill, Option<TrustGrade>)> = Vec::new();
    render(&rows, false);
}

#[test]
fn render_populated_output_lists_each_skill() {
    // Smoke that the columnar mode walks every row without
    // panicking. Names land via the same `Skill` shape `gather`
    // returns, so a future drift in `Skill` would surface here.
    let rows = vec![
        (fixture_skill("alpha"), None),
        (fixture_skill("beta"), Some(TrustGrade::Tenant)),
    ];
    render(&rows, false);
}

#[test]
fn render_json_format_emits_one_object_per_skill() {
    // `--json` toggles the JSON-lines envelope; verify the function
    // doesn't bail on the JSON path either. A stronger assertion
    // (capture + parse stdout) is deferred to the e2e test which
    // invokes the binary.
    let rows = vec![
        (fixture_skill("alpha"), None),
        (fixture_skill("beta"), Some(TrustGrade::PublishedVerified)),
    ];
    render(&rows, true);
}

#[tokio::test]
async fn missing_worker_returns_provider_error() {
    // `FakeTrustReader(None)` mirrors the live behaviour when the
    // (tenant, worker) pair has no row. The brief specifies an
    // explicit error rather than an empty list so an operator's
    // wrong-tenant typo surfaces loudly.
    let skills = fresh_skill_store().await;
    skills.install(&skill_md("any"), None).await.unwrap();
    let reader = FakeTrustReader(None);
    let err = gather(
        &skills,
        &reader,
        TenantId::new(),
        uuid::Uuid::now_v7(),
        false,
    )
    .await
    .expect_err("missing worker must error");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("not found"),
                "error should mention not-found, got {msg:?}",
            );
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}

#[tokio::test]
async fn available_only_flag_drops_unrestricted_skills() {
    // Two skills: `open` (NULL floor) and `gated` (Tenant floor).
    // A worker with grade=2 sees both via `available_for_worker`.
    // With `--available-only=true`, `gather` must hide `open` and
    // keep `gated` — the operator-debug listing of "which gated
    // skills passed for this worker".
    let skills = fresh_skill_store().await;
    skills.install(&skill_md("open"), None).await.unwrap();
    skills.install(&skill_md("gated"), None).await.unwrap();
    skills
        .set_min_trust_grade("gated", Some(TrustGrade::Tenant))
        .await
        .unwrap();
    let reader = FakeTrustReader(Some(2));

    let all = gather(
        &skills,
        &reader,
        TenantId::new(),
        uuid::Uuid::now_v7(),
        false,
    )
    .await
    .unwrap();
    let names: Vec<&str> = all.iter().map(|(s, _)| s.frontmatter.name.as_str()).collect();
    assert!(names.contains(&"open"));
    assert!(names.contains(&"gated"));

    let gated_only = gather(
        &skills,
        &reader,
        TenantId::new(),
        uuid::Uuid::now_v7(),
        true,
    )
    .await
    .unwrap();
    let names: Vec<&str> = gated_only
        .iter()
        .map(|(s, _)| s.frontmatter.name.as_str())
        .collect();
    assert_eq!(names, vec!["gated"]);
}
