//! Judge-vs-human disagreement calibration suite (MISSION_BACKLOG #207).
//!
//! Today the judge gates many decisions (skill creation, persona
//! evolution, broader-role generation) but there is no calibration
//! check for the judge itself. This module loads pre-labelled
//! fixtures (`crates/argo-judge/fixtures/disagreement/*.json`), runs
//! each one through a [`crate::Judge`], and reports the rate of
//! agreement bucketed by tag (clear-allow / clear-block /
//! contentious). Use it to flag drift on model upgrades and to
//! parameter-sweep rubric thresholds.
//!
//! Reproducibility: for an LLM-backed judge, deterministic output is
//! only achievable with `temperature = 0` and a fixed judge model;
//! the report otherwise excludes UUIDs and timestamps so re-running
//! against the same fixtures + same judge produces a byte-identical
//! JSON envelope.
use crate::evidence::{JudgeEvidence, ProposedArtifact};
use crate::judge::Judge;
use argo_types::{ArgoError, ArgoResult, JudgeSubject, JudgeVerdict, SessionId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Default fixtures directory, relative to the workspace root.
pub const DEFAULT_FIXTURES_DIR: &str = "crates/argo-judge/fixtures/disagreement";

/// Tag bucket a fixture belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisagreementTag {
    /// Mundane, low-risk inputs that should be allowed.
    ClearAllow,
    /// Clearly harmful / illegal inputs that should be blocked.
    ClearBlock,
    /// Gray-area inputs (medical / legal / financial advice etc.)
    /// where reasonable humans may disagree.
    Contentious,
}

impl DisagreementTag {
    /// Stable kebab-case label used in reports.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ClearAllow => "clear-allow",
            Self::ClearBlock => "clear-block",
            Self::Contentious => "contentious",
        }
    }
}

/// One pre-labelled disagreement-eval case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeDisagreementCase {
    /// Stable id (`fx-disagree-001`-style).
    pub id: String,
    /// Tag bucket.
    pub tag: DisagreementTag,
    /// Short prompt presented to the judge as the task statement.
    pub input: String,
    /// Reference human verdict: `"allow"`, `"block"`, or `"partial"`.
    pub human_verdict: String,
    /// One-line rationale a labeller would attach to this verdict.
    pub rationale: String,
}

/// Per-bucket agreement summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketBreakdown {
    /// Total cases in the bucket.
    pub total: u32,
    /// Number of cases the judge agreed on.
    pub agreed: u32,
    /// Number of cases the judge disagreed on.
    pub disagreed: u32,
    /// `agreed / total`, or `0.0` if the bucket is empty.
    pub agreement_rate: f32,
}

/// One judge / human disagreement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseMismatch {
    /// Fixture id.
    pub id: String,
    /// Bucket the fixture belongs to.
    pub tag: DisagreementTag,
    /// Short prompt fed to the judge.
    pub input: String,
    /// What the human labelled.
    pub human_verdict: String,
    /// What the judge returned (one of `"allow"`, `"block"`, `"partial"`).
    pub judge_verdict: String,
    /// Human-supplied rationale (from the fixture).
    pub human_rationale: String,
    /// Judge-supplied reason for `partial` / `failure` verdicts.
    pub judge_reason: Option<String>,
}

/// Full disagreement report. Fields are deterministic for a given
/// fixture set + judge + rubric (no UUIDs or timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisagreementReport {
    /// Total fixtures evaluated.
    pub total: u32,
    /// Cases where judge agreed with human.
    pub agreed: u32,
    /// Cases where judge disagreed with human.
    pub disagreed: u32,
    /// `agreed / total`, or `0.0` if no cases.
    pub agreement_rate: f32,
    /// Per-tag breakdown keyed by [`DisagreementTag::label`].
    pub buckets: BTreeMap<String, BucketBreakdown>,
    /// Per-case mismatches, sorted by fixture id for stable output.
    pub mismatches: Vec<CaseMismatch>,
}

impl DisagreementReport {
    /// Did the run clear the operator-supplied agreement floor?
    #[must_use]
    pub fn passes_threshold(&self, min_rate: f32) -> bool {
        self.agreement_rate >= min_rate
    }
}

/// Load all `*.json` fixtures from `dir`, sorted by filename.
///
/// Returns an empty `Vec` if `dir` is missing — useful for CI gates
/// that want to fail-soft when the fixture directory has not been
/// vendored. Per-file parse errors are propagated.
pub fn load_fixtures(dir: &Path) -> ArgoResult<Vec<JudgeDisagreementCase>> {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(ArgoError::Schema(format!(
                "read fixtures dir {}: {e}",
                dir.display()
            )))
        }
    };
    let mut paths: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| ArgoError::Schema(format!("read fixture {}: {e}", path.display())))?;
        let case: JudgeDisagreementCase = serde_json::from_str(&raw)
            .map_err(|e| ArgoError::Schema(format!("parse fixture {}: {e}", path.display())))?;
        out.push(case);
    }
    Ok(out)
}

/// Run every fixture through `judge` and produce a [`DisagreementReport`].
///
/// One [`SessionId`] is minted per run (not per case) so all calls in
/// the same run share a session. The judge sees each case as a
/// `JudgeSubject::SkillCreate` with `name = case.id` and
/// `hash = sha256(case.input)`; evidence is the input string as the
/// task statement with no agent output, tool calls, or ground-truth.
pub async fn run_disagreement_eval(
    judge: &dyn Judge,
    cases: &[JudgeDisagreementCase],
) -> ArgoResult<DisagreementReport> {
    let session = SessionId::new();
    let mut report = DisagreementReport::default();
    let mut per_bucket: BTreeMap<DisagreementTag, (u32, u32)> = BTreeMap::new();
    for case in cases {
        let subject = JudgeSubject::SkillCreate {
            name: case.id.clone(),
            hash: sha256(case.input.as_bytes()),
        };
        let evidence = JudgeEvidence {
            task_statement: case.input.clone(),
            agent_output: String::new(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            proposed_artifact: ProposedArtifact::None,
            ground_truth: Vec::new(),
        };
        let record = judge.adjudicate(session, subject, evidence).await?;
        let (judge_label, judge_reason) = label_of_verdict(&record.verdict);
        let entry = per_bucket.entry(case.tag).or_insert((0, 0));
        entry.0 += 1;
        if judge_label == case.human_verdict {
            report.agreed += 1;
            entry.1 += 1;
        } else {
            report.mismatches.push(CaseMismatch {
                id: case.id.clone(),
                tag: case.tag,
                input: case.input.clone(),
                human_verdict: case.human_verdict.clone(),
                judge_verdict: judge_label.into(),
                human_rationale: case.rationale.clone(),
                judge_reason,
            });
        }
    }
    report.mismatches.sort_by(|a, b| a.id.cmp(&b.id));
    report.total = cases.len() as u32;
    report.disagreed = report.total - report.agreed;
    report.agreement_rate = rate(report.agreed, report.total);
    for (tag, (total, agreed)) in per_bucket {
        report.buckets.insert(
            tag.label().to_string(),
            BucketBreakdown {
                total,
                agreed,
                disagreed: total - agreed,
                agreement_rate: rate(agreed, total),
            },
        );
    }
    Ok(report)
}

fn rate(num: u32, denom: u32) -> f32 {
    if denom == 0 {
        0.0
    } else {
        (num as f32) / (denom as f32)
    }
}

fn label_of_verdict(v: &JudgeVerdict) -> (&'static str, Option<String>) {
    match v {
        JudgeVerdict::Success => ("allow", None),
        JudgeVerdict::Failure { reason } => ("block", Some(reason.clone())),
        JudgeVerdict::Partial { reason } => ("partial", Some(reason.clone())),
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

#[cfg(test)]
#[path = "eval_suite_tests.rs"]
mod tests;
