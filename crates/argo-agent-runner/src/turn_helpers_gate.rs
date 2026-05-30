//! Dispatch-gate audit emitters extracted from
//! [`super::turn_helpers_run`] to keep that file under the 500-LOC
//! ceiling (HARD RULE 6).
//!
//! Each helper turns a blocked tool dispatch into (a) one audit row + one
//! run-event of the gate's kind, and (b) a synthetic error
//! [`ToolCallResult`] the loop surfaces to the model so the agent learns
//! the boundary instead of seeing a silent no-op. The three gates, in the
//! precedence `resolve_tool_call` applies them:
//!
//! 1. [`emit_deny`] — the platform's hard "never" (`[tools].denied`).
//! 2. [`emit_probation_hold`] — DEC-025 #2 Genesis P3: a freshly-hired
//!    teammate's WHOLE action vocabulary is approval-gated until its
//!    probation window clears.
//! 3. [`emit_escalation_required`] — a specific tool the manifest marked
//!    `[escalation].human_approval_required`.
//!
//! Probation and escalation are deliberately distinct audit kinds so
//! forensic readers can tell "brand-new teammate on probation" apart from
//! "this tool always needs sign-off" (observability, priority #2).

use crate::helpers::{emit_event, RunEventSink};
use crate::turn_vertical::SessionGrantState;
use crate::turn_vertical_fence::ProbationStatus;
use argo_audit::{AuditKind, AuditSink};
use argo_types::{ArgoResult, SessionId, ToolCallRequest, ToolCallResult};
use std::collections::HashSet;

/// The verdict the gate cascade reaches for one tool call, BEFORE any
/// audit I/O or dispatch. Pulled out as a pure value so the precedence
/// (`deny` ▸ `probation` ▸ `manifest-approval` ▸ `dispatch`) is testable
/// in isolation — the adversarial bar for Genesis P3 is "no path
/// bypasses probation", which is exactly a precedence property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GateDecision {
    /// Tool is in the vertical's `[tools].denied` list — hard never.
    Deny,
    /// Worker is inside its Genesis P3 probation window and holds no
    /// grant for this tool — held pending (DEC-025 #2).
    ProbationHold,
    /// Tool is manifest-declared approval-required and holds no grant —
    /// held pending.
    EscalationRequired,
    /// All gates cleared — dispatch. `consume_grant` is `true` when a
    /// gate was satisfied by an operator grant a one-shot must consume
    /// after dispatch (manifest-approval OR active probation).
    Dispatch { consume_grant: bool },
}

/// Pure gate cascade for one tool call. Order is load-bearing:
///
/// 1. **Deny** wins over everything (the platform's hard never).
/// 2. **Probation** gates the WHOLE action vocabulary while the window
///    holds — it sits ABOVE the manifest-approval set so a brand-new
///    teammate cannot execute an un-marked tool unsupervised. An explicit
///    operator grant (`session_grants.allows`) lets the action through.
/// 3. **Manifest-approval** gates the specific tools a vertical marked.
/// 4. Otherwise **dispatch**.
///
/// A grant only matters once a higher gate would otherwise hold the call;
/// when it does, and the gate was probation OR manifest-approval, the
/// dispatch records `consume_grant = true` so a one-shot is spent.
#[must_use]
pub(super) fn decide_gate(
    tool_name: &str,
    probation: ProbationStatus,
    vertical_deny: &HashSet<String>,
    vertical_approval: &HashSet<String>,
    session_grants: &SessionGrantState,
) -> GateDecision {
    if vertical_deny.contains(tool_name) {
        return GateDecision::Deny;
    }
    let has_grant = session_grants.allows(tool_name);
    if probation.is_active() && !has_grant {
        return GateDecision::ProbationHold;
    }
    let manifest_gated = vertical_approval.contains(tool_name);
    if manifest_gated && !has_grant {
        return GateDecision::EscalationRequired;
    }
    // A grant was the thing that let an otherwise-gated call through.
    let consume_grant = has_grant && (probation.is_active() || manifest_gated);
    GateDecision::Dispatch { consume_grant }
}

/// Emit the `RbacDeny` audit + event and return an error result when a
/// tool is in the bound vertical's `[tools].denied` list (the platform's
/// hard "never").
pub(super) async fn emit_deny(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    let deny_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "denied_by": "vertical_manifest",
    });
    audit
        .write(
            Some(session_id),
            AuditKind::RbacDeny,
            deny_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "RbacDeny", deny_payload);
    Ok(argo_tool::error_text(
        call.id,
        format!(
            "tool '{}' denied by the bound vertical's manifest [tools].denied list",
            call.name
        ),
        0,
    ))
}

/// Emit the `EscalationRequired` audit + event and return a held-pending
/// error result when a tool is declared in the bound vertical's
/// `[escalation].human_approval_required` list and the session holds no
/// matching grant.
pub(super) async fn emit_escalation_required(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    let escalation_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "reason": "human_approval_required",
        "source": "vertical_manifest",
    });
    audit
        .write(
            Some(session_id),
            AuditKind::EscalationRequired,
            escalation_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "EscalationRequired", escalation_payload);
    Ok(argo_tool::error_text(
        call.id,
        format!(
            "tool '{}' requires human approval (declared in the bound vertical's manifest [escalation].human_approval_required); run `argo session approve <session-id> --tool {}` (add --once for a single-dispatch grant) and re-issue the turn",
            call.name, call.name
        ),
        0,
    ))
}

/// Emit the `ProbationHold` audit + event and return a held-pending error
/// result when a tool dispatch is blocked because the worker is inside
/// its Genesis P3 probation window (DEC-025 #2). Distinct from
/// [`emit_escalation_required`] (manifest-declared approval) so the audit
/// chain + approvals inbox can tell "held because brand-new teammate on
/// probation" apart from "held because this specific tool needs sign-off".
pub(super) async fn emit_probation_hold(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    let payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "reason": "probation_window_active",
        "source": "genesis_p3_probation",
    });
    audit
        .write(
            Some(session_id),
            AuditKind::ProbationHold,
            payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "ProbationHold", payload);
    Ok(argo_tool::error_text(
        call.id,
        format!(
            "tool '{}' is held: this teammate is in its Genesis probation window, so every action requires human approval until probation ends. Run `argo session approve <session-id> --tool {}` (add --once for a single-dispatch grant) and re-issue the turn, or wait for probation to clear.",
            call.name, call.name
        ),
        0,
    ))
}

#[cfg(test)]
mod gate_decision_tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn grant(tool: &str) -> SessionGrantState {
        let mut g = SessionGrantState::default();
        g.persistent.insert(tool.to_string());
        g
    }

    fn one_shot(tool: &str) -> SessionGrantState {
        let mut g = SessionGrantState::default();
        g.one_shot_remaining.insert(tool.to_string(), 1);
        g
    }

    const EMPTY: &[&str] = &[];

    #[test]
    fn no_probation_undeclared_tool_dispatches() {
        // Baseline: a non-probation worker with an un-gated tool runs.
        let d = decide_gate(
            "draft_reply",
            ProbationStatus::Normal,
            &set(EMPTY),
            &set(EMPTY),
            &SessionGrantState::default(),
        );
        assert_eq!(d, GateDecision::Dispatch { consume_grant: false });
    }

    #[test]
    fn probation_holds_every_action_even_undeclared_ones() {
        // THE CORE PROPERTY: a probationary worker is held on a tool that
        // appears in NEITHER the deny set NOR the approval set — probation
        // gates the whole vocabulary, so there is no "un-marked tool"
        // bypass. This is the multi-seam hole the brief warns about,
        // proven shut at the decision layer.
        for tool in ["draft_reply", "send_email", "anything_at_all"] {
            let d = decide_gate(
                tool,
                ProbationStatus::InProbation,
                &set(EMPTY),
                &set(EMPTY),
                &SessionGrantState::default(),
            );
            assert_eq!(d, GateDecision::ProbationHold, "tool {tool} must be held");
        }
    }

    #[test]
    fn deny_wins_over_probation() {
        // Even during probation, a denied tool reports Deny (the hard
        // never), not ProbationHold — deny is the top of the cascade.
        let d = decide_gate(
            "terminal",
            ProbationStatus::InProbation,
            &set(&["terminal"]),
            &set(EMPTY),
            &grant("terminal"), // a grant must NOT override a hard deny
        );
        assert_eq!(d, GateDecision::Deny);
    }

    #[test]
    fn operator_grant_lets_a_probation_action_through_and_consumes() {
        // The documented escape hatch: an explicit `argo session approve`
        // grant lets a probationary action dispatch, and a one-shot is
        // marked for consumption.
        let d = decide_gate(
            "draft_reply",
            ProbationStatus::InProbation,
            &set(EMPTY),
            &set(EMPTY),
            &one_shot("draft_reply"),
        );
        assert_eq!(d, GateDecision::Dispatch { consume_grant: true });
    }

    #[test]
    fn manifest_approval_still_gates_after_probation_clears() {
        // Post-probation, the worker graduates to its NORMAL policy — a
        // manifest-approval-required tool is still gated; an un-marked one
        // dispatches. Proves probation clearing doesn't disable the
        // ordinary approval gate.
        let approval = set(&["send_email"]);
        let held = decide_gate(
            "send_email",
            ProbationStatus::Normal,
            &set(EMPTY),
            &approval,
            &SessionGrantState::default(),
        );
        assert_eq!(held, GateDecision::EscalationRequired);

        let ran = decide_gate(
            "draft_reply",
            ProbationStatus::Normal,
            &set(EMPTY),
            &approval,
            &SessionGrantState::default(),
        );
        assert_eq!(ran, GateDecision::Dispatch { consume_grant: false });
    }

    #[test]
    fn manifest_grant_consumes_only_for_the_gated_tool() {
        // A grant on a manifest-gated tool dispatches + consumes; the same
        // grant on a non-gated tool dispatches WITHOUT consuming (nothing
        // was gating it).
        let approval = set(&["send_email"]);
        let gated = decide_gate(
            "send_email",
            ProbationStatus::Normal,
            &set(EMPTY),
            &approval,
            &one_shot("send_email"),
        );
        assert_eq!(gated, GateDecision::Dispatch { consume_grant: true });

        let ungated = decide_gate(
            "draft_reply",
            ProbationStatus::Normal,
            &set(EMPTY),
            &approval,
            &one_shot("draft_reply"),
        );
        assert_eq!(ungated, GateDecision::Dispatch { consume_grant: false });
    }
}
