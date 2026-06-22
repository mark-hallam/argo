//! Genesis Phase-2 "Set boundaries" handling for the `/api/v1/genesis` SSE
//! stream — synthesize the server-truth echo and best-effort persist the
//! truth-boundary policy TOML next to the generated vertical.
//!
//! Split out of `routes_genesis_stream` to keep that file under the per-file
//! LOC ceiling and to co-locate boundaries handling. Input bounds on the
//! operator-supplied spec (`BoundariesSpec::validate_input_bounds`) live in
//! `argo_genesis::policy_synth_bounds`; the stream handler calls them inline as
//! a pre-synthesis `400` guard.

use argo_genesis::policy_synth::BoundariesSpec;
use std::path::Path;

/// Synthesize the boundaries echo for the `vertical_ready` payload and
/// best-effort persist the policy TOML next to the vertical.
///
/// Returns `None` when no boundaries were configured. The echo is the
/// server-truth summary of the synthesized truth-boundary policy (the SAME
/// synthesis the persisted policy uses), so the SPA renders what the server
/// produced, not its local form (no-theater). Disk persistence only happens
/// when the resolved tenant is a real UUID (the signed-in path) — the
/// anonymous/test fallback slug isn't a valid policy `tenant_id`, so we echo
/// without persisting rather than fabricating a tenant. Per-worker RUNTIME
/// loading of the persisted policy is DONE (PR #2801 + Wave-3 ⑩): the runner
/// resolves this exact file at session boot for the bound teammate, keyed on
/// the server-authoritative tenant + vertical and scoped to the teammate's own
/// tenant — see `argo_agent_runner::turn_helpers_truth_boundary`.
pub(super) fn persist_and_echo_boundaries(
    spec: Option<&BoundariesSpec>,
    tenant_id: &str,
    vertical_dir: &Path,
) -> Option<serde_json::Value> {
    let spec = spec?;
    if spec.is_empty() {
        return None;
    }
    if let Ok(uuid) = uuid::Uuid::parse_str(tenant_id) {
        if let Err(e) = argo_genesis::policy_synth::persist_policy(
            spec,
            argo_types::TenantId(uuid),
            vertical_dir,
        ) {
            tracing::warn!(
                target: "gateway::genesis_stream",
                error = %e,
                "boundaries policy persist failed; echoing synthesized spec only",
            );
        }
    }
    Some(argo_genesis::policy_synth::boundaries_echo(spec))
}
