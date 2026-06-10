//! `POST /api/v1/genesis/conversation` — the multi-turn conversational intake
//! ("Talk it through", DEC-039).
//!
//! Additive companion to the one-shot `/api/v1/genesis` SSE: instead of one
//! "describe + watch" submission, this endpoint runs a back-and-forth that
//! elicits the SAME [`argo_genesis::CapabilitySpec`] slots
//! (`responsibilities` / `cadence` / `exceptions` / `success_criteria` +
//! charter/persona) one question at a time. Each POST is one turn; the captured
//! state is persisted **per tenant** between turns via
//! [`argo_genesis::IntakeStore`] (disk, tenant-segmented — see that module's
//! isolation guarantee).
//!
//! ## Convergence — no parallel materializer
//!
//! When every required slot is filled the response carries an assembled
//! [`argo_genesis::CapabilitySpec`] (`status: "ready"`). The SPA submits the
//! SAME spec's slots to the existing `/api/v1/genesis` one-shot endpoint, which
//! runs the IDENTICAL `materialize → review → hire` path. This layer only
//! elicits; it never forks the slot types or the materializer.
//!
//! ## Two bounded LLM calls per turn, both degrade fail-safe
//!
//! 1. extract the operator's answer → slot delta ([`argo_genesis::extract_delta`]);
//! 2. propose the next question ([`argo_genesis::propose_question`]).
//!
//! Both ride the SAME shared `complete_once` transport the classifier / author /
//! rubric use ([`super::routes_genesis_stream_llm`]). When NO provider key is
//! present, OR either call fails, the turn falls back to the deterministic wizard
//! ([`argo_genesis::IntakeSlot::wizard_question`] +
//! [`argo_genesis::IntakeState::apply_raw_answer`]) — so a no-LLM demo still
//! completes a full intake.

use std::sync::Arc;

use argo_genesis::{
    extract_delta, propose_question, IntakeClient, IntakeSlot, IntakeState, IntakeStore, NextStep,
};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};

/// Dispatch entry — parse the turn, run it, return the JSON envelope. Mirrors
/// the `(status_line, body)` shape of the other genesis JSON shims.
pub(super) async fn respond(
    req: &str,
    session_store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = super::body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let conversation_id = match parsed
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(id) => id.to_string(),
        None => return bad_request("`conversation_id` (non-empty string) is required"),
    };
    let message = parsed
        .get("message")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let confirm = parsed
        .get("confirm")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let tenant = resolve_intake_tenant(req, session_store).await;
    let store = IntakeStore::new(argo_vertical::default_dir());
    // Gate the LLM enrichment on a provider key; absent one, the turn runs the
    // deterministic wizard (the fail-safe demo path).
    let provider = super::startup_check::provider_key_present().then(resolve_provider_id);

    match run_turn(
        &store,
        &tenant,
        &conversation_id,
        message.as_deref(),
        confirm,
        provider.as_deref(),
    )
    .await
    {
        Ok((status, value)) => (status, value.to_string()),
        Err(e) => server_error(&e),
    }
}

/// Resolve the tenant the intake store is keyed on for this turn — the
/// JWT-bound tenant, NOT a client-supplied header.
///
/// `rate_limit::extract_tenant` honors `?tenant=` / `X-Tenant-Id`, which an
/// authenticated tenant-A caller can set to `<B>` to read or poison tenant B's
/// in-flight teammate draft (P1 isolation gap, security audit 2026-06-10).
/// [`super::resolve_tenant::resolve_tenant_for_request`] prefers the
/// `__Host-session` JWT `tnt` claim (WS1 keystone, PRs #2710/#2754), so a
/// session caller can only ever resolve their OWN tenant; the explicit header
/// is reachable only on the no-session path. Falls back to `extract_tenant`
/// solely when there is no session store or the cwd resolver errors, so an
/// anonymous/demo turn still keys somewhere rather than dropping to an empty
/// tenant.
async fn resolve_intake_tenant(req: &str, session_store: Option<&Arc<SessionStore>>) -> String {
    match session_store {
        Some(session_store) => {
            match super::resolve_tenant::resolve_tenant_for_request(req, session_store).await {
                Ok(tenant) => tenant.to_string(),
                Err(_) => super::rate_limit::extract_tenant(req),
            }
        }
        None => super::rate_limit::extract_tenant(req),
    }
}

/// Run one conversation turn against the persisted state. `provider` is
/// `Some(id)` to enable the two bounded LLM calls, `None` for the pure wizard.
async fn run_turn(
    store: &IntakeStore,
    tenant: &str,
    conversation_id: &str,
    message: Option<&str>,
    confirm: bool,
    provider: Option<&str>,
) -> Result<(&'static str, Value), String> {
    let mut state = store.load(tenant, conversation_id).unwrap_or_default();

    // Apply the operator's answer (if any) to the slot the previous question
    // targeted — the persisted `pending_slot` when present (DEC-048: optional
    // slots + follow-ups mean the target is no longer always the first
    // unfilled slot), else the legacy first-unfilled fallback so pre-DEC-048
    // conversations on disk keep working.
    if let Some(msg) = message.map(str::trim).filter(|m| !m.is_empty()) {
        let target = state.pending_slot.take().or_else(|| state.first_unfilled_slot());
        state.record_operator(msg);
        if let Some(slot) = target {
            if IntakeSlot::OPTIONAL.contains(&slot) && is_skip(msg) {
                // An explicit skip resolves the optional slot unfilled — the
                // seeded defaults apply and the build is never wedged.
                state.mark_optional_resolved(slot);
            } else {
                apply_answer(&mut state, slot, msg, provider).await;
                if IntakeSlot::OPTIONAL.contains(&slot) {
                    state.mark_optional_resolved(slot);
                }
                // DEC-048 quality bar: at most ONE follow-up per slot when the
                // captured value is too vague to mint a useful teammate.
                // Deterministic + provider-free; a second vague answer is
                // accepted (the cap), so intake can never loop or stall.
                if !state.followups_used.contains(&slot) {
                    if let Some(followup) = argo_genesis::needs_followup(&state, slot) {
                        state.followups_used.push(slot);
                        state.pending_slot = Some(slot);
                        state.record_assistant(followup);
                        let envelope = json!({
                            "conversation_id": conversation_id,
                            "status": "asking",
                            "slot": slot.slug(),
                            "assistant_message": followup,
                            "can_confirm": false,
                            "captured": captured(&state),
                            "transcript": transcript(&state),
                        });
                        store
                            .save(tenant, conversation_id, &state)
                            .map_err(|e| format!("persist intake: {e}"))?;
                        return Ok(("200 OK", envelope));
                    }
                }
            }
        }
    }

    // Confirm requires the required set complete AND no quality follow-up
    // awaiting its answer — otherwise "Skip the rest & build" would silently
    // bypass the DEC-048 quality bar (adversarial-review P1). A refused
    // confirm falls through to next_step(), which re-asks the follow-up; ONE
    // answer (vague is fine — the cap accepts it) unblocks confirm.
    if confirm
        && state.is_complete()
        && argo_genesis::required_followup_pending(&state).is_none()
    {
        state.confirmed = true;
    }

    let envelope = match state.next_step() {
        NextStep::Ask { slot, question } => {
            let question = enrich_question(&state, slot, &question, provider).await;
            state.record_assistant(&question);
            state.pending_slot = Some(slot);
            json!({
                "conversation_id": conversation_id,
                "status": "asking",
                "slot": slot.slug(),
                "assistant_message": question,
                // True once every REQUIRED slot is filled AND no quality
                // follow-up is pending — the SPA offers "build now" only when
                // confirm would actually succeed.
                "can_confirm": state.is_complete()
                    && argo_genesis::required_followup_pending(&state).is_none(),
                "captured": captured(&state),
                "transcript": transcript(&state),
            })
        }
        NextStep::Confirm(spec) => json!({
            "conversation_id": conversation_id,
            "status": "ready",
            "captured": captured(&state),
            "spec": *spec,
            "transcript": transcript(&state),
        }),
        NextStep::Done(spec) => json!({
            "conversation_id": conversation_id,
            "status": "done",
            "captured": captured(&state),
            "spec": *spec,
            "transcript": transcript(&state),
        }),
    };

    store
        .save(tenant, conversation_id, &state)
        .map_err(|e| format!("persist intake: {e}"))?;
    Ok(("200 OK", envelope))
}

/// Fill `slot` from the operator's raw answer, preferring the LLM slot-delta
/// extraction and ALWAYS falling back to the deterministic wizard fill so the
/// targeted slot is never left empty (the conversation always advances).
async fn apply_answer(state: &mut IntakeState, slot: IntakeSlot, msg: &str, provider: Option<&str>) {
    if let Some(provider) = provider {
        let client = ConversationClient::new(provider);
        if let Some(delta) = extract_delta(slot, msg, &client).await {
            state.apply_delta(delta);
        }
    }
    if !state.slot_filled(slot) {
        state.apply_raw_answer(slot, msg);
    }
}

/// The next question text — an LLM-proposed phrasing when available, else the
/// deterministic wizard question already carried by [`NextStep::Ask`].
async fn enrich_question(
    state: &IntakeState,
    slot: IntakeSlot,
    wizard: &str,
    provider: Option<&str>,
) -> String {
    if let Some(provider) = provider {
        let client = ConversationClient::new(provider);
        if let Some(question) = propose_question(state, slot, &client).await {
            return question;
        }
    }
    wizard.to_string()
}

/// The "what we've captured so far" panel — the live slot snapshot the SPA
/// renders beside the chat.
fn captured(state: &IntakeState) -> Value {
    json!({
        "charter": state.charter,
        "persona": state.persona,
        "responsibilities": state.responsibilities,
        "cadence": state.cadence,
        "exceptions": state.exceptions,
        "success_criteria": state.success_criteria,
        "access_integrations": state.access_integrations,
        "escalation_failures_threshold": state.escalation_failures_threshold,
        "escalation_to": state.escalation_to,
    })
}

/// `true` when an optional-slot answer is an explicit skip ("skip", "not
/// sure", "no", "none", "n/a", "pass", "nothing").
fn is_skip(msg: &str) -> bool {
    let lower = msg.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "skip" | "not sure" | "no" | "none" | "n/a" | "na" | "pass" | "nothing" | "dunno"
    ) || lower.starts_with("skip ")
        || lower.starts_with("not sure")
}

/// The dialogue as a wire array of `{speaker, text}` for the chat history.
fn transcript(state: &IntakeState) -> Value {
    let turns: Vec<Value> = state
        .transcript
        .iter()
        .map(|t| {
            let speaker = match t.speaker {
                argo_genesis::Speaker::Assistant => "assistant",
                argo_genesis::Speaker::Operator => "operator",
            };
            json!({ "speaker": speaker, "text": t.text })
        })
        .collect();
    Value::Array(turns)
}

/// [`IntakeClient`] over the shared provider transport — the SAME machinery as
/// the archetype classifier / author / rubric grader (`complete_once`).
struct ConversationClient {
    provider_id: String,
}

impl ConversationClient {
    fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
        }
    }
}

#[async_trait]
impl IntakeClient for ConversationClient {
    async fn complete(&self, prompt: &str) -> ArgoResult<String> {
        complete_once(&self.provider_id, prompt).await
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    ("400 Bad Request", error_envelope("bad_request", message))
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        error_envelope("internal", message),
    )
}

fn error_envelope(code: &str, message: &str) -> String {
    json!({ "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
#[path = "routes_genesis_intake_conversation_tests.rs"]
mod tests;
