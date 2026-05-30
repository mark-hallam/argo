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
use std::path::{Path, PathBuf};
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

/// Run the synchronous Genesis hire gate for `vertical_id` and decide the
/// fate of the hire. Returns `Some(response)` (a `422`) when the teammate
/// must be refused, or `None` to let the hire proceed. Non-Genesis catalog
/// verticals are not gated → `None`. Always audits the verdict for a
/// Genesis vertical, pass or block.
pub(super) async fn enforce_genesis_hire_gate(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    principal_id: &str,
    vertical_id: &str,
) -> Option<(&'static str, String)> {
    let genesis_dir = genesis_vertical_dir(vertical_id)?;
    let gate = super::routes_genesis_gate::run_hire_gate_from_dir(&genesis_dir).await;
    emit_gate_audit(store, tenant_id, principal_id, vertical_id, &gate).await;
    if gate.state.allows_hire() {
        None
    } else {
        Some(gate_blocked_response(&gate))
    }
}

/// Resolve the on-disk directory of a Genesis-generated vertical
/// (`<root>/.genesis/<tenant_slug>/<id>/`) using the default verticals
/// root. Returns `None` for hand-authored catalog verticals (which Genesis
/// P0's gate does not cover) or when the id isn't a Genesis vertical.
fn genesis_vertical_dir(id: &str) -> Option<PathBuf> {
    let root = crate::handlers::vertical::resolve_verticals_dir();
    genesis_vertical_dir_in(&root, id)
}

/// `true` when `id` resolves to a Genesis-generated vertical under
/// `<verticals_root>/.genesis/<tenant>/<id>/`.
pub(super) fn genesis_vertical_exists(verticals_root: &Path, id: &str) -> bool {
    genesis_vertical_dir_in(verticals_root, id).is_some()
}

fn genesis_vertical_dir_in(verticals_root: &Path, id: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(verticals_root.join(".genesis")).ok()?;
    entries.flatten().find_map(|e| {
        let dir = e.path().join(id);
        (dir.join("vertical.toml").is_file()).then_some(dir)
    })
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
            "gate": {
                "state": gate.state.as_str(),
                "hireable": gate.state.allows_hire(),
                "eval_passed": gate.eval_passed,
                "eval_total": gate.eval_total,
                "eval_pass_rate": gate.eval_pass_rate,
                "safety_verdict": gate.safety_verdict,
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
        "gate_safety_verdict": gate.safety_verdict,
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
