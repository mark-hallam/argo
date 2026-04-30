use super::*;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::Path;

// ── metrics ──────────────────────────────────────────────────

#[test]
fn exact_match_scores_one_or_zero() {
    let t = EvalTask {
        name: "x".into(),
        input: "i".into(),
        expected: "hello".into(),
        metric: EvalMetric::ExactMatch,
    };
    assert!((t.score("hello") - 1.0).abs() < 1e-9);
    assert!((t.score("world") - 0.0).abs() < 1e-9);
}

#[test]
fn substring_contains_requires_all_substrings() {
    let t = EvalTask {
        name: "x".into(),
        input: "i".into(),
        expected: "ignored".into(),
        metric: EvalMetric::SubstringContains {
            must_contain: vec!["foo".into(), "bar".into()],
        },
    };
    assert!((t.score("foo and bar") - 1.0).abs() < 1e-9);
    assert!((t.score("foo only") - 0.0).abs() < 1e-9);
    assert!((t.score("bar only") - 0.0).abs() < 1e-9);
}

#[test]
fn jaccard_tokens_in_unit_interval() {
    let t = EvalTask {
        name: "x".into(),
        input: "i".into(),
        expected: "the quick brown fox".into(),
        metric: EvalMetric::JaccardTokens,
    };
    let perfect = t.score("the quick brown fox");
    let none = t.score("zzz");
    let half = t.score("the quick");
    assert!((perfect - 1.0).abs() < 1e-9);
    assert!((none - 0.0).abs() < 1e-9);
    assert!(half > 0.0 && half < 1.0);
}

// ── jsonl loading ────────────────────────────────────────────

async fn write(p: &Path, body: &str) {
    tokio::fs::write(p, body).await.unwrap();
}

#[tokio::test]
async fn load_jsonl_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("tasks.jsonl");
    let line = serde_json::to_string(&EvalTask {
        name: "core/add".into(),
        input: "2+2".into(),
        expected: "4".into(),
        metric: EvalMetric::ExactMatch,
    })
    .unwrap();
    write(&p, &format!("{line}\n")).await;
    let tasks = load_jsonl(&p).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "core/add");
}

#[tokio::test]
async fn load_jsonl_skips_blank_lines() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("tasks.jsonl");
    let line = serde_json::to_string(&EvalTask {
        name: "x".into(),
        input: "i".into(),
        expected: "o".into(),
        metric: EvalMetric::ExactMatch,
    })
    .unwrap();
    write(&p, &format!("\n{line}\n\n   \n")).await;
    let tasks = load_jsonl(&p).await.unwrap();
    assert_eq!(tasks.len(), 1);
}

#[tokio::test]
async fn load_jsonl_fails_loudly_on_bad_line() {
    // §3: a half-loaded eval set is dangerous; we surface a parse
    // error instead of silently dropping.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("tasks.jsonl");
    write(&p, "{not valid json\n").await;
    let err = load_jsonl(&p).await.unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

// ── runner ───────────────────────────────────────────────────

struct EchoExpected;

#[async_trait]
impl EvalSubject for EchoExpected {
    async fn run(&self, task: &EvalTask) -> ArgoResult<String> {
        Ok(task.expected.clone())
    }
}

struct AlwaysWrong;

#[async_trait]
impl EvalSubject for AlwaysWrong {
    async fn run(&self, _task: &EvalTask) -> ArgoResult<String> {
        Ok("nope".into())
    }
}

struct AlwaysErrors;

#[async_trait]
impl EvalSubject for AlwaysErrors {
    async fn run(&self, _task: &EvalTask) -> ArgoResult<String> {
        Err(ArgoError::Provider("synthetic".into()))
    }
}

fn three_tasks() -> Vec<EvalTask> {
    vec![
        EvalTask {
            name: "a".into(),
            input: "i".into(),
            expected: "x".into(),
            metric: EvalMetric::ExactMatch,
        },
        EvalTask {
            name: "b".into(),
            input: "i".into(),
            expected: "y".into(),
            metric: EvalMetric::ExactMatch,
        },
        EvalTask {
            name: "c".into(),
            input: "i".into(),
            expected: "z".into(),
            metric: EvalMetric::ExactMatch,
        },
    ]
}

#[tokio::test]
async fn run_eval_perfect_subject_scores_one() {
    let (records, summary) = run_eval(&EchoExpected, &three_tasks()).await;
    assert_eq!(records.len(), 3);
    assert!((summary.mean_score - 1.0).abs() < 1e-9);
    assert!((summary.pass_rate - 1.0).abs() < 1e-9);
    assert_eq!(summary.errors, 0);
}

#[tokio::test]
async fn run_eval_always_wrong_scores_zero() {
    let (_, summary) = run_eval(&AlwaysWrong, &three_tasks()).await;
    assert!((summary.mean_score - 0.0).abs() < 1e-9);
    assert!((summary.pass_rate - 0.0).abs() < 1e-9);
}

#[tokio::test]
async fn run_eval_subject_errors_count_separately() {
    let (records, summary) = run_eval(&AlwaysErrors, &three_tasks()).await;
    assert_eq!(records.len(), 3);
    assert_eq!(summary.errors, 3);
    // Score and pass-rate are 0 (errors count as zero-score records).
    assert!((summary.mean_score - 0.0).abs() < 1e-9);
}

#[tokio::test]
async fn empty_task_set_has_zero_summary() {
    let (records, summary) = run_eval(&EchoExpected, &[]).await;
    assert!(records.is_empty());
    assert_eq!(summary.n, 0);
}
