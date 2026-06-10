//! LLM-fallback path for `POST /api/v1/pitch/ask`.
//!
//! When the deterministic canonical-Q&A lookup misses the operator's
//! question, this module drafts an answer via the active provider
//! manifest (currently `anthropic.toml`; the
//! `provider-claude-code-cli-subprocess` brief swaps in a CLI driver
//! with no change here) — then re-evaluates the draft through
//! [`argo_capability::evaluate_claim`] against the channel-appropriate
//! truth-boundary policy.
//!
//! The hot canonical-Q&A path in `routes_pitch_ask.rs` is unchanged.
//! This module is invoked only on a miss. Per the brief:
//!
//! 1. The LLM gets a system prompt rendering the truth-boundary
//!    policy + operator-reviewed company facts.
//! 2. The draft text is re-scanned with regex extractors for forbidden
//!    phrasings ("we are valued at", "our revenue is", etc.) and the
//!    matched class is fed back through `evaluate_claim`.
//! 3. If `Block` fires, the runtime swaps in a canonical refusal from
//!    the corpus (or a default refusal for the matched class) and the
//!    original LLM draft is logged for forensics but NOT shipped.
//!
//! Two audit rows always fire: [`AuditKind::PitchLlmCallEmitted`]
//! (model + tokens + latency + response hash) and
//! [`AuditKind::PitchTruthBoundaryEvaluated`] (disposition +
//! block_reason + matched_class + swapped_response).

use super::routes_pitch_ask::PitchChannel;
// Re-export so existing `routes_pitch_ask_llm::CompanyFacts` paths
// (routes_pitch_ask.rs + the tests module) keep resolving after the
// facts loader moved to a sibling module to satisfy the LOC ceiling.
pub(super) use super::routes_pitch_ask_llm_facts::CompanyFacts;
use argo_agent_runner::{build_transport, load_manifest};
use argo_capability::{
    evaluate_claim, ChannelKind, ClaimCandidate, ClaimDisposition, TruthBoundaryPolicy,
};
use argo_provider::{ProviderRequest, ProviderResponse, Sampling, Transport};
use argo_skill::canonical_qa::CanonicalQaIndex;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// One LLM-path response. Mirrored on the wire by `routes_pitch_ask.rs`
/// when it assembles the JSON envelope (this module is provider-agnostic).
#[derive(Debug, Clone)]
pub(super) struct LlmAskOutcome {
    /// The text shipped to the operator. Either the LLM draft (Allow /
    /// Escalate) or the canonical refusal that was swapped in (Block).
    pub answer: String,
    /// The LLM's draft text. Always populated; equal to `answer` on
    /// Allow / Escalate.
    pub draft: String,
    /// Verdict of the post-LLM truth-boundary evaluation.
    pub disposition: ClaimDisposition,
    /// The claim class that matched, if any. None on Allow.
    pub matched_class: Option<String>,
    /// True when the canonical refusal replaced the LLM draft.
    pub swapped_response: bool,
    /// Model id the transport returned (echoed from the manifest).
    pub model: String,
    /// Provider-reported input tokens.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens.
    pub tokens_out: Option<u32>,
    /// Sha256 hex of `answer` — pinned in the audit row so the audit
    /// chain can detect replay or in-flight tampering without
    /// leaking the answer body.
    pub response_hash: String,
}

/// Result of the canonical-refusal lookup. Returned by
/// [`refusal_for_class`] so the caller can audit "default vs corpus".
#[derive(Debug, Clone)]
pub(super) struct CanonicalRefusal {
    pub text: String,
    /// True iff the corpus carried a refusal for the matched class.
    /// Exposed on the struct so the unit tests can pin the distinction
    /// between "the LLM hit a class we have a canonical refusal for"
    /// vs "we fell back to the default deferral string".
    #[allow(dead_code)]
    pub from_corpus: bool,
}

/// Drive the LLM fallback for a missed canonical lookup. Returns the
/// outcome the caller renders into the JSON envelope.
///
/// `provider_id` controls the manifest used (`anthropic` by default).
/// `index` is reused to look up canonical refusal text by claim class.
/// `policy` is the channel-appropriate truth-boundary policy.
///
/// # Errors
/// Surfaces transport / manifest / credential errors as
/// `ArgoError::Provider` so the gateway can map them to a 503.
pub(super) async fn ask_with_llm(
    question: &str,
    channel: PitchChannel,
    policy: &TruthBoundaryPolicy,
    index: &CanonicalQaIndex,
    facts: &CompanyFacts,
    provider_id: &str,
) -> ArgoResult<LlmAskOutcome> {
    let system = render_system_prompt(channel, policy, facts);
    let req = ProviderRequest {
        model: PITCH_MODEL.to_string(),
        messages: vec![user_message(question)],
        tools: Vec::new(),
        system: Some(system),
        sampling: Sampling {
            temperature: Some(0.3),
            max_tokens: Some(PITCH_MAX_TOKENS),
            ..Sampling::default()
        },
    };
    let transport = build_pitch_transport(provider_id).await?;
    let resp: ProviderResponse = transport.complete(&req).await?;
    let draft = resp.text.unwrap_or_default();
    finalize_outcome(
        &draft,
        channel,
        policy,
        index,
        resp.tokens_in,
        resp.tokens_out,
    )
}

/// Pure shaping step — given a draft string, compute the truth-boundary
/// disposition, swap in a canonical refusal if needed, and assemble
/// the [`LlmAskOutcome`]. Split from [`ask_with_llm`] so the unit
/// tests can drive it without a live provider.
pub(super) fn finalize_outcome(
    draft: &str,
    channel: PitchChannel,
    policy: &TruthBoundaryPolicy,
    index: &CanonicalQaIndex,
    tokens_in: Option<u32>,
    tokens_out: Option<u32>,
) -> ArgoResult<LlmAskOutcome> {
    let matched_class = extract_forbidden_class(draft);
    let declared_classes = matched_class.iter().cloned().collect::<Vec<_>>();
    let evaluation = evaluate_claim(
        policy,
        channel.as_channel_kind(),
        &ClaimCandidate {
            text: draft.to_string(),
            declared_classes,
        },
    );
    let (answer, swapped) = match &evaluation.disposition {
        ClaimDisposition::Forbid { .. } => {
            let class = evaluation
                .matched_class
                .clone()
                .or_else(|| matched_class.clone())
                .unwrap_or_else(|| "forbidden".to_string());
            let refusal = refusal_for_class(&class, index);
            (refusal.text, true)
        }
        ClaimDisposition::Escalate => (
            "Let me check with the founder before answering that one.".to_string(),
            false,
        ),
        ClaimDisposition::Allow => (draft.to_string(), false),
    };
    let response_hash = sha256_hex(&answer);
    Ok(LlmAskOutcome {
        answer,
        draft: draft.to_string(),
        disposition: evaluation.disposition,
        matched_class: evaluation.matched_class.or(matched_class),
        swapped_response: swapped,
        model: PITCH_MODEL.to_string(),
        tokens_in,
        tokens_out,
        response_hash,
    })
}

/// Render the system prompt the LLM gets. Sections, in order:
/// persona, channel tone, truth-boundary policy, company facts,
/// output discipline. Each section is a Markdown header so the model
/// can attend to them as distinct rules.
pub(super) fn render_system_prompt(
    channel: PitchChannel,
    policy: &TruthBoundaryPolicy,
    facts: &CompanyFacts,
) -> String {
    let mut out = String::new();
    out.push_str("# Persona\n\n");
    out.push_str(
        "You are the SKYCOT Pitch Agent. You represent the company in investor, customer, \
         and press conversations. You speak briefly, with evidence, and with deference to the \
         founder on anything outside your approved scope.\n\n",
    );
    out.push_str(&format!(
        "# Channel\n\nYou are speaking on the **{}** channel. {}\n\n",
        channel.label(),
        channel_tone_notes(channel)
    ));
    out.push_str("# Truth boundary (do NOT cross)\n\n");
    out.push_str(&render_policy_block(policy));
    out.push_str("\n# Company facts (the only facts you may ground answers in)\n\n");
    out.push_str(&facts.as_markdown());
    out.push_str("\n# Output discipline\n\n");
    out.push_str(
        "- Answer in 1-3 sentences. Evidence-led, never aspirational.\n\
         - Use only facts from the section above. If a question requires a fact not in that \
           set, say so and offer to escalate to the founder.\n\
         - If the question targets a forbidden claim class above, do NOT attempt to answer \
           with a number, range, or hedge. Use a single sentence: \
           \"I'm not authorized to share <topic> publicly. Mark handles that one directly.\"\n\
         - Your answer may be read aloud by a text-to-speech voice, so write for the ear: \
           spell technical identifiers as natural spoken words, never as code tokens. Say \
           \"tenant ID\" not \"tenant_id\", \"process payment\" not \"process_payment\", \
           \"single Rust binary\" not \"single-Rust-binary\". No underscores, no snake_case, \
           no backticks, no inline code — a listener must never hear the word \"underscore\".\n\
         - Never repeat the user's question back. No headers, no bullets, no markdown — \
           plain conversational prose only.\n",
    );
    out
}

fn channel_tone_notes(channel: PitchChannel) -> &'static str {
    match channel {
        PitchChannel::Rehearsal => {
            "This is a private operator rehearsal — speak as you would to Mark directly."
        }
        PitchChannel::WebEmbed => {
            "This is the public web widget — assume an anonymous visitor reading on skycot.com."
        }
        PitchChannel::LiveMeeting => {
            "This is a live joined meeting (Zoom or Meet) — be conversational, short, and warm."
        }
        PitchChannel::Chat => {
            "This is a text channel (Slack, iMessage, email) — be concise and skim-friendly."
        }
    }
}

fn render_policy_block(policy: &TruthBoundaryPolicy) -> String {
    use argo_capability::ChannelScope;
    let mut out = String::new();
    for class in &policy.claim_classes {
        let scope = match &class.channels {
            ChannelScope::AllChannels => "all channels".to_string(),
            ChannelScope::Only(list) => format!(
                "only on: {}",
                list.iter().map(channel_kind_label).collect::<Vec<_>>().join(", ")
            ),
            ChannelScope::Except(list) => format!(
                "everywhere except: {}",
                list.iter().map(channel_kind_label).collect::<Vec<_>>().join(", ")
            ),
        };
        let verb = match &class.disposition {
            ClaimDisposition::Forbid { .. } => "FORBIDDEN",
            ClaimDisposition::Escalate => "ESCALATE-ONLY",
            ClaimDisposition::Allow => "Allowed",
        };
        out.push_str(&format!("- `{}` — {verb} ({scope})\n", class.name));
    }
    out
}

fn channel_kind_label(c: &ChannelKind) -> &'static str {
    match c {
        ChannelKind::Rehearsal => "Rehearsal",
        ChannelKind::WebEmbed => "WebEmbed",
        ChannelKind::LiveMeeting => "LiveMeeting",
        ChannelKind::Chat => "Chat",
    }
}

/// Default Anthropic model id used for the Pitch Agent. Pinned here so
/// the demo doesn't accidentally roll forward mid-call — operators
/// upgrade by editing this constant.
const PITCH_MODEL: &str = "claude-sonnet-4-6";
/// Output cap. Pitch answers are 1-3 sentences; 500 is comfortable
/// headroom for refusal phrasing.
const PITCH_MAX_TOKENS: u32 = 500;

/// Build the LLM transport for the Pitch Agent from the named provider
/// manifest. Routes through the shared [`build_transport`] dispatcher so
/// the manifest's `transport_kind` decides the driver:
///
/// - `anthropic` (default) → HTTP `AnthropicTransport` (production /
///   API-key path, unchanged).
/// - `claude-code-cli` → subprocess `ClaudeCodeCliTransport`, billing
///   against the operator's Claude Code subscription (dev/demo
///   cost-control path). Set `ARGO_PITCH_LLM_PROVIDER=claude-code-cli`
///   to activate.
///
/// Boxed (not `Arc<AnthropicTransport>`) so the subprocess driver can
/// flow through the same return type; the caller only needs `complete`.
pub(super) async fn build_pitch_transport(provider_id: &str) -> ArgoResult<Box<dyn Transport>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let manifest = load_manifest(&cwd, provider_id).map_err(|e| {
        ArgoError::Provider(format!("pitch llm: load provider `{provider_id}` manifest: {e}"))
    })?;
    build_transport(manifest).await
}

pub(super) fn user_message(prompt: &str) -> Message {
    Message {
        id: MessageId(Uuid::now_v7()),
        session_id: SessionId(Uuid::nil()),
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

/// Lightweight regex-free extractor: scans the draft for common
/// forbidden phrasings and returns the matching class name. Pattern
/// list lives inline (per the brief's v1 scope — v2 may move to a
/// learned classifier).
///
/// Patterns are case-insensitive substring matches. Order matters —
/// we return on first hit, so the most specific patterns go first.
pub(super) fn extract_forbidden_class(draft: &str) -> Option<String> {
    let lower = draft.to_lowercase();
    for (pattern, class) in FORBIDDEN_PATTERNS {
        if lower.contains(pattern) {
            return Some((*class).to_string());
        }
    }
    None
}

/// `(pattern, class)` table. Each pattern is a lowercase substring
/// match — keep them specific enough that a legitimate Allow answer
/// won't accidentally trip them.
const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    // valuation
    ("we are valued at", "valuation"),
    ("our valuation is", "valuation"),
    ("the round is at", "valuation"),
    ("valued at $", "valuation"),
    ("valued around", "valuation"),
    ("worth approximately", "valuation"),
    ("post-money", "valuation"),
    ("pre-money", "valuation"),
    // revenue
    ("our revenue is", "revenue"),
    ("our arr is", "revenue"),
    ("our mrr is", "revenue"),
    ("we make $", "revenue"),
    ("annual recurring revenue of", "revenue"),
    ("monthly recurring revenue of", "revenue"),
    ("revenue of $", "revenue"),
    // customer-count
    ("we have over", "customer-count"),
    ("we serve over", "customer-count"),
    ("we have more than", "customer-count"),
    ("our customer count is", "customer-count"),
    ("we have approximately", "customer-count"),
    // unannounced-roadmap
    ("q3 roadmap includes", "unannounced-roadmap"),
    ("q4 roadmap includes", "unannounced-roadmap"),
    ("our upcoming roadmap", "unannounced-roadmap"),
    ("our next release will", "unannounced-roadmap"),
    ("internally we are building", "unannounced-roadmap"),
    ("unannounced feature", "unannounced-roadmap"),
];

/// Look up a canonical refusal for the matched class. Falls back to a
/// generic deferral if no corpus entry carries the class.
pub(super) fn refusal_for_class(class: &str, index: &CanonicalQaIndex) -> CanonicalRefusal {
    for entry in index.entries() {
        if entry.declared_claim_classes.iter().any(|c| c == class) {
            // Only treat refusal-shaped entries as canonical refusals
            // — these are the ones whose answer text was authored to
            // defer (e.g. "Mark handles those directly"). Heuristic:
            // the answer mentions the founder or explicitly says
            // "not authorized" / "happy to put you in touch".
            let body = entry.answer.to_lowercase();
            if body.contains("mark")
                || body.contains("founder")
                || body.contains("not authorized")
                || body.contains("happy to put you in touch")
            {
                return CanonicalRefusal {
                    text: entry.answer.clone(),
                    from_corpus: true,
                };
            }
        }
    }
    CanonicalRefusal {
        text: default_refusal_for(class),
        from_corpus: false,
    }
}

fn default_refusal_for(class: &str) -> String {
    format!(
        "I'm not authorized to share {} publicly. Mark handles those conversations directly \
         — happy to put you in touch.",
        humanize_class(class)
    )
}

fn humanize_class(class: &str) -> &'static str {
    match class {
        "valuation" => "valuation",
        "revenue" => "revenue numbers",
        "customer-count" => "customer counts",
        "unannounced-roadmap" => "unannounced roadmap",
        _ => "that one",
    }
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
#[path = "routes_pitch_ask_llm_tests.rs"]
mod tests;
