//! `vertical_ready` SSE payload + audit row for `routes_genesis_stream`.
//!
//! Extracted from `routes_genesis_stream.rs` to keep that file under the
//! per-file LOC ceiling. Both functions surface the REAL hire-gate verdict
//! (DEC-025 / Genesis P0) — no longer a hardcoded `judge_verdict:"pass"`.

use argo_audit::{AuditKind, AuditSink};
use argo_genesis::{GateVerdict, GenesisPlan};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// Build the final `vertical_ready` payload carrying the REAL gate verdict.
///
/// `judge_verdict` is the composed gate state (`pass` | `blocked` |
/// `needs_review`); `hireable` is the single boolean the SPA gates the Hire
/// button on (only `pass` allows hire). The `gate` block is honest evidence
/// (eval pass-rate, safety verdict, reasons, probation window) the operator
/// can see.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_final_payload(
    plan: &GenesisPlan,
    vertical_dir: &Path,
    tenant_id: &str,
    display_name: &str,
    eval_seed_count: usize,
    gate: &GateVerdict,
    dry_run_until: Option<&str>,
) -> Value {
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.len() as u64)
        .unwrap_or(0);
    // The truthful, operator-confirmable role contract (DEC-025 #4) — derived
    // from the teammate's ACTUAL bound capability profile. The SPA renders
    // this as the "what can this teammate do" review screen and echoes
    // `contract_digest` back at the hire-commit call to confirm activation.
    // A freshly-generated teammate is always `unconfirmed` here (the operator
    // has not yet seen the contract), so a fresh SSE teammate is `needs_review`
    // and NOT hireable until confirmed — human-review-before-activation (#1).
    let role_contract = gate
        .role_contract
        .as_ref()
        .and_then(|c| serde_json::to_value(c).ok())
        .unwrap_or(Value::Null);
    json!({
        "vertical_id": plan.vertical_id,
        "vertical_dir_path": vertical_dir.to_string_lossy(),
        "tenant_id": tenant_id,
        "manifest": {
            "id": plan.vertical_id,
            "name": display_name,
            "archetype": plan.archetype.slug(),
            "persona": plan.persona,
            "actions_declared": actions,
        },
        "prompts": [
            "prompts/system.md",
            "prompts/persona.md",
        ],
        "skills": [],
        "eval_seed_count": eval_seed_count,
        "judge_verdict": gate.state.as_str(),
        "hireable": gate.state.allows_hire(),
        "role_contract": role_contract,
        "contract_digest": gate.contract_digest,
        "gate": {
            "state": gate.state.as_str(),
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
            "dry_run_until": dry_run_until,
        },
    })
}

/// Audit the synthesis + the REAL gate verdict so every Genesis run carries
/// a tamper-evident row of what was checked and why it passed / was blocked
/// / degraded (DEC-025 observability).
pub(super) async fn emit_audit(
    store: Option<&Arc<SessionStore>>,
    tenant_id: &str,
    vertical_id: &str,
    eval_seed_count: usize,
    gen_mode: &str,
    gate: &GateVerdict,
) {
    let Some(store) = store else { return };
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let payload = json!({
        "endpoint": "genesis_stream",
        "vertical_id": vertical_id,
        "eval_seed_count": eval_seed_count,
        "generation": gen_mode,
        "gate_state": gate.state.as_str(),
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
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
    {
        tracing::warn!(
            target: "gateway::genesis_stream",
            tenant = %tenant_id,
            error = %e,
            "genesis stream audit write failed"
        );
    }
}
