//! Unit tests for the disagreement-eval suite. The mock judge never
//! makes an LLM call — it returns scripted verdicts so the tests
//! exercise loading, aggregation, bucketing, and report serialization
//! without network I/O.
use super::*;
use argo_types::{
    ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Scoped temp dir for the load-fixtures test that does not require the
/// `tempfile` crate as a dev-dep on `argo-judge`.
struct ScopedTempDir(PathBuf);

impl ScopedTempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("argo-judge-eval-{tag}-{pid}-{n}"));
        std::fs::create_dir_all(&dir).expect("create scoped temp dir");
        Self(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScopedTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Mock judge with a script `id → verdict` keyed by the fixture id
/// (which we propagate through `JudgeSubject::SkillCreate.name`).
struct MockJudge {
    by_id: HashMap<String, JudgeVerdict>,
    fallback: JudgeVerdict,
}

#[async_trait]
impl Judge for MockJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        let id = match &subject {
            JudgeSubject::SkillCreate { name, .. } => name.clone(),
            _ => String::new(),
        };
        let verdict = self
            .by_id
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.fallback.clone());
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict,
            judge_model: ProviderModel {
                provider: "mock".into(),
                model: "mock-judge".into(),
            },
            rubric_id: "mock".into(),
            rubric_version: "0".into(),
            evidence_hashes: evidence.evidence_hashes(),
            created_at: Utc::now(),
        })
    }
}

fn case(id: &str, tag: DisagreementTag, human: &str) -> JudgeDisagreementCase {
    JudgeDisagreementCase {
        id: id.into(),
        tag,
        input: format!("input for {id}"),
        human_verdict: human.into(),
        rationale: format!("rationale for {id}"),
    }
}

fn fail(reason: &str) -> JudgeVerdict {
    JudgeVerdict::Failure { reason: reason.into() }
}

fn partial(reason: &str) -> JudgeVerdict {
    JudgeVerdict::Partial { reason: reason.into() }
}

fn mk_judge(scripts: &[(&str, JudgeVerdict)]) -> MockJudge {
    let by_id = scripts.iter().map(|(k, v)| ((*k).into(), v.clone())).collect();
    MockJudge { by_id, fallback: JudgeVerdict::Success }
}

#[test]
fn load_fixtures_reads_json_files_sorted() {
    let dir = ScopedTempDir::new("load");
    for (i, tag) in ["clear-allow", "clear-block", "contentious"]
        .iter()
        .enumerate()
    {
        let path = dir.path().join(format!("fx-{i:03}.json"));
        std::fs::write(
            &path,
            format!(
                r#"{{"id":"fx-{i:03}","tag":"{tag}","input":"x","human_verdict":"allow","rationale":"r"}}"#
            ),
        )
        .unwrap();
    }
    std::fs::write(dir.path().join("README.txt"), "ignored").unwrap();
    let loaded = load_fixtures(dir.path()).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].id, "fx-000");
    assert_eq!(loaded[1].tag, DisagreementTag::ClearBlock);
}

#[test]
fn load_missing_dir_returns_empty() {
    let missing = std::path::Path::new("/definitely/does/not/exist/argo-disagree");
    let cases = load_fixtures(missing).unwrap();
    assert!(cases.is_empty());
}

#[tokio::test]
async fn run_with_mock_judge_counts_agreement() {
    let cases = vec![
        case("a1", DisagreementTag::ClearAllow, "allow"),
        case("b1", DisagreementTag::ClearBlock, "block"),
        case("c1", DisagreementTag::Contentious, "partial"),
    ];
    let judge = mk_judge(&[
        ("a1", JudgeVerdict::Success),
        ("b1", fail("no")),
        ("c1", partial("gray")),
    ]);
    let report = run_disagreement_eval(&judge, &cases).await.unwrap();
    assert_eq!(report.total, 3);
    assert_eq!(report.agreed, 3);
    assert!(report.mismatches.is_empty());
}

#[tokio::test]
async fn agreement_rate_calculation_handles_partial_run() {
    // 4 cases: judge agrees on 3, disagrees on 1 → 75%.
    let cases = vec![
        case("a1", DisagreementTag::ClearAllow, "allow"),
        case("a2", DisagreementTag::ClearAllow, "allow"),
        case("b1", DisagreementTag::ClearBlock, "block"),
        case("b2", DisagreementTag::ClearBlock, "block"),
    ];
    let judge = mk_judge(&[
        ("a1", JudgeVerdict::Success),
        ("a2", JudgeVerdict::Success),
        ("b1", fail("no")),
        ("b2", JudgeVerdict::Success), // miss
    ]);
    let report = run_disagreement_eval(&judge, &cases).await.unwrap();
    assert_eq!(report.agreed, 3);
    assert_eq!(report.disagreed, 1);
    assert!((report.agreement_rate - 0.75).abs() < 1e-6);
    assert_eq!(report.mismatches.len(), 1);
    assert_eq!(report.mismatches[0].id, "b2");
    assert_eq!(report.mismatches[0].judge_verdict, "allow");
    assert_eq!(report.mismatches[0].human_verdict, "block");
}

#[tokio::test]
async fn bucketed_breakdown_reports_per_tag() {
    let cases = vec![
        case("a1", DisagreementTag::ClearAllow, "allow"),
        case("a2", DisagreementTag::ClearAllow, "allow"),
        case("b1", DisagreementTag::ClearBlock, "block"),
        case("c1", DisagreementTag::Contentious, "partial"),
        case("c2", DisagreementTag::Contentious, "block"),
    ];
    let judge = mk_judge(&[
        ("a1", JudgeVerdict::Success),
        ("a2", fail("x")),
        ("b1", fail("x")),
        ("c1", partial("y")),
        ("c2", partial("y")),
    ]);
    let report = run_disagreement_eval(&judge, &cases).await.unwrap();
    let allow = report.buckets.get("clear-allow").unwrap();
    assert_eq!((allow.total, allow.agreed), (2, 1));
    let block = report.buckets.get("clear-block").unwrap();
    assert_eq!((block.total, block.agreed), (1, 1));
    let cont = report.buckets.get("contentious").unwrap();
    assert_eq!((cont.total, cont.agreed), (2, 1));
}

#[tokio::test]
async fn passes_threshold_gates_on_agreement_rate() {
    let cases = vec![
        case("a1", DisagreementTag::ClearAllow, "allow"),
        case("a2", DisagreementTag::ClearAllow, "allow"),
    ];
    let judge = mk_judge(&[("a1", JudgeVerdict::Success), ("a2", fail("x"))]);
    let report = run_disagreement_eval(&judge, &cases).await.unwrap();
    assert!(!report.passes_threshold(0.85));
    assert!(report.passes_threshold(0.4));
}

#[tokio::test]
async fn json_envelope_is_stable_across_runs() {
    let cases = vec![
        case("a1", DisagreementTag::ClearAllow, "allow"),
        case("b1", DisagreementTag::ClearBlock, "block"),
    ];
    let judge = mk_judge(&[("a1", JudgeVerdict::Success), ("b1", fail("no"))]);
    let r1 = run_disagreement_eval(&judge, &cases).await.unwrap();
    let r2 = run_disagreement_eval(&judge, &cases).await.unwrap();
    let j1 = serde_json::to_string(&r1).unwrap();
    let j2 = serde_json::to_string(&r2).unwrap();
    assert_eq!(j1, j2);
    let parsed: DisagreementReport = serde_json::from_str(&j1).unwrap();
    assert_eq!(parsed.total, r1.total);
}
