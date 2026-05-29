//! `POST /api/v1/gtm/draft-followup` — Grant (the GTM Engineer) drafts a
//! truth-bounded inbound-growth follow-up.
//!
//! Grant owns a *responsibility* (inbound growth), not a one-off task:
//! given a waitlist signup he drafts a short, warm, brand-safe outreach
//! follow-up via the real LLM (the operator's Claude subscription —
//! DEC-012 `claude-code-cli`, Sonnet floor DEC-013), re-scans the draft
//! through the same truth-boundary the Pitch Agent uses
//! ([`super::routes_pitch_ask_llm::extract_forbidden_class`]), then routes
//! it to the **approval queue** (`/approvals`) plus an audit row. Grant
//! never auto-sends — the operator approves, then (deploy-time) the send
//! fires. No theater: drafts are real LLM output, nothing is faked.
//!
//! This is the second vertical built on Portia's proven
//! LLM + truth-boundary + audit primitives — not new infrastructure.
//!
//! Body: `{ "signup": { "name": "...", "email": "...", "context"?: "..." } }`.
//! Returns `{ draft, disposition, source, model, approval_id, ... }`.

use super::routes_pitch_ask_llm::{build_pitch_transport, extract_forbidden_class, user_message};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderRequest, ProviderResponse, Sampling};
use argo_store::approvals::{create_request, DefaultDecision, NewApprovalRequest};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Model floor for Grant's drafts. Sonnet (never Haiku) per DEC-013.
const GTM_MODEL: &str = "claude-sonnet-4-6";
/// Output cap — inbound follow-ups are 3-6 sentences.
const GTM_MAX_TOKENS: u32 = 400;

/// Parsed request body for `POST /api/v1/gtm/draft-followup`.
#[derive(Debug, Deserialize)]
struct DraftFollowupRequest {
    signup: Signup,
}

/// A waitlist signup Grant is asked to follow up with.
#[derive(Debug, Deserialize)]
struct Signup {
    /// Person's name (required — used to personalize the greeting).
    name: String,
    /// Email address (optional; surfaced in the approval summary).
    #[serde(default)]
    email: String,
    /// Free-text context about the lead (company, how they found us,
    /// what they want). Optional — Grant keeps the draft generic when
    /// absent rather than inventing detail.
    #[serde(default)]
    context: Option<String>,
}

/// Grant's lead qualification — the *triage* half of his charter
/// ("qualifies signups and drafts outreach"). Deterministic + rules-based
/// on purpose: the LLM writes the draft, but the qualification is
/// transparent signal extraction the operator can trust and audit (no
/// black-box scoring). `tier` is "Hot" | "Warm" | "Cold".
#[derive(Debug, Clone)]
struct Qualification {
    tier: &'static str,
    rationale: String,
    signals: Vec<String>,
}

/// Seniority markers — a decision-maker on the thread is the strongest
/// single signal. Substring match against the lowercased context.
const SENIORITY_MARKERS: &[&str] = &[
    "founder", "ceo", "cto", "coo", "cfo", "vp ", "vice president", "chief", "head of",
    "director", "owner", "president",
];
/// Active-intent markers — evidence they're evaluating / want to act now.
const INTENT_MARKERS: &[&str] = &[
    "evaluat", "looking for", "want", "need", "reduce", "cut ", "automat", "replace",
    "trial", "pilot", "budget", "buy", "scal", "onboard", "roll out", "rollout",
];
/// Scale markers — paired with a digit, signals real team/org size.
const SCALE_MARKERS: &[&str] =
    &["person", "employee", "seat", "team", "enterprise", "company-wide", "headcount", "org"];
/// Free-mail domain prefixes — a corporate domain is a (weak) B2B signal.
const FREE_EMAIL_DOMAINS: &[&str] = &[
    "gmail.", "yahoo.", "hotmail.", "outlook.", "icloud.", "proton.", "aol.", "live.", "gmx.",
    "mail.com",
];

/// Score a signup into a Hot/Warm/Cold tier with explainable signals.
/// Pure — unit-testable without a provider or store.
fn qualify(signup: &Signup) -> Qualification {
    let ctx = signup.context.as_deref().unwrap_or("").to_lowercase();
    let email = signup.email.trim().to_lowercase();
    let mut score = 0i32;
    let mut signals: Vec<String> = Vec::new();

    if SENIORITY_MARKERS.iter().any(|m| ctx.contains(m)) {
        score += 2;
        signals.push("senior decision-maker".to_string());
    }
    if INTENT_MARKERS.iter().any(|m| ctx.contains(m)) {
        score += 2;
        signals.push("active buying intent".to_string());
    }
    if ctx.chars().any(|c| c.is_ascii_digit()) && SCALE_MARKERS.iter().any(|m| ctx.contains(m)) {
        score += 1;
        signals.push("team-size / scale context".to_string());
    }
    let domain = email.rsplit('@').next().unwrap_or("");
    let corporate = email.contains('@')
        && !domain.is_empty()
        && !FREE_EMAIL_DOMAINS.iter().any(|d| domain.starts_with(d));
    if corporate {
        score += 1;
        signals.push("corporate email domain".to_string());
    }
    if ctx.trim().len() >= 60 {
        score += 1;
        signals.push("shared detailed context".to_string());
    }

    let tier = if score >= 4 {
        "Hot"
    } else if score >= 2 {
        "Warm"
    } else {
        "Cold"
    };
    let rationale = if signals.is_empty() {
        "Limited signal — treat as cold; a light, low-pressure note keeps the door open."
            .to_string()
    } else {
        format!("{tier} lead — {}.", signals.join(", "))
    };
    Qualification {
        tier,
        rationale,
        signals,
    }
}

/// Handle `POST /api/v1/gtm/draft-followup`. Returns the
/// `(status_line, json_body)` tuple the gateway dispatcher expects.
///
/// On the happy path Grant drafts via the live LLM, the draft is
/// truth-boundary-checked, an approval row + audit row are written to the
/// cwd-bound workspace tenant, and `200 OK` carries the draft envelope.
pub(super) async fn draft_followup_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = req.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    let parsed: DraftFollowupRequest = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("invalid JSON body: {e}")),
    };
    let signup = parsed.signup;
    if signup.name.trim().is_empty() {
        return bad_request("`signup.name` is required");
    }

    // Grant qualifies BEFORE he drafts (the triage half of his charter).
    // The tier informs the draft's close — Hot leads get a direct CTA.
    let qualification = qualify(&signup);

    // Same LLM gating as Portia: prefer a GTM-specific provider override,
    // fall back to the shared pitch provider env, default to `anthropic`.
    let provider = std::env::var("ARGO_GTM_LLM_PROVIDER")
        .or_else(|_| std::env::var("ARGO_PITCH_LLM_PROVIDER"))
        .unwrap_or_else(|_| "anthropic".to_string());

    let request = ProviderRequest {
        model: GTM_MODEL.to_string(),
        messages: vec![user_message(&build_user_prompt(&signup, &qualification))],
        tools: Vec::new(),
        system: Some(gtm_system_prompt()),
        sampling: Sampling {
            temperature: Some(0.4),
            max_tokens: Some(GTM_MAX_TOKENS),
            ..Sampling::default()
        },
    };

    let transport = match build_pitch_transport(&provider).await {
        Ok(t) => t,
        Err(e) => return service_unavailable(&format!("gtm llm transport: {e}")),
    };
    let response: ProviderResponse = match transport.complete(&request).await {
        Ok(r) => r,
        Err(e) => return service_unavailable(&format!("gtm llm call: {e}")),
    };
    let draft = response.text.unwrap_or_default();
    if draft.trim().is_empty() {
        return service_unavailable("gtm llm returned an empty draft");
    }

    // Truth-boundary: a draft that strays into guarded claims
    // (valuation / revenue / customer-count …) is held for founder
    // review rather than shipped — brand-safe, exactly like Portia.
    let (final_draft, disposition) = finalize_draft(&draft);

    // Route to the approval queue + audit (never auto-send). A missing
    // store (shouldn't happen in the live gateway) degrades to a draft
    // with no approval id rather than a 500.
    let (approval_id, tenant_label) = match store {
        Some(store) => {
            match write_to_approvals(store, &signup, &final_draft, disposition, &qualification).await
            {
                Ok((id, tenant)) => (Some(id), Some(tenant)),
                Err(e) => {
                    tracing::warn!(target: "gateway::gtm", error = %e, "gtm approval write failed");
                    (None, None)
                }
            }
        }
        None => (None, None),
    };

    let out = json!({
        "agent": "Grant",
        "responsibility": "inbound-growth",
        "qualification": {
            "tier": qualification.tier,
            "rationale": qualification.rationale,
            "signals": qualification.signals,
        },
        "draft": final_draft,
        "disposition": disposition,
        "source": "llm",
        "model": GTM_MODEL,
        "approval_id": approval_id,
        "tenant": tenant_label,
        "signup": { "name": signup.name, "email": signup.email },
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
                 flagging the lead for personal follow-up rather than an automated reply.]"
            ),
            "Escalate",
        ),
        None => (draft.to_string(), "Allow"),
    }
}

/// Resolve the cwd-bound workspace tenant, write the drafted follow-up as
/// a pending `approval_request` (so it surfaces in `/approvals`), and emit
/// an [`AuditKind::ApprovalAsked`] row in the same tenant. Returns
/// `(approval_id, tenant_string)`.
///
/// `AuditKind::ApprovalAsked` is reused deliberately — Grant *is* asking
/// for sign-off — so no new audit-kind variant is introduced (keeps the
/// `claim_6_approval_audit_kinds` variant-count contract untouched).
async fn write_to_approvals(
    store: &Arc<SessionStore>,
    signup: &Signup,
    draft: &str,
    disposition: &str,
    qualification: &Qualification,
) -> ArgoResult<(String, String)> {
    let tenant = resolve_workspace_tenant(store).await?;
    let action_id = Uuid::now_v7().as_bytes().to_vec();
    let email = if signup.email.trim().is_empty() {
        "no-email"
    } else {
        signup.email.trim()
    };
    let summary = format!(
        "Grant drafted an inbound follow-up to {} <{email}> — {} lead ({disposition}).",
        signup.name, qualification.tier
    );
    let body = format!("{summary}\n\nQualification: {}\n\n---\n{draft}", qualification.rationale);
    let new_req = NewApprovalRequest {
        action_id,
        tenant_id: tenant,
        requested_by: "Grant",
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
            "agent": "Grant",
            "responsibility": "inbound-growth",
            "action": "gtm.followup_drafted",
            "signup_name": signup.name,
            "signup_email": signup.email,
            "qualification_tier": qualification.tier,
            "qualification_signals": qualification.signals,
            "disposition": disposition,
            "source": "llm",
            "model": GTM_MODEL,
        }),
        None,
    )
    .await?;
    // Force the batched row to disk so a cold read right after the call
    // sees Grant's audit entry (mirrors the demo-seed write pattern).
    sink.flush_pending().await?;

    Ok((approval_id, tenant.to_string()))
}

/// Resolve the deterministic, cwd-derived workspace tenant — the same one
/// `/team` and `/approvals` read — so Grant's draft lands where the
/// operator will see it.
async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

/// The user-turn prompt: the signup Grant is following up with, plus his
/// own qualification so the draft's close matches the lead's temperature.
fn build_user_prompt(signup: &Signup, qualification: &Qualification) -> String {
    let mut out = String::new();
    out.push_str("Draft an inbound follow-up to this new SKYCOT waitlist signup.\n\n");
    out.push_str(&format!("Name: {}\n", signup.name.trim()));
    if !signup.email.trim().is_empty() {
        out.push_str(&format!("Email: {}\n", signup.email.trim()));
    }
    match signup.context.as_deref().map(str::trim) {
        Some(ctx) if !ctx.is_empty() => {
            out.push_str(&format!("Context they shared: {ctx}\n"));
        }
        _ => {
            out.push_str(
                "Context: none provided — keep the note generic; do not invent details \
                 about them or their company.\n",
            );
        }
    }
    // Tier-aware close: Grant qualified this lead, so match the call-to-action
    // to its temperature rather than sending every lead the same ask.
    out.push_str(&format!(
        "\nGrant's qualification: {} lead ({}). Tailor the close accordingly — \
         Hot: propose a specific, low-friction next step (a short call). \
         Warm: offer a quick call or a genuinely useful resource. \
         Cold: keep it light and low-pressure — just invite a reply.\n",
        qualification.tier, qualification.rationale
    ));
    out
}

/// Grant's system prompt: persona, the one responsibility, brand-safe
/// facts, and output discipline. Facts here are intentionally
/// non-quantitative — no valuation / revenue / customer-count claims —
/// so a faithful draft never trips the truth-boundary.
fn gtm_system_prompt() -> String {
    let mut out = String::new();
    out.push_str("# Persona\n\n");
    out.push_str(
        "You are Grant, SKYCOT's GTM (go-to-market) Engineer. You own inbound growth: when \
         someone joins the waitlist, you write the first warm, specific, human follow-up. You \
         are relationship-driven, persistent, brand-safe, and pipeline-minded.\n\n",
    );
    out.push_str("# Your task\n\n");
    out.push_str(
        "Write ONE short outbound follow-up email (3-6 sentences). Warm, specific, and useful. \
         Greet them by first name. If they shared context, reference it naturally; if not, keep \
         it generic. Invite a reply or a short call. Sign off as \"Grant, SKYCOT\". Output ONLY \
         the email body — no subject line, no preamble, no notes.\n\n",
    );
    out.push_str("# What SKYCOT is (approved facts — only claim these)\n\n");
    out.push_str(
        "- SKYCOT is a platform for hiring AI teammates that take on standing responsibilities, \
         not one-off tasks.\n\
         - Teammates delegate work to each other, share and learn skills, work across channels, \
         and run with human approval on sensitive actions.\n\
         - Every action is audited, and risky steps route to a human approval queue.\n\
         - SKYCOT is built in Melbourne, Australia.\n\n",
    );
    out.push_str("# Truth boundary (hard rules)\n\n");
    out.push_str(
        "Never state or imply a valuation, revenue, funding amount, or customer/user count. \
         Never promise pricing, dates, or features that aren't listed above. If a compelling \
         hook would require a number or claim you don't have, stay qualitative instead. When in \
         doubt, defer: it is always safe to offer a short call with the team.\n",
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
#[path = "routes_gtm_draft_followup_tests.rs"]
mod tests;
