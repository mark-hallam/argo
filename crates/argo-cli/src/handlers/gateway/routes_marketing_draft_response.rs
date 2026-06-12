//! `POST /api/v1/marketing/draft-response` — Marcus (the Marketing
//! teammate) drafts a truth-bounded, approval-gated piece of public
//! marketing copy in Mark's documented voice.
//!
//! Marcus owns a *responsibility* (top-of-funnel marketing + social),
//! not a one-off task. Two modes, two channels:
//!
//! - `mode = "reply"` — the operator pastes a mention / URL / quoted
//!   post; Marcus drafts a reply to it.
//! - `mode = "article"` — the operator gives a topic; Marcus drafts a
//!   thought-leadership article on it.
//! - `channel = "x"` — short, punchy, no sign-off (X/Twitter format).
//! - `channel = "linkedin"` — long-form CEO voice, ending `— Mark + the
//!   team` (LinkedIn format).
//!
//! Like Grant, Marcus drafts via the real LLM (the operator's Claude
//! subscription — DEC-012 `claude-code-cli`, Sonnet floor DEC-013),
//! re-scans the draft through the same truth-boundary the Pitch Agent
//! uses ([`super::routes_pitch_ask_llm::extract_forbidden_class`]),
//! then routes the draft to the **approval queue** (`/approvals`) plus
//! a hash-chained audit row. **Marcus never posts** — there is no
//! posting integration (greenfield per the marketing-teammate plan);
//! the operator hand-posts the approved draft. No theater: drafts are
//! real LLM output, nothing is faked (DEC-007).
//!
//! This is the third hero teammate built on Portia's proven
//! LLM + truth-boundary + audit primitives (DEC-028) — not new
//! infrastructure; it is a near-identical sibling of Grant's route.
//!
//! Body: `{ "request": { "mode": "reply"|"article", "channel":
//! "x"|"linkedin", "source": "...", "topic"?: "..." } }`.
//! Returns `{ draft, disposition, mode, channel, source, model,
//! approval_id, ... }`.

use super::routes_pitch_ask_llm::{build_pitch_transport, extract_forbidden_class, user_message};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderRequest, ProviderResponse, Sampling};
use argo_store::approvals::{create_request, DefaultDecision, NewApprovalRequest};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Model floor for Marcus's drafts. Sonnet (never Haiku) per DEC-013.
const MARKETING_MODEL: &str = "claude-sonnet-4-6";
/// Output cap for a short X/Twitter draft (a tweet or a tight thread).
const X_MAX_TOKENS: u32 = 320;
/// Output cap for a long-form LinkedIn draft (a CEO-voice post).
const LINKEDIN_MAX_TOKENS: u32 = 900;

/// Parsed request body for `POST /api/v1/marketing/draft-response`.
#[derive(Debug, Deserialize)]
struct DraftRequestBody {
    request: DraftRequest,
}

/// What Marcus is asked to draft.
#[derive(Debug, Deserialize)]
struct DraftRequest {
    /// `"reply"` (respond to a pasted mention/URL) or `"article"`
    /// (write a thought-leadership piece from a topic). Free text on
    /// the wire; normalized + validated in [`Mode::parse`].
    mode: String,
    /// `"x"` (short) or `"linkedin"` (long-form). Validated in
    /// [`Channel::parse`].
    channel: String,
    /// In `reply` mode: the pasted mention / URL / quoted post Marcus
    /// is replying to. In `article` mode: optional extra framing.
    /// Marcus does NOT auto-fetch a URL (no web-fetch tool exists yet);
    /// the operator pastes the text they want him to react to.
    #[serde(default)]
    source: String,
    /// In `article` mode: the topic to write about (required there).
    /// Ignored in `reply` mode.
    #[serde(default)]
    topic: Option<String>,
}

/// Draft mode — what shape of copy Marcus produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Reply to a pasted mention / URL / quoted post.
    Reply,
    /// Thought-leadership article from a topic.
    Article,
}

impl Mode {
    /// Parse the wire string into a [`Mode`]. Case-insensitive.
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "reply" => Some(Self::Reply),
            "article" => Some(Self::Article),
            _ => None,
        }
    }

    /// Stable wire label echoed back in the response envelope.
    fn label(self) -> &'static str {
        match self {
            Self::Reply => "reply",
            Self::Article => "article",
        }
    }
}

/// Publishing channel — shapes length + tone + sign-off only. It does
/// NOT imply any posting integration (there is none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// X / Twitter — short, punchy, no sign-off.
    X,
    /// LinkedIn — long-form CEO voice, `— Mark + the team` sign-off.
    LinkedIn,
}

impl Channel {
    /// Parse the wire string into a [`Channel`]. Case-insensitive;
    /// accepts `twitter` as an alias for `x`.
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "x" | "twitter" => Some(Self::X),
            "linkedin" => Some(Self::LinkedIn),
            _ => None,
        }
    }

    /// Stable wire label echoed back in the response envelope.
    fn label(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::LinkedIn => "linkedin",
        }
    }

    /// Output-token cap for this channel's format.
    fn max_tokens(self) -> u32 {
        match self {
            Self::X => X_MAX_TOKENS,
            Self::LinkedIn => LINKEDIN_MAX_TOKENS,
        }
    }
}

/// Handle `POST /api/v1/marketing/draft-response`. Returns the
/// `(status_line, json_body)` tuple the gateway dispatcher expects.
///
/// On the happy path Marcus drafts via the live LLM, the draft is
/// truth-boundary-checked, an approval row + audit row are written to
/// the cwd-bound workspace tenant, and `200 OK` carries the draft
/// envelope. The draft is NEVER posted anywhere.
pub(super) async fn draft_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = req.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    let parsed: DraftRequestBody = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("invalid JSON body: {e}")),
    };
    let request = parsed.request;

    let mode = match Mode::parse(&request.mode) {
        Some(m) => m,
        None => return bad_request("`request.mode` must be \"reply\" or \"article\""),
    };
    let channel = match Channel::parse(&request.channel) {
        Some(c) => c,
        None => return bad_request("`request.channel` must be \"x\" or \"linkedin\""),
    };

    // Validate the mode-specific input is present so Marcus never
    // invents a subject out of thin air.
    let topic = request.topic.as_deref().map(str::trim).unwrap_or("");
    let source = request.source.trim();
    match mode {
        Mode::Reply if source.is_empty() => {
            return bad_request(
                "`request.source` is required in reply mode (paste the mention / post / URL)",
            );
        }
        Mode::Article if topic.is_empty() => {
            return bad_request("`request.topic` is required in article mode");
        }
        _ => {}
    }

    // Same LLM gating as Portia / Grant: prefer a marketing-specific
    // provider override, fall back to the shared pitch provider env,
    // default to `anthropic`.
    let provider = std::env::var("ARGO_MARKETING_LLM_PROVIDER")
        .or_else(|_| std::env::var("ARGO_PITCH_LLM_PROVIDER"))
        .unwrap_or_else(|_| "anthropic".to_string());

    let request_obj = ProviderRequest {
        model: MARKETING_MODEL.to_string(),
        messages: vec![user_message(&build_user_prompt(mode, channel, source, topic))],
        tools: Vec::new(),
        system: Some(marketing_system_prompt(mode, channel)),
        sampling: Sampling {
            temperature: Some(0.6),
            max_tokens: Some(channel.max_tokens()),
            ..Sampling::default()
        },
    };

    let transport = match build_pitch_transport(&provider, store).await {
        Ok(t) => t,
        Err(e) => return service_unavailable(&format!("marketing llm transport: {e}")),
    };
    let response: ProviderResponse = match transport.complete(&request_obj).await {
        Ok(r) => r,
        Err(e) => return service_unavailable(&format!("marketing llm call: {e}")),
    };
    let draft = response.text.unwrap_or_default();
    if draft.trim().is_empty() {
        return service_unavailable("marketing llm returned an empty draft");
    }

    // Truth-boundary: a draft that strays into guarded claims
    // (valuation / revenue / customer-count …) is held for founder
    // review rather than shipped — brand-safe, exactly like Portia.
    let (final_draft, disposition) = finalize_draft(&draft);

    // Route to the approval queue + audit (never auto-post). A missing
    // store (shouldn't happen in the live gateway) degrades to a draft
    // with no approval id rather than a 500.
    let (approval_id, tenant_label) = match store {
        Some(store) => {
            match write_to_approvals(store, req, mode, channel, &final_draft, disposition).await {
                Ok((id, tenant)) => (Some(id), Some(tenant)),
                Err(e) => {
                    tracing::warn!(target: "gateway::marketing", error = %e, "marketing approval write failed");
                    (None, None)
                }
            }
        }
        None => (None, None),
    };

    let out = json!({
        "agent": "Marcus",
        "responsibility": "marketing",
        "mode": mode.label(),
        "channel": channel.label(),
        "draft": final_draft,
        "disposition": disposition,
        "source": "llm",
        "model": MARKETING_MODEL,
        "approval_id": approval_id,
        "tenant": tenant_label,
        "tokens": { "input": response.tokens_in, "output": response.tokens_out },
    });
    ("200 OK", out.to_string())
}

/// Pure truth-boundary shaping: run the draft through the shared
/// forbidden-class extractor. A clean draft is `Allow`ed as-is; a draft
/// that touches a guarded topic is swapped for a safe "held for founder
/// review" note and marked `Escalate`. Split out so it is unit-testable
/// without a live provider.
fn finalize_draft(draft: &str) -> (String, &'static str) {
    match extract_forbidden_class(draft) {
        Some(class) => (
            format!(
                "[Held for founder review — this draft touched a guarded topic ({class}); \
                 flagging it for Mark to handle directly rather than publishing an \
                 unverified public claim.]"
            ),
            "Escalate",
        ),
        None => (draft.to_string(), "Allow"),
    }
}

/// Resolve the cwd-bound workspace tenant, write the drafted copy as a
/// pending `approval_request` (so it surfaces in `/approvals`), and emit
/// an [`AuditKind::ApprovalAsked`] row in the same tenant. Returns
/// `(approval_id, tenant_string)`.
///
/// `AuditKind::ApprovalAsked` is reused deliberately — Marcus *is*
/// asking for sign-off before anything is public — so no new audit-kind
/// variant is introduced (keeps the `claim_6_approval_audit_kinds`
/// variant-count contract untouched), exactly as Grant's route does.
async fn write_to_approvals(
    store: &Arc<SessionStore>,
    req: &str,
    mode: Mode,
    channel: Channel,
    draft: &str,
    disposition: &str,
) -> ArgoResult<(String, String)> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store).await?;
    let action_id = Uuid::now_v7().as_bytes().to_vec();
    let summary = format!(
        "Marcus drafted a {} {} post ({disposition}) — review before you post it.",
        channel.label(),
        mode.label()
    );
    let body = format!("{summary}\n\n---\n{draft}");
    let new_req = NewApprovalRequest {
        action_id,
        tenant_id: tenant,
        requested_by: "Marcus",
        deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
        default_decision: DefaultDecision::Deny,
        draft_content: Some(&body),
    };
    let approval_id = create_request(store.pool(), &new_req).await?;

    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        None,
        AuditKind::ApprovalAsked,
        json!({
            "approval_id": approval_id,
            "agent": "Marcus",
            "responsibility": "marketing",
            "action": "marketing.response_drafted",
            "mode": mode.label(),
            "channel": channel.label(),
            "disposition": disposition,
            "source": "llm",
            "model": MARKETING_MODEL,
        }),
        None,
    )
    .await?;
    // Force the batched row to disk so a cold read right after the call
    // sees Marcus's audit entry (mirrors Grant's write pattern).
    sink.flush_pending().await?;

    Ok((approval_id, tenant.to_string()))
}

/// The user-turn prompt: the concrete thing Marcus is reacting to (the
/// pasted mention in reply mode, or the topic in article mode), plus the
/// channel-format reminder.
fn build_user_prompt(mode: Mode, channel: Channel, source: &str, topic: &str) -> String {
    let mut out = String::new();
    match mode {
        Mode::Reply => {
            out.push_str(
                "Draft a reply to the following post / mention. Engage with what it \
                 actually says — do not change the subject, do not invent facts about \
                 the author or their company.\n\n",
            );
            out.push_str("--- the post / mention to reply to ---\n");
            out.push_str(source);
            out.push('\n');
            out.push_str("--- end ---\n");
        }
        Mode::Article => {
            out.push_str(
                "Draft a short thought-leadership piece on the following topic, in \
                 Mark's voice. Open concrete, build the idea, land on a take.\n\n",
            );
            out.push_str(&format!("Topic: {topic}\n"));
            if !source.is_empty() {
                out.push_str(&format!("Extra framing the operator gave: {source}\n"));
            }
        }
    }
    out.push_str(match channel {
        Channel::X => {
            "\nFormat: X/Twitter. Short and punchy — one tight post (a few short \
             sentences) or a small thread. No hashtags, no thread-emoji, no sign-off."
        }
        Channel::LinkedIn => {
            "\nFormat: LinkedIn long-form. A few short paragraphs in a considered CEO \
             voice. End with the sign-off line: — Mark + the team."
        }
    });
    out
}

/// Marcus's system prompt: persona, the one responsibility, voice rules
/// (from `VOICE_PROFILE.md` + the marketing charter's anti-hype list),
/// brand-safe facts, and a hard truth boundary. Facts here are
/// intentionally non-quantitative — no valuation / revenue /
/// customer-count claims — so a faithful draft never trips the
/// truth-boundary.
fn marketing_system_prompt(mode: Mode, channel: Channel) -> String {
    let mut out = String::new();
    out.push_str("# Persona\n\n");
    out.push_str(
        "You are Marcus, SKYCOT's marketing teammate. You own top-of-funnel marketing \
         and social: replying to mentions of SKYCOT, joining adjacent conversations, and \
         drafting thought-leadership content. You write in the founder Mark Hallam's \
         documented voice — a tip-of-the-spear builder, hands at the coal face, honest \
         about what's still being figured out.\n\n",
    );
    out.push_str("# Your task\n\n");
    out.push_str(match mode {
        Mode::Reply => {
            "Draft ONE reply to the post / mention the user pastes. Add a genuine, \
             specific point of view — never a generic \"great post!\". "
        }
        Mode::Article => {
            "Draft ONE short thought-leadership piece on the topic the user gives. "
        }
    });
    out.push_str(match channel {
        Channel::X => {
            "Keep it short and punchy (X/Twitter). Output ONLY the post text — no \
             preamble, no notes, no hashtags, no sign-off.\n\n"
        }
        Channel::LinkedIn => {
            "Write it long-form in a CEO voice (LinkedIn): a few short paragraphs, \
             ending with the line \"— Mark + the team\". Output ONLY the post text — no \
             preamble, no notes.\n\n"
        }
    });
    out.push_str("# Voice (Mark's documented style)\n\n");
    out.push_str(
        "- Open with a specific thing — never \"In today's AI landscape...\".\n\
         - Mix short and long sentences. End on a take, not a summary.\n\
         - It's fine to be mid-thought in public (\"still figuring this out\", \"my read \
           is...\"). That honesty is the point.\n\
         - The load-bearing idea Mark wants to own: ideas are still the human moat — AI \
           executes brilliantly, humans generate the novel ideas. Ladder back to it when \
           it fits.\n\n",
    );
    out.push_str("# Hard no-list (never write these)\n\n");
    out.push_str(
        "- AI-marketing clichés: revolutionize, transform, game-changer, \
           next-generation, AI-powered, paradigm-shift.\n\
         - LinkedIn-influencer tells: \"Here's what I learned 👇\", numbered \
           pseudo-threads, \"Let that sink in\", emoji bullets, \"Here's the kicker\".\n\
         - Corporate-speak: synergy, ecosystem, holistic, leverage (as a verb).\n\
         - Self-aggrandizing openers: \"As a founder...\", \"As a CEO...\".\n\n",
    );
    out.push_str("# What SKYCOT is (approved facts — only claim these)\n\n");
    out.push_str(
        "- SKYCOT is a platform for hiring AI teammates that take on standing \
           responsibilities, not one-off tasks.\n\
         - Teammates delegate work to each other, share and learn skills, work across \
           channels, and run with human approval on sensitive actions.\n\
         - Every action is recorded on a tamper-evident audit ledger, and risky steps \
           route to a human approval queue before anything happens.\n\
         - SKYCOT is built in Melbourne, Australia.\n\n",
    );
    out.push_str("# Truth boundary (hard rules)\n\n");
    out.push_str(
        "Never state or imply a valuation, revenue, funding amount, or customer/user \
         count. Never promise pricing, dates, or features that aren't listed above. If a \
         compelling hook would require a number or claim you don't have, stay qualitative \
         instead. When in doubt, defer — a draft Mark reviews is always better than an \
         unverified public claim.\n",
    );
    out
}

/// `400 Bad Request` envelope.
fn bad_request(message: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({ "error": message }).to_string())
}

/// `503 Service Unavailable` envelope (LLM transport / call failures).
fn service_unavailable(message: &str) -> (&'static str, String) {
    ("503 Service Unavailable", json!({ "error": message }).to_string())
}

#[cfg(test)]
#[path = "routes_marketing_draft_response_tests.rs"]
mod tests;
