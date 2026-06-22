//! Genesis-teammate truth-boundary context BUILDERS, extracted from
//! [`super::turn_helpers_truth_boundary`] to keep that file under the 500-LOC
//! HARD RULE. These decide WHICH policy a Genesis-hired teammate is governed by
//! at session boot — the operator's synthesized per-teammate policy when one is
//! configured, else the conservative default — keyed on the server-authoritative
//! `tenant_id` + `vertical_id` and scoped to the teammate's own tenant.

use super::turn_helpers_truth_boundary::TruthBoundaryTurnCtx;
use argo_capability::{ChannelKind, RefusalTemplateRegistry};
use argo_types::TenantId;
use std::path::Path;

/// Build the truth-boundary gate for a Genesis-generated teammate (DEC-025
/// #6 / Genesis P5 part (a)). Binds `argo_capability::genesis_default_policy`
/// scoped to `tenant_id`, so the teammate's declared financial/legal/medical/
/// factual claims **escalate** (queue an approval, hold the reply) regardless
/// of the process-global `ARGO_TRUTH_BOUNDARY_POLICY_PATH`.
///
/// Channel is [`ChannelKind::Chat`] (the conservative all-channels default
/// for a generated teammate); the gate's per-turn tenant guard
/// (`active_policy_for`) still applies the policy only to `tenant_id`'s
/// sessions, so this can never gate another tenant. No refusal templates are
/// needed — every gated class is `Escalate`, not `Forbid`.
#[must_use]
pub(in crate::turn) fn genesis_default_truth_boundary_ctx(
    tenant_id: TenantId,
) -> TruthBoundaryTurnCtx {
    TruthBoundaryTurnCtx::new(
        argo_capability::genesis_default_policy(tenant_id),
        ChannelKind::Chat,
        RefusalTemplateRegistry::empty(),
    )
}

/// Build the truth-boundary gate for a Genesis-hired teammate, preferring the
/// operator's SYNTHESIZED per-vertical policy over the conservative default
/// (Genesis P5 (a) + Wave-3 ⑩ per-worker policy routing).
///
/// At session boot we try to load the policy the access-control panel persisted
/// next to the worker's vertical, keyed on the server-authoritative
/// `tenant_id` + `vertical_id` (NEVER a client field). When it loads, the
/// operator's ACTUAL boundaries (e.g. a read-only financial scope, a configured
/// dollar ceiling, a custom escalation topic) bite at runtime. When no
/// per-vertical policy exists, we fall back to
/// [`genesis_default_truth_boundary_ctx`] — the conservative default — so a
/// Genesis-bound worker is never LESS guarded than before this wiring landed.
#[must_use]
pub(in crate::turn) fn genesis_truth_boundary_ctx_for_worker(
    tenant_id: TenantId,
    vertical_id: Option<&str>,
) -> TruthBoundaryTurnCtx {
    genesis_truth_boundary_ctx_for_worker_in(&argo_vertical::default_dir(), tenant_id, vertical_id)
}

/// Root-parameterized core of [`genesis_truth_boundary_ctx_for_worker`], split
/// out so tests can exercise the synthesized-vs-default ROUTING decision against
/// a fixture verticals tree without mutating the process-global
/// `$ARGO_VERTICALS_DIR` (which would race other tests in the same binary).
/// Production callers go through the public wrapper, which passes
/// [`argo_vertical::default_dir()`].
///
/// The decision is strictly safety-preserving: a worker WITH a synthesized
/// per-vertical policy on disk gets THAT policy (the operator's configured
/// boundaries bite); a worker WITHOUT one falls back to
/// [`genesis_default_truth_boundary_ctx`] — never weaker than before this
/// routing existed.
#[must_use]
pub(in crate::turn) fn genesis_truth_boundary_ctx_for_worker_in(
    verticals_root: &Path,
    tenant_id: TenantId,
    vertical_id: Option<&str>,
) -> TruthBoundaryTurnCtx {
    match crate::turn_vertical_fence::load_synthesized_policy_in(
        verticals_root,
        tenant_id,
        vertical_id,
    ) {
        Some(policy) => {
            TruthBoundaryTurnCtx::new(policy, ChannelKind::Chat, RefusalTemplateRegistry::empty())
        }
        None => genesis_default_truth_boundary_ctx(tenant_id),
    }
}
