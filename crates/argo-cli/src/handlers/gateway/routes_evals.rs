//! Per-worker eval dashboard payloads.
use argo_eval::{detect_eval_regression, EvalRegressionConfig, EvalRunFixture, EvalRunReport};
use serde_json::{json, Value};

pub(super) async fn list_response(worker_id: &str, _req: &str) -> (&'static str, String) {
    let (baseline, candidate) = sample_reports(worker_id);
    let regression = detect_eval_regression(
        &baseline,
        &candidate,
        EvalRegressionConfig {
            min_common_fixtures: 3,
            ..EvalRegressionConfig::default()
        },
    );
    let latest = run_json(
        worker_id,
        &candidate,
        &baseline,
        regression.decision,
        18_400,
    );
    let previous = run_json(
        worker_id,
        &baseline,
        &baseline,
        argo_eval::EvalRegressionDecision::Stable,
        16_900,
    );
    (
        "200 OK",
        json!({
            "workerId": worker_id,
            "latest": latest,
            "history": [latest.clone(), previous],
        })
        .to_string(),
    )
}

fn sample_reports(worker_id: &str) -> (EvalRunReport, EvalRunReport) {
    let prefix = worker_id.chars().take(8).collect::<String>();
    let baseline = EvalRunReport {
        id: format!("eval_{prefix}_baseline"),
        vertical: "candidate-screening".into(),
        created_at: "2026-05-13T09:30:00Z".into(),
        fixtures: vec![
            fixture("reply-late-stage-candidate", true, 0.92),
            fixture("salary-expectation", true, 0.95),
            fixture("missing-resume", true, 0.88),
        ],
    };
    let candidate = EvalRunReport {
        id: format!("eval_{prefix}_candidate"),
        vertical: "candidate-screening".into(),
        created_at: "2026-05-14T09:30:00Z".into(),
        fixtures: vec![
            fixture("reply-late-stage-candidate", false, 0.32),
            fixture("salary-expectation", true, 0.97),
            fixture("missing-resume", true, 0.89),
        ],
    };
    (baseline, candidate)
}

fn fixture(id: &str, passed: bool, score: f64) -> EvalRunFixture {
    EvalRunFixture {
        fixture: id.into(),
        skill: "candidate-screening-followup".into(),
        metric: if id == "salary-expectation" {
            "rubric"
        } else {
            "contains"
        }
        .into(),
        passed,
        score,
    }
}

fn run_json(
    worker_id: &str,
    run: &EvalRunReport,
    baseline: &EvalRunReport,
    decision: argo_eval::EvalRegressionDecision,
    duration_ms: u64,
) -> Value {
    json!({
        "id": &run.id,
        "workerId": worker_id,
        "skill": "candidate-screening-followup",
        "version": if run.id.ends_with("candidate") { "v4.2.0" } else { "v4.1.0" },
        "createdAt": &run.created_at,
        "status": if run.pass_rate() >= 0.8 { "passed" } else { "failed" },
        "passRate": run.pass_rate(),
        "meanScore": mean_score(&run.fixtures),
        "fixtureCount": run.fixtures.len(),
        "durationMs": duration_ms,
        "decision": decision,
        "fixtures": fixtures_json(&run.fixtures, &baseline.fixtures),
    })
}

fn fixtures_json(fixtures: &[EvalRunFixture], baseline: &[EvalRunFixture]) -> Vec<Value> {
    fixtures
        .iter()
        .map(|f| {
            let previous = baseline
                .iter()
                .find(|b| b.fixture == f.fixture)
                .map(|b| b.score)
                .unwrap_or(f.score);
            json!({
                "fixture": f.fixture,
                "skill": &f.skill,
                "metric": &f.metric,
                "expected": expected_for(&f.fixture),
                "actual": actual_for(&f.fixture, f.passed),
                "passed": f.passed,
                "score": f.score,
                "previousScore": previous,
                "scoreDelta": f.score - previous,
            })
        })
        .collect()
}

fn mean_score(fixtures: &[EvalRunFixture]) -> f64 {
    if fixtures.is_empty() {
        0.0
    } else {
        fixtures.iter().map(|f| f.score).sum::<f64>() / fixtures.len() as f64
    }
}

fn expected_for(fixture: &str) -> &'static str {
    match fixture {
        "reply-late-stage-candidate" => "asks for availability",
        "salary-expectation" => "routes compensation to recruiter",
        "missing-resume" => "requests resume upload",
        _ => "matches expected outcome",
    }
}

fn actual_for(fixture: &str, passed: bool) -> &'static str {
    match (fixture, passed) {
        ("reply-late-stage-candidate", false) => "sent generic acknowledgement",
        ("salary-expectation", true) => "routes compensation to recruiter",
        ("missing-resume", true) => "asks for resume upload and confirms deadline",
        _ => "matches expected outcome",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn response_contains_history_and_latest() {
        let (_, body) =
            list_response("wkr_eval", "GET /api/v1/workers/wkr_eval/evals HTTP/1.1").await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["workerId"], "wkr_eval");
        assert_eq!(parsed["history"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["latest"]["decision"], "regressed");
    }

    #[tokio::test]
    async fn fixture_detail_includes_prior_diff() {
        let (_, body) =
            list_response("wkr_eval", "GET /api/v1/workers/wkr_eval/evals HTTP/1.1").await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        let fixtures = parsed["latest"]["fixtures"].as_array().unwrap();
        let regressed = fixtures
            .iter()
            .find(|f| f["fixture"] == "reply-late-stage-candidate")
            .unwrap();
        assert_eq!(regressed["passed"], false);
        assert!(regressed["scoreDelta"].as_f64().unwrap() < 0.0);
    }
}
