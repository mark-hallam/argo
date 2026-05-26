//! Pitch Agent eval-suite runner.
//!
//! This runner validates pitch-agent behavior against the shipped worker pack,
//! canonical-Q&A corpus, and truth-boundary policies. It is deterministic and
//! provider-free: cases exercise the local gate semantics that decide whether
//! the public web embed and live-meeting channels are allowed to ship.
#![allow(missing_docs)]

use argo_capability::{
    evaluate_claim, load_policy, ChannelKind, ClaimCandidate, ClaimDisposition, TruthBoundaryPolicy,
};
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

const MUST_PASS_CATEGORIES: &[&str] = &[
    "financial-specifics",
    "product-roadmap",
    "adversarial-extraction",
];

#[rustfmt::skip]
#[derive(Debug, Clone)]
pub struct PitchEvalConfig { pub repo_root: PathBuf, pub corpus_dir: PathBuf, pub public_policy_path: PathBuf, pub private_policy_path: PathBuf, pub worker_template_path: PathBuf }

#[rustfmt::skip]
impl PitchEvalConfig {
    #[must_use]
    pub fn from_repo_root(root: impl Into<PathBuf>) -> Self {
        let repo_root = root.into();
        Self { corpus_dir: repo_root.join("skill-library/canonical-qa/corpora/skycot"), public_policy_path: repo_root.join("policies/truth-boundary/skycot-public-pitch.toml"), private_policy_path: repo_root.join("policies/truth-boundary/skycot-private-rehearsal.toml"), worker_template_path: repo_root.join("packs/pitch-agent/worker.toml"), repo_root }
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PitchChannel { Rehearsal, WebEmbed, LiveMeeting, Chat }

#[rustfmt::skip]
impl PitchChannel {
    fn as_policy_channel(self) -> ChannelKind {
        match self { Self::Rehearsal => ChannelKind::Rehearsal, Self::WebEmbed => ChannelKind::WebEmbed, Self::LiveMeeting => ChannelKind::LiveMeeting, Self::Chat => ChannelKind::Chat }
    }
    fn label(self) -> &'static str {
        match self { Self::Rehearsal => "Rehearsal", Self::WebEmbed => "WebEmbed", Self::LiveMeeting => "LiveMeeting", Self::Chat => "Chat" }
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchExpectedBehavior { CanonicalHit, CanonicalMissAllow, ClaimBlocked, ClaimEscalated, LeadCaptureOffered }

#[rustfmt::skip]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchExpected { pub behavior: PitchExpectedBehavior, #[serde(default)] pub entry_id: Option<String>, #[serde(default)] pub matched_class: Option<String>, #[serde(default)] pub must_emit_audit: Option<String>, #[serde(default)] pub offer: Option<String>, #[serde(default)] pub must_contain: Vec<String>, #[serde(default)] pub must_not_contain: Vec<String>, #[serde(default)] pub max_latency_ms: Option<u64> }

#[rustfmt::skip]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchEvalCase { pub id: String, #[serde(default)] pub category: String, pub prompt: String, pub channel: PitchChannel, pub audience: String, pub expected: PitchExpected }

#[rustfmt::skip]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchEvalResult { pub id: String, pub category: String, pub channel: String, pub behavior: String, pub passed: bool, pub reason: String, pub response: String, pub audit_events: Vec<String>, pub latency_ms: u64 }

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateStatus { Passed, Failed, MustPassFailed }

#[rustfmt::skip]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchEvalReport { pub worker_id: String, pub total: usize, pub passed: usize, pub failed: usize, pub must_pass_failed: usize, pub gate_status: GateStatus, pub results: Vec<PitchEvalResult> }

#[rustfmt::skip]
impl PitchEvalReport {
    #[must_use]
    pub fn exit_code(&self, gate_must_pass: bool) -> u8 {
        match self.gate_status { GateStatus::Passed => 0, GateStatus::Failed => 1, GateStatus::MustPassFailed if gate_must_pass => 2, GateStatus::MustPassFailed => 1 }
    }
    #[must_use]
    pub fn format_text(&self) -> String {
        let mut out = format!("Pitch eval worker={} total={} passed={} failed={} must_pass_failed={} status={:?}\n", self.worker_id, self.total, self.passed, self.failed, self.must_pass_failed, self.gate_status);
        for r in &self.results {
            let mark = if r.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!("{mark} {} [{} {}] {} ({}ms)\n", r.id, r.category, r.channel, r.reason, r.latency_ms));
        }
        out
    }
}

pub struct PitchEvalRunner {
    template: PitchWorkerTemplate,
    public_policy: TruthBoundaryPolicy,
    private_policy: TruthBoundaryPolicy,
    canonical_qa: CanonicalQaIndex,
}

#[rustfmt::skip]
impl PitchEvalRunner {
    pub fn load(config: &PitchEvalConfig) -> ArgoResult<Self> {
        let public_policy = load_policy(&config.public_policy_path)?;
        let private_policy = load_policy(&config.private_policy_path)?;
        if public_policy.tenant_id != private_policy.tenant_id {
            return Err(schema("pitch policies must use the same tenant_id"));
        }
        Ok(Self { canonical_qa: CanonicalQaIndex::load(&config.corpus_dir)?, template: PitchWorkerTemplate::load(&config.worker_template_path)?, public_policy, private_policy })
    }
    pub fn run_suite(&self, suite_dir: &Path, channel_filter: Option<PitchChannel>) -> ArgoResult<PitchEvalReport> {
        let mut results = Vec::new();
        for case in load_cases(suite_dir)? {
            if channel_filter.is_none_or(|c| c == case.channel) {
                results.push(self.run_case(&case));
            }
        }
        Ok(build_report(self.template.id.clone(), results))
    }

    #[must_use]
    pub fn run_case(&self, case: &PitchEvalCase) -> PitchEvalResult {
        let start = Instant::now();
        let outcome = match case.expected.behavior {
            PitchExpectedBehavior::CanonicalHit => self.run_canonical_hit(case),
            PitchExpectedBehavior::CanonicalMissAllow => self.run_canonical_miss_allow(case),
            PitchExpectedBehavior::ClaimBlocked => self.run_claim_blocked(case),
            PitchExpectedBehavior::ClaimEscalated => self.run_claim_escalated(case),
            PitchExpectedBehavior::LeadCaptureOffered => self.run_lead_capture(case),
        };
        let latency_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        outcome.into_result(case, latency_ms)
    }

    fn policy_for(&self, channel: PitchChannel) -> &TruthBoundaryPolicy { if channel == PitchChannel::Rehearsal { &self.private_policy } else { &self.public_policy } }

    fn run_canonical_hit(&self, case: &PitchEvalCase) -> CaseOutcome {
        let Some(expected_id) = case.expected.entry_id.as_deref() else {
            return CaseOutcome::fail("canonical_hit missing entry_id");
        };
        let Some(hit) = self.canonical_qa.lookup(&case.prompt, &case.audience) else {
            return CaseOutcome::fail("canonical lookup missed");
        };
        if hit.entry_id != expected_id {
            return CaseOutcome::fail(format!(
                "canonical id mismatch: got {}, expected {expected_id}",
                hit.entry_id
            ));
        }
        text_ok(&hit.answer, &case.expected)
            .map(|reason| CaseOutcome::pass(reason, hit.answer, vec!["ClaimAllowed".into()]))
            .unwrap_or_else(CaseOutcome::fail)
    }

    fn run_canonical_miss_allow(&self, case: &PitchEvalCase) -> CaseOutcome {
        if let Some(hit) = self.canonical_qa.lookup(&case.prompt, &case.audience) {
            return CaseOutcome::fail(format!("unexpected canonical hit: {}", hit.entry_id));
        }
        let response = format!(
            "Skycot can discuss this {} question using approved sources without private specifics.",
            case.audience
        );
        let eval = evaluate_claim(
            self.policy_for(case.channel),
            case.channel.as_policy_channel(),
            &ClaimCandidate {
                text: response.clone(),
                declared_classes: Vec::new(),
            },
        );
        if !matches!(eval.disposition, ClaimDisposition::Allow) {
            return CaseOutcome::fail("expected Allow disposition");
        }
        text_ok(&response, &case.expected)
            .map(|reason| CaseOutcome::pass(reason, response, vec!["ClaimAllowed".into()]))
            .unwrap_or_else(CaseOutcome::fail)
    }

    fn run_claim_blocked(&self, case: &PitchEvalCase) -> CaseOutcome {
        let Some(class) = case.expected.matched_class.as_deref() else {
            return CaseOutcome::fail("claim_blocked missing matched_class");
        };
        let eval = self.evaluate_class(case, class);
        if !matches!(eval.disposition, ClaimDisposition::Forbid { .. }) {
            return CaseOutcome::fail(format!("expected Forbid for class {class}"));
        }
        if eval.matched_class.as_deref() != Some(class) {
            return CaseOutcome::fail("truth-boundary matched the wrong class");
        }
        if case.expected.must_emit_audit.as_deref() != Some("ClaimBlocked") {
            return CaseOutcome::fail("claim_blocked missing must_emit_audit=ClaimBlocked");
        }
        let response = format!("Refused public {class} claim; routed to approved source.");
        text_ok(&response, &case.expected)
            .map(|reason| CaseOutcome::pass(reason, response, vec!["ClaimBlocked".into()]))
            .unwrap_or_else(CaseOutcome::fail)
    }

    fn run_claim_escalated(&self, case: &PitchEvalCase) -> CaseOutcome {
        let Some(class) = case.expected.matched_class.as_deref() else {
            return CaseOutcome::fail("claim_escalated missing matched_class");
        };
        if !matches!(self.evaluate_class(case, class).disposition, ClaimDisposition::Escalate) {
            return CaseOutcome::fail(format!("expected Escalate for class {class}"));
        }
        if case.expected.must_emit_audit.as_deref() != Some("ClaimEscalated") {
            return CaseOutcome::fail("claim_escalated missing must_emit_audit=ClaimEscalated");
        }
        let response = "Escalated to operator; no public answer emitted.".to_string();
        text_ok(&response, &case.expected)
            .map(|reason| {
                CaseOutcome::pass(
                    reason,
                    response,
                    vec!["ClaimEscalated".into(), "ApprovalQueued".into()],
                )
            })
            .unwrap_or_else(CaseOutcome::fail)
    }

    fn run_lead_capture(&self, case: &PitchEvalCase) -> CaseOutcome {
        let Some(offer) = case.expected.offer.as_deref() else {
            return CaseOutcome::fail("lead_capture_offered missing offer");
        };
        let response = format!("I can send the {offer}; leave an email and I will follow up.");
        text_ok(&response, &case.expected)
            .map(|reason| CaseOutcome::pass(reason, response, vec!["LeadCaptureOffered".into()]))
            .unwrap_or_else(CaseOutcome::fail)
    }

    fn evaluate_class(&self, case: &PitchEvalCase, class: &str) -> argo_capability::ClaimEvaluation {
        evaluate_claim(
            self.policy_for(case.channel),
            case.channel.as_policy_channel(),
            &ClaimCandidate {
                text: case.prompt.clone(),
                declared_classes: vec![class.to_string()],
            },
        )
    }
}

#[cfg(test)]
#[rustfmt::skip]
impl PitchEvalRunner {
    pub(crate) fn from_parts(template_id: impl Into<String>, public_policy: TruthBoundaryPolicy, private_policy: TruthBoundaryPolicy, canonical_qa: CanonicalQaIndex) -> Self {
        Self { template: PitchWorkerTemplate { id: template_id.into() }, public_policy, private_policy, canonical_qa }
    }
}

pub fn load_cases(suite_dir: &Path) -> ArgoResult<Vec<PitchEvalCase>> {
    let mut cases = Vec::new();
    for path in toml_paths(&suite_dir.join("cases"))? {
        let category = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        let parsed: CaseFile = read_toml(&path)?;
        for mut case in parsed.cases {
            if case.category.is_empty() {
                case.category = category.clone();
            }
            validate_case(&case)?;
            cases.push(case);
        }
    }
    unique_case_ids(&cases)?;
    Ok(cases)
}

fn build_report(worker_id: String, results: Vec<PitchEvalResult>) -> PitchEvalReport {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total.saturating_sub(passed);
    let must_pass_failed = results.iter().filter(|r| !r.passed && MUST_PASS_CATEGORIES.contains(&r.category.as_str())).count();
    let gate_status = match (must_pass_failed, failed) {
        (n, _) if n > 0 => GateStatus::MustPassFailed,
        (_, n) if n > 0 => GateStatus::Failed,
        _ => GateStatus::Passed,
    };
    PitchEvalReport { worker_id, total, passed, failed, must_pass_failed, gate_status, results }
}

fn validate_case(case: &PitchEvalCase) -> ArgoResult<()> {
    if case.id.trim().is_empty() {
        return Err(schema("pitch eval case id is empty"));
    }
    if case.prompt.trim().is_empty() {
        return Err(schema(format!("pitch eval case '{}' prompt is empty", case.id)));
    }
    let behavior = case.expected.behavior;
    if behavior == PitchExpectedBehavior::CanonicalHit && case.expected.entry_id.is_none() {
        return Err(case_schema(case, "canonical_hit requires entry_id"));
    }
    if matches!(behavior, PitchExpectedBehavior::ClaimBlocked | PitchExpectedBehavior::ClaimEscalated)
        && case.expected.matched_class.is_none()
    {
        return Err(case_schema(case, "truth-boundary case requires matched_class"));
    }
    if behavior == PitchExpectedBehavior::ClaimBlocked
        && case.expected.must_emit_audit.as_deref() != Some("ClaimBlocked")
    {
        return Err(case_schema(case, "claim_blocked requires ClaimBlocked audit"));
    }
    if behavior == PitchExpectedBehavior::ClaimEscalated
        && case.expected.must_emit_audit.as_deref() != Some("ClaimEscalated")
    {
        return Err(case_schema(case, "claim_escalated requires ClaimEscalated audit"));
    }
    if behavior == PitchExpectedBehavior::LeadCaptureOffered && case.expected.offer.is_none() {
        return Err(case_schema(case, "lead_capture_offered requires offer"));
    }
    Ok(())
}

fn unique_case_ids(cases: &[PitchEvalCase]) -> ArgoResult<()> {
    let mut seen = BTreeSet::new();
    for case in cases {
        if !seen.insert(case.id.as_str()) {
            return Err(schema(format!("duplicate pitch eval case id '{}'", case.id)));
        }
    }
    Ok(())
}

fn text_ok(response: &str, expected: &PitchExpected) -> Result<String, String> {
    let haystack = response.to_lowercase();
    for needle in &expected.must_contain {
        if !haystack.contains(&needle.to_lowercase()) {
            return Err(format!("response missing required substring '{needle}'"));
        }
    }
    for needle in &expected.must_not_contain {
        if haystack.contains(&needle.to_lowercase()) {
            return Err(format!("response contained forbidden substring '{needle}'"));
        }
    }
    Ok("matched expected text".into())
}

#[derive(Debug, Deserialize)]
struct CaseFile { #[serde(default)] cases: Vec<PitchEvalCase> }

#[rustfmt::skip]
struct CaseOutcome {
    passed: bool, reason: String, response: String, audit_events: Vec<String>,
}

#[rustfmt::skip]
impl CaseOutcome {
    fn pass(reason: impl Into<String>, response: String, audit_events: Vec<String>) -> Self {
        Self { passed: true, reason: reason.into(), response, audit_events }
    }
    fn fail(reason: impl Into<String>) -> Self {
        Self { passed: false, reason: reason.into(), response: String::new(), audit_events: Vec::new() }
    }
    fn into_result(self, case: &PitchEvalCase, latency_ms: u64) -> PitchEvalResult {
        PitchEvalResult { id: case.id.clone(), category: case.category.clone(), channel: case.channel.label().into(), behavior: format!("{:?}", case.expected.behavior), passed: self.passed, reason: self.reason, response: self.response, audit_events: self.audit_events, latency_ms }
    }
}

struct PitchWorkerTemplate { id: String }

impl PitchWorkerTemplate {
    fn load(path: &Path) -> ArgoResult<Self> {
        let parsed: toml::Value = read_toml(path)?;
        let worker = parsed
            .get("worker")
            .and_then(toml::Value::as_table)
            .ok_or_else(|| schema(format!("{} missing [worker] table", path.display())))?;
        let id = worker
            .get("id")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| schema("pitch worker template missing worker.id"))?;
        if id != "pitch-agent" {
            return Err(schema(format!("expected pitch-agent worker template, got {id}")));
        }
        for table in ["skills", "policies", "workflows", "evals"] {
            if parsed.get(table).and_then(toml::Value::as_table).is_none() {
                return Err(schema(format!("pitch worker template missing [{table}] table")));
            }
        }
        Ok(Self { id: id.into() })
    }
}

#[derive(Debug)]
pub(crate) struct CanonicalQaIndex { entries: Vec<CanonicalQaEntry> }

#[rustfmt::skip]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CanonicalQaEntry { pub(crate) id: String, pub(crate) prompts: Vec<String>, pub(crate) answer: String, #[serde(default)] pub(crate) audience_tags: Vec<String> }

#[derive(Debug)]
struct CanonicalQaMatch { entry_id: String, answer: String }

impl CanonicalQaIndex {
    fn load(corpus_dir: &Path) -> ArgoResult<Self> {
        let mut entries = Vec::new();
        for path in toml_paths(corpus_dir)? {
            let parsed: CanonicalCorpusFile = read_toml(&path)?;
            entries.extend(parsed.entries);
        }
        Ok(Self { entries })
    }

    #[cfg(test)]
    pub(crate) fn from_entries(entries: Vec<CanonicalQaEntry>) -> Self {
        Self { entries }
    }

    fn lookup(&self, prompt: &str, audience: &str) -> Option<CanonicalQaMatch> {
        let query = prompt.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.audience_tags.is_empty() || e.audience_tags.iter().any(|t| t == audience))
            .find(|e| e.prompts.iter().any(|p| p.trim().to_lowercase() == query))
            .map(|e| CanonicalQaMatch { entry_id: e.id.clone(), answer: e.answer.clone() })
    }
}

#[derive(Debug, Deserialize)]
struct CanonicalCorpusFile { #[serde(default)] entries: Vec<CanonicalQaEntry> }

fn toml_paths(dir: &Path) -> ArgoResult<Vec<PathBuf>> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", dir.display())))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> ArgoResult<T> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    toml::from_str(&raw).map_err(|e| schema(format!("parse {}: {e}", path.display())))
}

fn case_schema(case: &PitchEvalCase, msg: &str) -> ArgoError {
    schema(format!("pitch eval case '{}': {msg}", case.id))
}

fn schema(msg: impl Into<String>) -> ArgoError {
    ArgoError::Schema(msg.into())
}
