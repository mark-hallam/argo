//! Genesis hire-commit safety-gate helpers (DEC-025 / Genesis P0).
//!
//! Extracted from `routes_workers_hire.rs` to keep that file under the
//! per-file LOC ceiling. Owns: the on-disk Genesis-vertical resolver, the
//! `422` blocked response shape, and the gate-verdict audit row. The fence
//! itself (the call site that runs the gate before `create_worker_from_draft`)
//! stays in `routes_workers_hire.rs`.

use argo_audit::{AuditKind, AuditSink};
use argo_genesis::GateVerdict;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// Audit-log redactor for the hire context. Delegates to
/// `argo_redact::redact_text` (Stream J2) so email / phone (incl.
/// `+CC`-prefixed international) / street-address sentinels stay in sync
/// with the canonical fence's pattern set.
pub(super) fn redact_text(input: &str) -> String {
    argo_redact::redact_text(input, &argo_redact::RedactionPolicy::default()).0
}

/// Recursively redact every string leaf in a JSON value.
pub(super) fn redact_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(redact_text(s)),
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), redact_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The operator's role-contract confirmation digest (DEC-025 #1 + #4), echoed
/// back from the `vertical_ready` SSE payload's `contract_digest` after the
/// operator reviewed the "what can this teammate do" contract. `None` (field
/// absent or empty) → the contract is unconfirmed and the hire gate holds in
/// `needs_review`, so the Genesis hire is refused. Accepts both
/// `confirmed_contract_digest` and the shorter `contract_digest` so SPA + SDK
/// callers can use either name.
pub(super) fn confirmed_contract_digest(parsed: &Value) -> Option<String> {
    ["confirmed_contract_digest", "contract_digest"]
        .iter()
        .find_map(|field| {
            parsed
                .get(*field)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
}

/// Run the synchronous Genesis hire gate for `vertical_id` and decide the
/// fate of the hire. Returns `Some(response)` (a `422`) when the teammate
/// must be refused, or `None` to let the hire proceed. Non-Genesis catalog
/// verticals are not gated → `None`. Always audits the verdict for a
/// Genesis vertical, pass or block.
///
/// `confirmed_digest` is the role-contract confirmation the operator echoed
/// back (DEC-025 #1 + #4): the gate derives the teammate's truthful contract
/// from the on-disk manifest and activates ONLY when this digest matches it.
/// `None` → the contract is unconfirmed and the gate holds in `needs_review`,
/// so the hire is refused — there is no activation without a confirmed
/// contract at this seam.
pub(super) async fn enforce_genesis_hire_gate(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    principal_id: &str,
    vertical_id: &str,
    confirmed_digest: Option<&str>,
) -> Option<(&'static str, String)> {
    let root = crate::handlers::vertical::resolve_verticals_dir();
    let tenant_str = tenant_id.to_string();
    // P5 (b) tenant-match enforcement (fail-closed): resolve ONLY under the
    // caller's own `.genesis/<tenant>/` segment. A vertical that exists for a
    // DIFFERENT tenant but not this one is a cross-tenant resolve attempt — we
    // refuse it (audited 403), never silently gate+hire another tenant's
    // teammate. A genuinely unknown id (exists nowhere) is `None` → the catalog
    // hire path handles it (this gate only covers Genesis verticals).
    let Some(genesis_dir) =
        super::genesis_tenant::genesis_vertical_dir_for_tenant(&root, &tenant_str, vertical_id)
    else {
        if super::genesis_tenant::genesis_vertical_exists_for_other_tenant(
            &root,
            &tenant_str,
            vertical_id,
        ) {
            emit_cross_tenant_denied_audit(store, tenant_id, principal_id, vertical_id).await;
            return Some(cross_tenant_denied_response(vertical_id));
        }
        return None;
    };
    let gate = super::routes_genesis_gate::run_hire_gate_from_dir_with_confirmation(
        &genesis_dir,
        confirmed_digest,
    )
    .await;
    emit_gate_audit(store, tenant_id, principal_id, vertical_id, &gate).await;
    if gate.state.allows_hire() {
        None
    } else {
        Some(gate_blocked_response(&gate))
    }
}

/// `true` when `id` resolves to a Genesis-generated vertical under the
/// caller's OWN tenant segment (`<verticals_root>/.genesis/<tenant>/<id>/`).
/// Tenant-scoped (P5 (b)): a vertical owned by a different tenant is not
/// visible here.
pub(super) fn genesis_vertical_exists(verticals_root: &Path, tenant_id: &str, id: &str) -> bool {
    super::genesis_tenant::genesis_vertical_exists_for_tenant(verticals_root, tenant_id, id)
}

/// `403 Forbidden` when a hire targets a Genesis vertical that belongs to a
/// DIFFERENT tenant (P5 (b) fail-closed cross-tenant denial). Distinct from
/// the `400 unknown vertical` an id-that-exists-nowhere gets, so a probe for
/// another tenant's teammate is legible in the audit + the response.
fn cross_tenant_denied_response(vertical_id: &str) -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({
            "error": {
                "code": "cross_tenant_vertical",
                "message": format!(
                    "vertical '{vertical_id}' is not available to this tenant"
                ),
            }
        })
        .to_string(),
    )
}

/// Audit a refused cross-tenant Genesis hire (tamper-evident security row).
async fn emit_cross_tenant_denied_audit(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    principal_id: &str,
    vertical_id: &str,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let payload = json!({
        "endpoint": "workers.hire.cross_tenant_denied",
        "vertical_id": vertical_id,
        "tenant_id": tenant_id.to_string(),
        "outcome": "denied",
    });
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(principal_id.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::workers_hire",
            error = %e,
            vertical_id = %vertical_id,
            "cross-tenant-denied audit write failed"
        );
    }
}

/// `422 Unprocessable Entity` when the Genesis hire gate refuses to
/// activate a teammate. Carries the gate state + the honest per-check
/// reasons so the SPA can surface "blocked / needs review" with the
/// failing checks instead of a generic error.
///
/// Shared with `routes_onboarding::first_hire_response` so all three
/// Genesis-activation seams (SSE, `POST /api/v1/workers`, and
/// `POST /api/v1/onboarding/first-hire`) emit the identical
/// `genesis_gate_failed` block shape and evidence.
pub(super) fn gate_blocked_response(gate: &GateVerdict) -> (&'static str, String) {
    // Surface the truthful role contract + its digest so the SPA can render
    // the "what can this teammate do" review screen on a `needs_review`
    // (awaiting-confirmation) outcome and echo `contract_digest` back to
    // confirm. On a `mismatch` the operator sees the confirmation is stale.
    let role_contract = gate
        .role_contract
        .as_ref()
        .and_then(|c| serde_json::to_value(c).ok())
        .unwrap_or(Value::Null);
    (
        "422 Unprocessable Entity",
        json!({
            "error": {
                "code": "genesis_gate_failed",
                "message": format!(
                    "teammate did not clear the Genesis safety gate ({})",
                    gate.state.as_str()
                ),
            },
            "role_contract": role_contract,
            "contract_digest": gate.contract_digest,
            "gate": {
                "state": gate.state.as_str(),
                "hireable": gate.state.allows_hire(),
                "eval_passed": gate.eval_passed,
                "eval_total": gate.eval_total,
                "eval_pass_rate": gate.eval_pass_rate,
                "comprehension_passed": gate.comprehension_passed,
                "comprehension_total": gate.comprehension_total,
                "comprehension_pass_rate": gate.comprehension_pass_rate,
                "safety_verdict": gate.safety_verdict,
                "contract_confirmation": gate.contract_confirmation,
                "contract_digest": gate.contract_digest,
                "reasons": gate.reasons,
            },
        })
        .to_string(),
    )
}

/// Audit the hire-commit gate verdict (tamper-evident row of what was
/// checked + the outcome) regardless of pass/block.
async fn emit_gate_audit(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    principal_id: &str,
    vertical_id: &str,
    gate: &GateVerdict,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let payload = json!({
        "endpoint": "workers.hire.genesis_gate",
        "vertical_id": vertical_id,
        "gate_state": gate.state.as_str(),
        "hireable": gate.state.allows_hire(),
        "gate_eval_passed": gate.eval_passed,
        "gate_eval_total": gate.eval_total,
        "gate_comprehension_passed": gate.comprehension_passed,
        "gate_comprehension_total": gate.comprehension_total,
        "gate_safety_verdict": gate.safety_verdict,
        "gate_contract_confirmation": gate.contract_confirmation,
        "gate_contract_digest": gate.contract_digest,
        "gate_reasons": gate.reasons,
    });
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::workers_hire",
            error = %e,
            vertical_id = %vertical_id,
            "genesis hire-gate audit write failed"
        );
    }
}
