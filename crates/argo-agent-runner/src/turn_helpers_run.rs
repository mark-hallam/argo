//! Per-round helpers extracted from
//! [`super::turn_helpers::run_turn_with_tools`] to keep that
//! orchestrator under the 80-LOC budget. Each helper owns one
//! phase of the tool-dispatch loop.
//!
//! No public API changes — every symbol here is `pub(super)` and
//! only consumed by `turn_helpers::run_turn_with_tools`.

use super::turn_helpers_truth_boundary::{
    default_claim_provenance, load_truth_boundary_ctx, TruthBoundaryTurnCtx,
};
use super::turn_helpers_gate::{emit_deny, emit_escalation_required, emit_probation_hold};
use super::turn_skills_runtime::PendingAbOutcome;
use super::{build_system_prompt_with_ab, ContextWindowPolicy, TurnResult};
use crate::dispatch::{build_dispatch_ctx, dispatch_one_with_runtime, provider_schemas_filtered};
use crate::helpers::{
    build_assistant_content, emit_event, record_turn_spend, stream_one_round, turn_cost_usd,
    RoundResp, RunEventSink, SpendCtx,
};
use crate::transport::load_manifest;
use crate::turn_vertical::{consume_one_shot_if_present, SessionGrantState};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_identity::{MemoryNs, MemoryScope};
use argo_orchestrator::HandoffId;
use argo_provider::{ProviderRequest, ToolSchema, Transport};
use argo_store::SessionStore;
use argo_tool::ToolRegistry;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, MessageRole, Principal, SessionId, ToolCallRequest,
    ToolCallResult,
};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Per-turn dispatch state — built once per call to
/// `run_turn_with_tools` and then reused across rounds.
pub(super) struct TurnDispatchCtx {
    pub(super) registry: ToolRegistry,
    pub(super) engine: CapabilityEngine,
    pub(super) principal: Principal,
    pub(super) background_processes: Arc<argo_tool::ProcessRegistry>,
    pub(super) tools: Vec<ToolSchema>,
    pub(super) system_prompt: Option<String>,
    pub(super) context_policy: ContextWindowPolicy,
    pub(super) truth_boundary: TruthBoundaryTurnCtx,
    pub(super) orch_session_id: argo_orchestrator::SessionId,
    pub(super) current_handoff_id: Option<HandoffId>,
    /// DEC-025 #2 Genesis P3 — whether the bound worker is still inside
    /// its probation window. Resolved once at session boot; consulted on
    /// every tool dispatch to force approval-gating during probation.
    pub(super) probation: crate::turn_vertical_fence::ProbationStatus,
    /// cu1b (DEC-050 G1): the file-tool jail root for this turn — `fs_root`
    /// when the worker has a sandbox, else the home `cwd`. cu1 redirected the
    /// build-time cwd (registry/engine) to this; the DISPATCH-time ToolContext
    /// (what `path_guard::confine_path` checks) must use the SAME root, or the
    /// granted `Fs*{fs_root/**}` capability is unreachable behind a repo-cwd
    /// jail. Manifest/memory/truth-boundary still load from the home `cwd`.
    pub(super) dispatch_cwd: std::path::PathBuf,
}

/// Assemble the dispatch registry, system prompt, tools, and
/// context-window policy for `run_turn_with_tools`. Pulls in the
/// `MemoryStore`, optional per-worker memory namespace, the
/// orchestrator store, and any vertical prompt prefix.
#[allow(clippy::too_many_arguments)]
pub(super) async fn build_turn_dispatch_ctx(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    model: &str,
    spend: &SpendCtx<'_>,
    vertical_prompt: Option<&str>,
    vertical_scope: Option<&str>,
    vertical_deny: &std::collections::HashSet<String>,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
    pending_ab_outcomes: &mut Vec<PendingAbOutcome>,
) -> ArgoResult<TurnDispatchCtx> {
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", session_id.0)))?;
    let context_policy = match load_manifest(cwd, &session.provider) {
        Ok(manifest) => ContextWindowPolicy::from_config(
            super::load_context_compress_config(cwd)?,
            super::model_context_window(&manifest, model),
        ),
        Err(_) => ContextWindowPolicy::disabled(),
    };
    // Default activation reads `ARGO_TRUTH_BOUNDARY_POLICY_PATH` (env-var
    // only). A Genesis-hired teammate overrides this below with its
    // conservative bound-at-birth policy once the worker-row read resolves
    // (P5 (a)).
    let mut truth_boundary = load_truth_boundary_ctx(cwd, session.tenant_id)?;
    // claude-memory-tool-wire: thread the live MemoryStore + workspace into
    // dispatch so the in-core `memory` tool can persist. WS1 tenant isolation
    // (3a): bind to the SESSION's OWN tenant workspace
    // (`first_workspace_for_tenant`), NOT the global demo/cwd one
    // `first_workspace()` returns — fail CLOSED so we never leak another tenant.
    let memory_ctx = store
        .first_workspace_for_tenant(session.tenant_id)
        .await?
        .map(|ws| {
            (
                Arc::new(argo_memory::MemoryStore::new(store.clone())),
                ws,
                vertical_scope.map(str::to_string),
            )
        });
    // codex-memory-tool-namespace-wiring (F-03 P1) + fix-worker-run-memory-
    // namespace: bind `memory` to a per-worker `MemoryNs`. Ground on the
    // PERSISTED `session.worker_id` (fallback) so the work-dispatch "Run now"
    // path binds even if the threaded param is dropped (DEC-041 fence below).
    let memory_namespace = worker_id
        .or(session.worker_id)
        .map(|wid| MemoryNs::new(session.tenant_id, wid, MemoryScope::WorkerSelf));
    let orchestrator_ctx = Arc::new(argo_orchestrator::OrchestratorStore::new(store.clone()));
    let orch_session_id = argo_orchestrator::SessionId::from_string(session_id.to_string());
    let current_handoff_id = super::turn_helpers_run_support::current_inbound_handoff_id(
        &orchestrator_ctx,
        session.tenant_id,
        &orch_session_id,
        worker_id,
    )
    .await?;
    // DEC-025 Genesis capability fence: a Genesis-hired teammate must NOT
    // inherit the owner's `*` keys. The fence keys off the worker's Genesis
    // PROVENANCE (`persona.provenance_chain`), NOT where its manifest lives,
    // so a first-hire'd / catalog-bound Genesis worker is fenced like an
    // SSE-hired `.genesis/` one; no-provenance workers keep `owner`.
    // `turn_vertical_fence` fails CLOSED if the manifest can't be resolved.
    //
    // DEC-025 #2 Genesis P3 probation: the SAME worker-row read also yields
    // whether the teammate is still inside its 24h probation window. While
    // it is, `resolve_tool_call` forces EVERY action approval-gated (held
    // pending, never executed unsupervised) regardless of its normal policy.
    // Read once here at session boot — never per-action on the hot loop.
    //
    // DEC-025 #6 Genesis P5 (a) truth-boundary bound at birth: the SAME
    // worker-row read also yields whether the teammate carries the
    // `truth_boundary_policy_id=` marker. When it does, bind the teammate's
    // claim policy scoped to the worker's tenant — replacing env-var-only
    // activation so a generated teammate cannot over-claim to a customer.
    //
    // wire-genesis-policy-per-worker-runtime-loading: prefer the operator's
    // SYNTHESIZED per-vertical policy (the access-control panel's output,
    // persisted next to the vertical) when one exists for this server-resolved
    // tenant + vertical, so the operator's per-teammate guardrails ACTUALLY
    // bite at runtime. Falls back to the conservative default when no
    // per-vertical policy is on disk.
    use crate::turn_vertical_fence as fence;
    let signals = fence::genesis_signals_for_worker(store, session.tenant_id, worker_id).await;
    if matches!(
        signals.truth_boundary,
        fence::TruthBoundaryBinding::Bound
    ) {
        truth_boundary = super::turn_helpers_truth_boundary::genesis_truth_boundary_ctx_for_worker(
            session.tenant_id,
            vertical_scope,
        );
    }
    // cu1 (DEC-050 G1): bind the worker's scoped sandbox root onto the derived
    // profile so `to_capability_set` emits `FsRead`/`FsWrite{<fs_root>/**}` for
    // whichever file tools the manifest allows. No sandbox → no `Fs*` cap
    // (default-deny preserved). A deny-all (fail-closed) profile carries no
    // `file_*` tool, so attaching a root to it still emits nothing.
    let fs_root_path = signals.fs_root.as_deref().map(Path::new);
    let worker_profile = fence::resolve_worker_profile_for_session(
        session.tenant_id,
        vertical_scope,
        signals.provenance,
    )
    .map(|p| p.with_fs_root(fs_root_path));
    // The file tools jail to the dispatch `cwd`. When the worker has a sandbox,
    // jail the tools to `fs_root` so the physical jail COINCIDES with the
    // granted glob — a read/write the capability fence permits resolves to a
    // real path inside the sandbox. Without a sandbox, the historical `cwd`
    // default is unchanged. `cwd` is still used above for manifest / memory /
    // truth-boundary loading (those live in the home dir, not the sandbox).
    let dispatch_cwd: &Path = fs_root_path.unwrap_or(cwd);
    let dispatch_cwd_owned = dispatch_cwd.to_path_buf();
    let (registry, engine, principal) = build_dispatch_ctx(
        dispatch_cwd,
        memory_ctx,
        memory_namespace,
        Arc::clone(&orchestrator_ctx),
        Arc::clone(store),
        worker_profile.as_ref(),
        // DEC-041 (D): fence to deny-all (never `owner`) whenever this session
        // acts as a worker — i.e. it carries a `worker_id`. A bare human
        // operator session (no worker_id) keeps `owner`.
        session.worker_id.is_some(),
    )?;
    let background_processes =
        super::turn_helpers_run_support::process_registry_for(session.tenant_id, session_id);
    // #721: hide manifest-denied tools from the model's tool list.
    let tools = provider_schemas_filtered(&registry, vertical_deny);
    let (mut system_prompt, ab_pending) = build_system_prompt_with_ab(
        store,
        session_id,
        vertical_scope,
        spend.tenant_id,
        worker_id,
        invocation_id,
    )
    .await?;
    pending_ab_outcomes.extend(ab_pending);
    if let Some(vp) = vertical_prompt {
        let merged = match system_prompt.take() {
            Some(skill_prompt) => format!("{vp}\n\n---\n\n{skill_prompt}"),
            None => vp.to_string(),
        };
        system_prompt = Some(merged);
    }
    Ok(TurnDispatchCtx {
        registry,
        engine,
        principal,
        background_processes,
        tools,
        system_prompt,
        context_policy,
        truth_boundary,
        orch_session_id,
        current_handoff_id,
        probation: signals.probation,
        dispatch_cwd: dispatch_cwd_owned,
    })
}

/// Outcome of a single provider call — either the assistant gave a
/// final text response (no tool calls) or it issued one-or-more
/// tool calls that need dispatching.
pub(super) struct RoundOutcome {
    pub(super) assistant_id: Uuid,
    pub(super) text: String,
    pub(super) declared_claim_classes: Vec<String>,
    pub(super) claim_provenance: argo_capability::ClaimProvenance,
    pub(super) tokens_in: Option<u32>,
    pub(super) tokens_out: Option<u32>,
    pub(super) tool_calls: Vec<ToolCallRequest>,
}

/// Issue one provider round: call the transport (streamed or
/// blocking), persist the assistant message, write the audit/event
/// pair, and record spend. Returns the parsed
/// [`RoundOutcome`] for the caller to inspect.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_one_round(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    req: ProviderRequest,
    stream: bool,
    round: usize,
) -> ArgoResult<RoundOutcome> {
    let resp = if stream {
        stream_one_round(transport, &req).await?
    } else {
        let r = transport.complete(&req).await?;
        RoundResp {
            text: r.text.unwrap_or_default(),
            tool_calls: r.tool_calls,
            tokens_in: r.tokens_in,
            tokens_out: r.tokens_out,
        }
    };
    let assistant_content = build_assistant_content(&resp.text, &resp.tool_calls);
    let assistant_id = store
        .append_message(
            session_id,
            MessageRole::Assistant,
            &assistant_content.to_string(),
        )
        .await?;
    // UF A7 — stamp the turn's USD cost onto the audit row so the SPA
    // activity feed (`ActivityRow.tsx`) renders a cost chip for normal
    // teammate work, not just forge/eval. `None` (mock provider / no
    // pricing) emits a JSON `null`, never a fabricated zero (NO THEATER).
    let cost_usd = turn_cost_usd(spend, resp.tokens_in, resp.tokens_out);
    let assistant_payload = serde_json::json!({
        "message_id": assistant_id,
        "round": round,
        "tool_calls": resp.tool_calls.len(),
        "tokens_in": resp.tokens_in,
        "tokens_out": resp.tokens_out,
        "cost_usd": cost_usd,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::AssistantMessage,
            assistant_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "AssistantMessage", assistant_payload);
    record_turn_spend(store, session_id, spend, resp.tokens_in, resp.tokens_out).await?;
    crate::turn_budget::record_subtask_turn_spend(
        store,
        session_id,
        spend,
        resp.tokens_in,
        resp.tokens_out,
    )
    .await?;
    Ok(RoundOutcome {
        assistant_id,
        text: resp.text,
        declared_claim_classes: Vec::new(),
        claim_provenance: default_claim_provenance(),
        tokens_in: resp.tokens_in,
        tokens_out: resp.tokens_out,
        tool_calls: resp.tool_calls,
    })
}

impl RoundOutcome {
    pub(super) fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }
}

/// Dispatch a single tool call through the deny/approval/grant
/// gates and persist the audit + ToolResult message. Mutates
/// `session_grants` when a one-shot grant is consumed.
#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch_one_tool_call(
    ctx: &TurnDispatchCtx,
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: &mut SessionGrantState,
    call: &ToolCallRequest,
) -> ArgoResult<()> {
    let call_payload = serde_json::json!({"tool": call.name, "call_id": call.id});
    audit
        .write(
            Some(session_id),
            AuditKind::ToolCall,
            call_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "ToolCall", call_payload);
    let result = resolve_tool_call(
        ctx,
        store,
        cwd,
        session_id,
        audit,
        spend,
        events,
        vertical_deny,
        vertical_approval,
        session_grants,
        call,
    )
    .await?;
    let result_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "is_error": result.is_error,
        "duration_ms": result.duration_ms,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::ToolResult,
            result_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "ToolResult", result_payload);
    let tool_msg = serde_json::json!([ContentBlock::ToolResult(result)]);
    store
        .append_message(session_id, MessageRole::Tool, &tool_msg.to_string())
        .await?;
    Ok(())
}

/// Apply the deny-wins → escalation-gate → grant-override → dispatch
/// flow. Deny is the platform's hard "never"; the escalation gate
/// pauses for human approval unless the session already has a
/// matching grant (one-shot grants consume after dispatch).
#[allow(clippy::too_many_arguments)]
async fn resolve_tool_call(
    ctx: &TurnDispatchCtx,
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: &mut SessionGrantState,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    // DEC-025 #2 Genesis P3 — the gate cascade (deny ▸ probation ▸
    // manifest-approval ▸ dispatch) is a single pure decision so the
    // precedence is testable in isolation. Probation gates the WHOLE
    // action vocabulary above the manifest-approval set; an explicit
    // operator grant (`argo session approve`) lets an otherwise-held call
    // through and is consumed (one-shot) after dispatch.
    let approved_via_grant = match super::turn_helpers_gate::decide_gate(
        &call.name,
        ctx.probation,
        vertical_deny,
        vertical_approval,
        session_grants,
    ) {
        super::turn_helpers_gate::GateDecision::Deny => {
            return emit_deny(session_id, audit, events, call).await;
        }
        super::turn_helpers_gate::GateDecision::ProbationHold => {
            return emit_probation_hold(session_id, audit, events, call).await;
        }
        super::turn_helpers_gate::GateDecision::EscalationRequired => {
            return emit_escalation_required(session_id, audit, events, call).await;
        }
        super::turn_helpers_gate::GateDecision::Dispatch { consume_grant } => consume_grant,
    };
    // tg1 — make a silent capability-fence deny LEGIBLE. The vertical
    // gate above said "dispatch", but the worker's derived capability
    // profile may still refuse this tool at the `engine.check` dispatch
    // seam. Record a pending `capability_request` BEFORE dispatch so the
    // connect-on-demand grant card learns "this teammate needed
    // <connector>". Observe-only: dispatch below is unchanged and still
    // returns its own deny to the model.
    super::turn_helpers_capability_request::maybe_record_capability_request(
        ctx, store, session_id, audit, events, call,
    )
    .await;
    let res = dispatch_one_with_runtime(
        &ctx.registry,
        &ctx.engine,
        &ctx.principal,
        spend.tenant_id,
        cwd,
        &ctx.orch_session_id,
        call,
        Some(ctx.background_processes.clone()),
        ctx.current_handoff_id.clone(),
    )
    .await;
    if approved_via_grant {
        consume_one_shot_if_present(store, session_id, &call.name, session_grants, audit, events)
            .await?;
    }
    Ok(res)
}

/// Emit the `Halt` audit + event when the round-budget is reached.
/// Caller still owns building the final [`TurnResult`].
pub(super) async fn emit_halt(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    max_rounds: usize,
) -> ArgoResult<()> {
    let halt_payload = serde_json::json!({
        "reason": "tool_budget_exhausted",
        "max_rounds": max_rounds,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::Halt,
            halt_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "Halt", halt_payload);
    Ok(())
}

/// Wrap the round outcome's text/tokens into the no-tool-call exit
/// [`TurnResult`] so the parent doesn't have to repeat the struct
/// literal in two places.
pub(super) fn finalize_turn(outcome: RoundOutcome) -> TurnResult {
    TurnResult {
        assistant_id: outcome.assistant_id,
        text: outcome.text,
        tokens_in: outcome.tokens_in,
        tokens_out: outcome.tokens_out,
        stopped_on_tool_budget: false,
    }
}
