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

use super::routes_pitch_ask_llm::{ask_with_llm, CompanyFacts};
use super::routes_auth_principal;
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{load_policy, ClaimDisposition, TruthBoundaryPolicy};
use argo_skill::canonical_qa::CanonicalQaIndex;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde::Deserialize;
use serde_json::json;
use shaping::{bad_request, build_canonical_hit_payload, build_llm_payload, service_unavailable};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use uuid::Uuid;

#[path = "routes_pitch_ask_shaping.rs"]
mod shaping;

/// Channel selector, canonicalized in `argo-capability` (next to the
/// `ChannelKind` it projects onto). Re-exported here so the sibling
/// gateway modules keep importing `super::routes_pitch_ask::PitchChannel`
/// unchanged; `.as_channel_kind()` / `.label()` are inherent methods on
/// the canonical type.
pub use argo_capability::PitchChannel;

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

pub(super) const MISS_FALLBACK: &str =
    "I'll get back to you on that one — let me check with the founder.";

/// Env-var gate for the LLM-fallback path. When set to `0` / `false`
/// / `off`, the gateway falls back to the deterministic
/// [`MISS_FALLBACK`] line on every canonical miss (the pre-LLM
/// behavior). Default is `1` (LLM enabled) — but the path is also
/// auto-disabled when no provider key is present in the environment,
/// so test runs without `ANTHROPIC_API_KEY` keep producing the
/// deterministic miss row instead of trying to dial out.
pub(super) const LLM_FALLBACK_ENV: &str = "ARGO_PITCH_LLM_FALLBACK";

/// Provider id used by the LLM-fallback path. Override with
/// `ARGO_PITCH_LLM_PROVIDER` so the
/// `provider-claude-code-cli-subprocess` driver can be wired in
/// without touching this file once it ships.
pub(super) const LLM_PROVIDER_ENV: &str = "ARGO_PITCH_LLM_PROVIDER";

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

    match hit {
        Some(matched) => {
            // Hot path: unchanged from the canonical-only shipping
            // implementation. Single audit row, sub-millisecond,
            // no LLM call.
            let (body_json, audit_payload) =
                build_canonical_hit_payload(channel, policy, &matched, latency_ms);
            emit_audit(
                store,
                req,
                AuditKind::PitchCanonicalQaMatched,
                audit_payload,
            )
            .await;
            ("200 OK", body_json.to_string())
        }
        None => handle_miss(req, store, &bundle, channel, &question, latency_ms, start).await,
    }
}

async fn handle_miss(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    bundle: &PitchBundle,
    channel: PitchChannel,
    question: &str,
    canonical_latency_ms: u64,
    start: Instant,
) -> (&'static str, String) {
    // Always emit the canonical-miss audit row first — the LLM path
    // is additive, so the operator can audit "we tried canonical and
    // missed" independently of "the LLM said X".
    let miss_audit = json!({
        "channel": channel.label(),
        "latency_ms": canonical_latency_ms,
        "question_len": question.chars().count(),
    });
    emit_audit(store, req, AuditKind::PitchCanonicalQaMiss, miss_audit).await;

    if !llm_fallback_enabled() {
        let body = json!({
            "matched": false,
            "source": "canonical",
            "answer": MISS_FALLBACK,
            "disposition": "Escalate",
            "channel": channel.label(),
            "latency_ms": canonical_latency_ms,
        });
        return ("200 OK", body.to_string());
    }

    let facts = match CompanyFacts::load_cached() {
        Ok(f) => f,
        Err(e) => return service_unavailable(&format!("pitch facts: {e}")),
    };
    let provider_id =
        std::env::var(LLM_PROVIDER_ENV).unwrap_or_else(|_| "anthropic".to_string());
    let policy = bundle.policy_for(channel);

    let llm_result = ask_with_llm(
        question,
        channel,
        policy,
        &bundle.index,
        &facts,
        &provider_id,
        store,
    )
    .await;
    let total_latency_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    let outcome = match llm_result {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(
                target: "gateway::pitch",
                error = %e,
                "pitch llm fallback failed; reverting to deterministic miss"
            );
            let body = json!({
                "matched": false,
                "source": "canonical",
                "answer": MISS_FALLBACK,
                "disposition": "Escalate",
                "channel": channel.label(),
                "latency_ms": total_latency_ms,
            });
            return ("200 OK", body.to_string());
        }
    };

    let (body_json, llm_audit, eval_audit) =
        build_llm_payload(channel, &outcome, total_latency_ms);
    emit_audit(store, req, AuditKind::PitchLlmCallEmitted, llm_audit).await;
    emit_audit(
        store,
        req,
        AuditKind::PitchTruthBoundaryEvaluated,
        eval_audit,
    )
    .await;
    ("200 OK", body_json.to_string())
}

fn llm_fallback_enabled() -> bool {
    // Explicit opt-out wins.
    if let Ok(v) = std::env::var(LLM_FALLBACK_ENV) {
        let lower = v.trim().to_lowercase();
        if matches!(lower.as_str(), "0" | "false" | "off" | "no") {
            return false;
        }
    }
    // Auto-disable when no provider key is present — keeps the
    // existing test fixtures (which don't set ANTHROPIC_API_KEY)
    // producing the deterministic miss row.
    let has_key = ["ANTHROPIC_API_KEY", "ARGO_GENESIS_PROVIDER_KEY"]
        .iter()
        .any(|k| {
            std::env::var(k)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        });
    has_key
}

// Disposition/answer shaping kept here (not in the `shaping` sibling) because
// `routes_pitch_ask_tests` exercises these two directly; the sibling's
// `build_*_payload` helpers call them via `super::`.
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

/// Build the spoken-realtime system instructions for `channel`: the same
/// truth-boundary policy + company facts the typed/loop pitch renders, reused
/// verbatim so the live-voice (WebRTC) session is bounded identically — no
/// parallel policy copy. Consumed by `super::routes_voice_realtime`.
///
/// # Errors
/// Surfaces corpus/policy load or facts-parse failures (as a `String`) so the
/// caller can map them to a 503.
pub(super) fn realtime_instructions(channel: PitchChannel) -> Result<String, String> {
    let bundle = resolve_bundle()?;
    let policy = bundle.policy_for(channel);
    let facts = CompanyFacts::load_cached().map_err(|e| e.to_string())?;
    Ok(super::routes_pitch_ask_llm::render_system_prompt(channel, policy, &facts))
}

/// The canonical SKYCOT company facts as Markdown — for grounding a live-voice
/// persona that is NOT the pitch agent (e.g. Cora the coach) in the same
/// approved facts Portia uses, without Portia's pitch framing.
pub(super) fn company_facts_markdown() -> Result<String, String> {
    Ok(CompanyFacts::load_cached()
        .map_err(|e| e.to_string())?
        .as_markdown())
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
    // Round-trip with the audit feed: stamp the same tenant the feed's
    // default scope reads back (canonical session/header/cwd resolver),
    // not the rate-limit `extract_tenant` config/anonymous tail. See
    // `resolve_tenant::resolve_audit_write_tenant`.
    let tenant = super::resolve_tenant::resolve_audit_write_tenant(req, store).await;
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant);
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
