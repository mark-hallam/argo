//! Unit tests for the honesty module.
//!
//! Eight cases, one per acceptance bullet:
//! 1. not-enough-history (history < min_history_count)
//! 2. perfect-agreement-honest (delta = 0)
//! 3. high-disagreement-suspicious (delta well above threshold)
//! 4. threshold-boundary (delta exactly at threshold → suspicious)
//! 5. empty-audit-NotEnoughHistory (count above min but sampler yields zero)
//! 6. missing-self-reported-skipped (sampler drops malformed rows)
//! 7. sample-size-respected (cap stays at HonestyConfig::sample_size)
//! 8. judge-failure-falls-through (judge errors don't fabricate Suspicious)

use super::*;
use argo_judge::{Judge, JudgeEvidence};
use argo_types::{
    ArgoError, ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel,
    SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Configurable in-memory `AuditSampler` for the test matrix.
struct FixtureSampler {
    total: usize,
    samples: Vec<SkillSample>,
    capture_limit: AtomicUsize,
}

impl FixtureSampler {
    fn new(total: usize, samples: Vec<SkillSample>) -> Self {
        Self {
            total,
            samples,
            capture_limit: AtomicUsize::new(usize::MAX),
        }
    }
}

#[async_trait]
impl AuditSampler for FixtureSampler {
    async fn total_count(&self, _skill_id: &str) -> ArgoResult<usize> {
        Ok(self.total)
    }

    async fn sample(&self, _skill_id: &str, limit: usize) -> ArgoResult<Vec<SkillSample>> {
        self.capture_limit.store(limit, Ordering::SeqCst);
        let mut out = self.samples.clone();
        out.truncate(limit);
        Ok(out)
    }
}

/// Maps `agent_output` markers to verdicts. `"OK"` → Success; `"BAD"` →
/// Failure. Records can include `"ERR"` to make the judge return Err
/// (exercised in the fall-through case).
struct ScriptedJudge;

#[async_trait]
impl Judge for ScriptedJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        let verdict = match evidence.agent_output.as_str() {
            "ERR" => {
                return Err(ArgoError::Provider("scripted judge failure".into()));
            }
            "OK" => JudgeVerdict::Success,
            _ => JudgeVerdict::Failure {
                reason: "scripted bad output".into(),
            },
        };
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict,
            judge_model: ProviderModel {
                provider: "test".into(),
                model: "scripted".into(),
            },
            rubric_id: "test".into(),
            rubric_version: "0".into(),
            evidence_hashes: vec![],
            created_at: Utc::now(),
        })
    }
}

fn mk_sample(claim: bool, output: &str) -> SkillSample {
    SkillSample {
        session_id: SessionId::new(),
        task_statement: "do x".into(),
        agent_output: output.into(),
        self_reported_success: claim,
    }
}

#[tokio::test]
async fn not_enough_history_returns_early() {
    // 10 audit rows vs default min_history_count=50.
    let sampler = FixtureSampler::new(10, vec![]);
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, HonestyConfig::default())
        .await
        .unwrap();
    match res {
        HonestyResult::NotEnoughHistory {
            available,
            required,
        } => {
            assert_eq!(available, 10);
            assert_eq!(required, 50);
        }
        other => panic!("expected NotEnoughHistory, got {other:?}"),
    }
}

#[tokio::test]
async fn perfect_agreement_yields_honest() {
    // 20 samples, all self_reported=true and agent_output="OK" → judge
    // also returns Success → delta = 0 → Honest.
    let samples: Vec<SkillSample> = (0..20).map(|_| mk_sample(true, "OK")).collect();
    let sampler = FixtureSampler::new(60, samples);
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, HonestyConfig::default())
        .await
        .unwrap();
    match res {
        HonestyResult::Honest {
            self_reported,
            observed,
            delta,
            breakdown,
        } => {
            assert!((self_reported - 1.0).abs() < 1e-6);
            assert!((observed - 1.0).abs() < 1e-6);
            assert!(delta.abs() < 1e-6);
            assert_eq!(breakdown.len(), 20);
            assert!(breakdown.iter().all(|v| !v.disagreement));
        }
        other => panic!("expected Honest, got {other:?}"),
    }
}

#[tokio::test]
async fn high_disagreement_yields_suspicious() {
    // 20 samples, all claim success but every output is "BAD" → judge
    // says Failure on every one → delta = 1.0.
    let samples: Vec<SkillSample> = (0..20).map(|_| mk_sample(true, "BAD")).collect();
    let sampler = FixtureSampler::new(60, samples);
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, HonestyConfig::default())
        .await
        .unwrap();
    match res {
        HonestyResult::Suspicious {
            self_reported,
            observed,
            delta,
            sample_size,
            breakdown,
        } => {
            assert!((self_reported - 1.0).abs() < 1e-6);
            assert!(observed.abs() < 1e-6);
            assert!((delta - 1.0).abs() < 1e-6);
            assert_eq!(sample_size, 20);
            assert_eq!(breakdown.len(), 20);
            assert!(breakdown.iter().all(|v| v.disagreement));
        }
        other => panic!("expected Suspicious, got {other:?}"),
    }
}

#[tokio::test]
async fn disagreement_at_threshold_is_suspicious() {
    // 10 samples; 3 disagree → delta = 0.30 (exact threshold). The
    // comparison is `>=` so this rolls Suspicious.
    let mut samples = Vec::new();
    // 3 disagreements: claim=true, judge=fail
    for _ in 0..3 {
        samples.push(mk_sample(true, "BAD"));
    }
    // 7 agreements: claim=true, judge=ok
    for _ in 0..7 {
        samples.push(mk_sample(true, "OK"));
    }
    let sampler = FixtureSampler::new(60, samples);
    let cfg = HonestyConfig {
        sample_size: 10,
        ..HonestyConfig::default()
    };
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, cfg)
        .await
        .unwrap();
    match res {
        HonestyResult::Suspicious {
            delta, sample_size, ..
        } => {
            assert_eq!(sample_size, 10);
            assert!(
                (delta - 0.30).abs() < 1e-6,
                "expected delta ~0.30, got {delta}"
            );
        }
        other => panic!("expected Suspicious at threshold, got {other:?}"),
    }
}

#[tokio::test]
async fn empty_audit_returns_not_enough_history() {
    // total_count clears the gate but the sampler returns zero rows
    // (e.g. all rows missing the convention). Should still surface
    // NotEnoughHistory rather than dividing by zero.
    let sampler = FixtureSampler::new(60, vec![]);
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, HonestyConfig::default())
        .await
        .unwrap();
    assert!(matches!(res, HonestyResult::NotEnoughHistory { .. }));
}

#[tokio::test]
async fn malformed_rows_skipped_by_sampler() {
    // Sampler convention: the sampler does the filtering, so we only
    // see qualifying rows here. Simulate that: total=100 (lots of audit
    // rows in raw DB) but only 10 carry the full self-report
    // convention. The result reflects those 10.
    let samples: Vec<SkillSample> = (0..10).map(|_| mk_sample(true, "OK")).collect();
    let sampler = FixtureSampler::new(100, samples);
    let cfg = HonestyConfig {
        sample_size: 50,
        ..HonestyConfig::default()
    };
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, cfg)
        .await
        .unwrap();
    match res {
        HonestyResult::Honest { breakdown, .. } => {
            // We didn't synthesize extra rows; the sampler drops the
            // malformed ones before they reach us.
            assert_eq!(breakdown.len(), 10);
        }
        other => panic!("expected Honest, got {other:?}"),
    }
}

#[tokio::test]
async fn sample_size_cap_passed_to_sampler() {
    // Verify cfg.sample_size is the `limit` the sampler is asked for.
    let samples: Vec<SkillSample> = (0..30).map(|_| mk_sample(true, "OK")).collect();
    let sampler = FixtureSampler::new(60, samples);
    let cfg = HonestyConfig {
        sample_size: 7,
        ..HonestyConfig::default()
    };
    let _ = run_honesty_test("s", &sampler, &ScriptedJudge, cfg)
        .await
        .unwrap();
    assert_eq!(sampler.capture_limit.load(Ordering::SeqCst), 7);
}

#[tokio::test]
async fn judge_failure_falls_through_as_agreement() {
    // 20 samples; on every one the judge errors. Fall-through should
    // treat the judge's missing verdict as matching the self-report,
    // so delta = 0 → Honest (NOT a spurious Suspicious).
    let samples: Vec<SkillSample> = (0..20).map(|_| mk_sample(true, "ERR")).collect();
    let sampler = FixtureSampler::new(60, samples);
    let res = run_honesty_test("s", &sampler, &ScriptedJudge, HonestyConfig::default())
        .await
        .unwrap();
    match res {
        HonestyResult::Honest {
            delta, breakdown, ..
        } => {
            assert!(delta.abs() < 1e-6, "expected delta=0, got {delta}");
            assert!(breakdown.iter().all(|v| !v.disagreement));
        }
        other => panic!("expected Honest fall-through, got {other:?}"),
    }
}
