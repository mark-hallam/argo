//! Tests for the A/B-by-eval closed loop.
//!
//! The core proof: install a baseline body that LACKS the required
//! guidance (fails the eval), supersede with an improved body that ADDS
//! it (passes), then assert the measured pass-rate delta is real
//! (`0.0 → 1.0`). The executor is the production `BodyContainsExecutor`
//! — no mock — so the delta is a genuine measurement of the two bodies.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;

async fn skills() -> SkillStore {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    SkillStore::new(session)
}

/// A `SKILL.md` whose body is `body` and whose single eval case requires
/// the output to CONTAIN `required`. The `BodyContainsExecutor` returns
/// the body, so the case passes iff `body` contains `required`.
fn skill_doc(name: &str, version: &str, body: &str, required: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"{version}\"\n\
         description: refund policy skill\n\
         provenance:\n  source: builtin\n  hash: sha256-test\n\
         evals:\n  - input: \"what is the refund window?\"\n    expected:\n      kind: contains\n      text: \"{required}\"\n\
         ---\n\
         {body}\n"
    )
}

#[tokio::test]
async fn measures_real_improvement_old_fails_new_passes() {
    let skills = skills().await;
    let required = "refund window is 30 days";

    // Baseline body lacks the required guidance → its eval case fails.
    let baseline_body = "Answer refund questions politely.";
    skills
        .install(&skill_doc("refund-helper", "v1", baseline_body, required), None)
        .await
        .expect("install baseline");

    // Improved body adds the exact guidance → its eval case passes.
    let improved_body = "Answer refund questions politely. The refund window is 30 days from purchase.";
    skills
        .supersede(
            &skill_doc("refund-helper", "v2", improved_body, required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede candidate");

    let report = run_ab_eval(&skills, "refund-helper", "v1", "v2")
        .await
        .expect("ab-eval runs");

    // Real measured numbers: baseline 0/1, candidate 1/1.
    assert_eq!(report.case_count, 1);
    assert_eq!(report.baseline.pass, 0, "old body must fail the case");
    assert_eq!(report.baseline.fail, 1);
    assert!((report.baseline.pass_rate - 0.0).abs() < f32::EPSILON);
    assert_eq!(report.candidate.pass, 1, "new body must pass the case");
    assert_eq!(report.candidate.fail, 0);
    assert!((report.candidate.pass_rate - 1.0).abs() < f32::EPSILON);
    // Delta is a true +100 percentage-point improvement.
    assert!((report.pass_rate_delta - 1.0).abs() < f32::EPSILON);
    assert!(report.improved);
}

#[tokio::test]
async fn no_improvement_reports_zero_delta() {
    let skills = skills().await;
    let required = "refund window is 30 days";
    // Both bodies already contain the guidance → both pass, delta = 0.
    let good = "The refund window is 30 days. Be kind.";
    skills
        .install(&skill_doc("refund-stable", "v1", good, required), None)
        .await
        .expect("install");
    skills
        .supersede(
            &skill_doc("refund-stable", "v2", good, required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede");

    let report = run_ab_eval(&skills, "refund-stable", "v1", "v2")
        .await
        .expect("ab-eval runs");

    assert_eq!(report.baseline.pass_rate, report.candidate.pass_rate);
    assert!((report.pass_rate_delta - 0.0).abs() < f32::EPSILON);
    assert!(!report.improved, "equal pass rates are not an improvement");
}

#[tokio::test]
async fn missing_version_is_an_error() {
    let skills = skills().await;
    let required = "x";
    skills
        .install(&skill_doc("solo", "v1", "x", required), None)
        .await
        .expect("install");
    let err = run_ab_eval(&skills, "solo", "v1", "v99")
        .await
        .expect_err("missing candidate version must error");
    assert!(err.to_string().contains("no version 'v99'"), "got: {err}");
}

#[tokio::test]
async fn identical_versions_rejected() {
    let skills = skills().await;
    let required = "x";
    skills
        .install(&skill_doc("same", "v1", "x", required), None)
        .await
        .expect("install");
    let err = run_ab_eval(&skills, "same", "v1", "v1")
        .await
        .expect_err("identical versions must error");
    assert!(err.to_string().contains("must differ"), "got: {err}");
}

#[tokio::test]
async fn record_writes_a_real_reeval_row_a_worker_can_read() {
    let skills = skills().await;
    let required = "refund window is 30 days";
    skills
        .install(&skill_doc("refund-rec", "v1", "be kind", required), None)
        .await
        .expect("install");
    let improved = "be kind; the refund window is 30 days";
    let candidate = skills
        .supersede(
            &skill_doc("refund-rec", "v2", improved, required),
            None,
            crate::skill::PrincipalOverride::Agent,
        )
        .await
        .expect("supersede");

    let report = run_ab_eval(&skills, "refund-rec", "v1", "v2")
        .await
        .expect("ab-eval");
    let run_id = record_candidate_reeval(&skills, &report)
        .await
        .expect("record");
    assert!(!run_id.is_empty());

    // The recorded row carries the REAL measured numbers, keyed on the
    // candidate skill id, with the baseline as previous_pass_rate.
    let row: (String, f64, Option<f64>, i64) = sqlx::query_as(
        "SELECT skill_version, pass_rate, previous_pass_rate, regressed \
         FROM skill_reeval_history WHERE eval_run_id = ?",
    )
    .bind(&run_id)
    .fetch_one(skills.store_arc().pool())
    .await
    .expect("fetch recorded row");
    assert_eq!(row.0, "v2");
    assert!((row.1 - 1.0).abs() < 1e-9, "candidate pass rate = 1.0");
    assert_eq!(row.2, Some(0.0), "previous = baseline pass rate 0.0");
    assert_eq!(row.3, 0, "improvement is not a regression");

    // Ground truth: a worker that LEARNED this candidate skill joins to
    // exactly this reeval row (the exact query the evals route runs).
    let worker = argo_store::WorkerId::new();
    sqlx::query("INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5)")
        .bind(worker.to_string())
        .bind(candidate.id.to_string())
        .execute(skills.store_arc().pool())
        .await
        .expect("attach skill to worker");
    let joined: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker_skill ws \
         JOIN skill_reeval_history h ON h.skill_id = ws.skill_id \
         WHERE ws.worker_id = ?",
    )
    .bind(worker.to_string())
    .fetch_one(skills.store_arc().pool())
    .await
    .expect("join");
    assert_eq!(joined, 1, "worker's evals tab surfaces the A/B run");
}

#[tokio::test]
async fn no_eval_cases_is_an_error() {
    let skills = skills().await;
    // Two versions, neither declaring evals → nothing to measure.
    let doc = |v: &str| {
        format!(
            "---\nname: noeval\nversion: \"{v}\"\ndescription: d\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody {v}\n"
        )
    };
    skills.install(&doc("v1"), None).await.expect("install");
    skills
        .supersede(&doc("v2"), None, crate::skill::PrincipalOverride::Agent)
        .await
        .expect("supersede");
    let err = run_ab_eval(&skills, "noeval", "v1", "v2")
        .await
        .expect_err("no eval cases must error");
    assert!(err.to_string().contains("declares eval cases"), "got: {err}");
}
