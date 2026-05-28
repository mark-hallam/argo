//! `POST /api/v1/pitch/ask` — Pitch Agent operator-facing endpoint.
//!
//! Mirrors the proven `crates/argo-eval/src/pitch_runner.rs` pipeline
//! that drives the 111-case pitch-eval suite: load the canonical-Q&A
//! corpus once at process boot, look up the question against the
//! in-memory IDF-weighted lexical index, then push any declared claim
//! classes through `argo_capability::evaluate_claim` against the
//! channel-appropriate truth-boundary policy.
//!
//! The endpoint is deterministic — no LLM call is made on this path.
//! Every question lands as exactly one audit row, either
//! [`AuditKind::PitchCanonicalQaMatched`] or
//! [`AuditKind::PitchCanonicalQaMiss`], so the demo `/audit` surface
//! shows the operator's question history live alongside every other
//! gateway event.

use super::{rate_limit, routes_auth_principal};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{
    evaluate_claim, load_policy, ChannelKind, ClaimCandidate, ClaimDisposition, TruthBoundaryPolicy,
};
use argo_skill::canonical_qa::CanonicalQaIndex;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use uuid::Uuid;

/// Channel selector mirrored from `argo_eval::PitchChannel`. Kept here
/// so the gateway parses the wire-shape without pulling argo-eval into
/// the gateway dep graph.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum PitchChannel {
    Rehearsal,
    WebEmbed,
    LiveMeeting,
    Chat,
}

impl PitchChannel {
    fn as_capability(self) -> ChannelKind {
        match self {
            Self::Rehearsal => ChannelKind::Rehearsal,
            Self::WebEmbed => ChannelKind::WebEmbed,
            Self::LiveMeeting => ChannelKind::LiveMeeting,
            Self::Chat => ChannelKind::Chat,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Rehearsal => "Rehearsal",
            Self::WebEmbed => "WebEmbed",
            Self::LiveMeeting => "LiveMeeting",
            Self::Chat => "Chat",
        }
    }
}

#[derive(Debug, Deserialize)]
struct PitchAskRequest {
    question: String,
    #[serde(default = "default_channel")]
    channel: PitchChannel,
    #[serde(default)]
    audience: Option<String>,
}

fn default_channel() -> PitchChannel {
    PitchChannel::Rehearsal
}

const MISS_FALLBACK: &str = "I'll get back to you on that one — let me check with the founder.";

pub(super) async fn ask_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("");
    let parsed: PitchAskRequest = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let question = parsed.question.trim().to_string();
    if question.is_empty() {
        return bad_request("`question` is required");
    }
    let channel = parsed.channel;
    let audience = parsed.audience.as_deref().unwrap_or("investor").to_string();
    let start = Instant::now();

    let bundle = match resolve_bundle() {
        Ok(b) => b,
        Err(e) => return service_unavailable(&format!("pitch corpus/policies: {e}")),
    };

    let policy = bundle.policy_for(channel);
    let hit = bundle.index.lookup(&question, &audience);
    let latency_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    let (response_body, kind, audit_payload) = match hit {
        Some(matched) => {
            let evaluation = evaluate_claim(
                policy,
                channel.as_capability(),
                &ClaimCandidate {
                    text: matched.answer.clone(),
                    declared_classes: matched.declared_claim_classes.clone(),
                },
            );
            let (disposition_label, block_reason) = disposition_for_response(&evaluation.disposition);
            let answer = render_answer(&matched.answer, &evaluation.disposition);
            let body = json!({
                "matched": true,
                "entry_id": matched.entry_id,
                "answer": answer,
                "declared_claim_classes": matched.declared_claim_classes,
                "disposition": disposition_label,
                "block_reason": block_reason,
                "channel": channel.label(),
                "latency_ms": latency_ms,
            });
            let audit = json!({
                "entry_id": matched.entry_id,
                "channel": channel.label(),
                "disposition": disposition_label,
                "declared_claim_classes": matched.declared_claim_classes,
                "latency_ms": latency_ms,
            });
            (body, AuditKind::PitchCanonicalQaMatched, audit)
        }
        None => {
            let body = json!({
                "matched": false,
                "answer": MISS_FALLBACK,
                "disposition": "Escalate",
                "channel": channel.label(),
                "latency_ms": latency_ms,
            });
            let audit = json!({
                "channel": channel.label(),
                "latency_ms": latency_ms,
                "question_len": question.chars().count(),
            });
            (body, AuditKind::PitchCanonicalQaMiss, audit)
        }
    };

    emit_audit(store, req, kind, audit_payload).await;
    ("200 OK", response_body.to_string())
}

fn disposition_for_response(disp: &ClaimDisposition) -> (&'static str, Option<String>) {
    match disp {
        ClaimDisposition::Allow => ("Allow", None),
        ClaimDisposition::Forbid {
            refusal_template_id,
        } => ("Block", Some(format!("forbidden_class: {refusal_template_id}"))),
        ClaimDisposition::Escalate => ("Escalate", None),
    }
}

fn render_answer(canonical: &str, disp: &ClaimDisposition) -> String {
    match disp {
        // Canonical answers for forbidden classes are pre-written
        // refusals (see `pricing-and-business-model.toml`'s
        // `valuation-question`), so we ship them verbatim.
        ClaimDisposition::Allow | ClaimDisposition::Forbid { .. } => canonical.to_string(),
        ClaimDisposition::Escalate => {
            "Let me check with the founder before answering that one.".to_string()
        }
    }
}

fn bad_request(msg: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": msg } }).to_string(),
    )
}

fn service_unavailable(msg: &str) -> (&'static str, String) {
    (
        "503 Service Unavailable",
        json!({ "error": { "code": "pitch_unavailable", "message": msg } }).to_string(),
    )
}

/// Process-wide cached corpus + policies. Loaded on first request;
/// subsequent requests share the same index (sub-millisecond lookup).
struct PitchBundle {
    index: CanonicalQaIndex,
    public_policy: TruthBoundaryPolicy,
    private_policy: TruthBoundaryPolicy,
}

impl PitchBundle {
    fn policy_for(&self, channel: PitchChannel) -> &TruthBoundaryPolicy {
        if channel == PitchChannel::Rehearsal {
            &self.private_policy
        } else {
            &self.public_policy
        }
    }
}

static BUNDLE: OnceLock<Result<Arc<PitchBundle>, String>> = OnceLock::new();

fn resolve_bundle() -> Result<Arc<PitchBundle>, String> {
    let cached = BUNDLE.get_or_init(load_bundle);
    cached.as_ref().map(Arc::clone).map_err(String::clone)
}

fn load_bundle() -> Result<Arc<PitchBundle>, String> {
    let corpus_dir = corpus_dir();
    let public_path = policy_dir().join("skycot-public-pitch.toml");
    let private_path = policy_dir().join("skycot-private-rehearsal.toml");
    let public_policy = load_policy(&public_path)
        .map_err(|e| format!("load {}: {e}", public_path.display()))?;
    let private_policy = load_policy(&private_path)
        .map_err(|e| format!("load {}: {e}", private_path.display()))?;
    let index = CanonicalQaIndex::load(&corpus_dir, TenantId(Uuid::nil()))
        .map_err(|e| format!("load corpus {}: {e}", corpus_dir.display()))?;
    Ok(Arc::new(PitchBundle {
        index,
        public_policy,
        private_policy,
    }))
}

fn corpus_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ARGO_PITCH_CORPUS_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    workspace_root().join("skill-library/canonical-qa/corpora/skycot")
}

fn policy_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ARGO_PITCH_POLICY_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    workspace_root().join("policies/truth-boundary")
}

/// Best-effort repo-root resolution. Falls back to cwd if no
/// `Cargo.toml` ancestor is found.
fn workspace_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cursor: &Path = &cwd;
    loop {
        if cursor.join("Cargo.toml").exists() && cursor.join("crates").is_dir() {
            return cursor.to_path_buf();
        }
        match cursor.parent() {
            Some(parent) => cursor = parent,
            None => return cwd,
        }
    }
}

async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    req: &str,
    kind: AuditKind,
    payload: serde_json::Value,
) {
    let Some(store) = store else { return };
    let principal_id = routes_auth_principal::principal_id_for_audit(req);
    let sink = AuditSink::new_with_tenant(Arc::clone(store), rate_limit::extract_tenant(req));
    if let Err(e) = sink.write(None, kind, payload, principal_id).await {
        tracing::warn!(
            target: "gateway::pitch",
            error = %e,
            "pitch ask audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_pitch_ask_tests.rs"]
mod tests;
