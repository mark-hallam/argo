//! Idempotency glue for `routes_onboarding::first_hire_response`.
//!
//! Defense-in-depth so two browser tabs (or a raw double-POST of the
//! CONFIRM call) cannot mint two teammates from one logical first hire.
//! Mirrors the `POST /api/v1/workers` hire-commit mechanism in
//! `routes_workers_hire` and — crucially — REUSES that module's shared,
//! process-global `(tenant, idempotency_key) → worker_id` cache rather
//! than standing up a parallel store. The workers-hire helpers were made
//! `pub(super)` for exactly this; there is one idempotency cache for both
//! worker-creation paths, so a key burned on `/hire` and the same key on
//! `/onboarding/first-hire` cannot diverge.
//!
//! ## Where it hooks (the needs_review → confirm shape MATTERS)
//!
//! `first_hire_response` has two POST shapes against the SAME url:
//!
//! - The FIRST POST carries no `confirmed_contract_digest`: Genesis runs,
//!   the gate holds the hire in `needs_review`, and the handler returns
//!   **422** — NO worker is created. We deliberately do NOT touch the
//!   idempotency cache on that path: there is nothing to de-dupe, and
//!   recording a 422 would wrongly short-circuit the later confirm.
//! - The CONFIRM POST carries the digest (and the SAME `idempotency_key`):
//!   it clears the gate and calls `create_worker_from_draft`, returning
//!   **201**. Idempotency wraps THAT worker creation — so a double-fired
//!   confirm collapses to the one row the first confirm minted.
//!
//! Lives in its own sibling so `routes_onboarding.rs` stays under the
//! 500-LOC per-file budget (it was exactly at the ceiling) — same split
//! pattern as `routes_onboarding_genesis.rs` / `routes_onboarding_helpers.rs`.

use super::routes_workers_hire::{
    body_idempotency_key, check_idempotency, conflict_response, digest_body, remember_idempotency,
    IdempotencyHit,
};
use argo_identity::WorkerId;
use argo_types::TenantId;
use serde_json::{json, Value};

/// A parsed first-hire idempotency intent: the de-dupe token (`None` when
/// the client omitted it — storage unavailable — so the hire proceeds
/// without client-side de-dupe, exactly as `/hire` does) plus the digest
/// of the meaningful request.
pub(super) struct IdemTicket {
    key: Option<String>,
    digest: String,
}

impl IdemTicket {
    /// Build the ticket from the parsed confirm body. The token is read
    /// via the SAME field/helper as `/hire` (`idempotency_key`). The
    /// digest covers the *meaningful* request — name + description +
    /// vertical_id + the confirmed contract digest — matching the spirit
    /// of what `/hire` digests (it hashes the whole body; this seam hashes
    /// the canonical fields so cosmetic body re-ordering between the two
    /// confirm POSTs of one logical hire still de-dupes, while a genuinely
    /// different hire under the same key correctly 409s). Hashed through
    /// the IDENTICAL `digest_body` hasher so both endpoints' entries are
    /// comparable in the shared cache.
    pub(super) fn open(
        parsed: &Value,
        name: &str,
        description: &str,
        vertical_id: &str,
        confirmed_digest: Option<&str>,
    ) -> Self {
        // A JSON object with stable key order gives a deterministic,
        // unambiguous digest input (no delimiter-collision across fields).
        let canonical = json!({
            "name": name,
            "description": description,
            "vertical_id": vertical_id,
            "confirmed_contract_digest": confirmed_digest.unwrap_or(""),
        })
        .to_string();
        Self {
            key: body_idempotency_key(parsed),
            digest: digest_body(&canonical),
        }
    }

    /// Probe the SHARED `(tenant, key)` cache BEFORE creating the worker.
    /// Returns `Some(response)` to short-circuit the handler — either a
    /// `201` replay of the already-committed worker (same key + same body)
    /// or a `409 idempotency_conflict` (same key + different body) — or
    /// `None` to proceed with `create_worker_from_draft`.
    pub(super) fn guard(
        &self,
        tenant: TenantId,
        name: &str,
        vertical_id: &str,
    ) -> Option<(&'static str, String)> {
        let key = self.key.as_deref()?;
        match check_idempotency(tenant, key, &self.digest) {
            IdempotencyHit::Match(existing) => Some(replay_response(existing, name, vertical_id)),
            IdempotencyHit::Conflict => Some(conflict_response(key)),
            IdempotencyHit::None => None,
        }
    }

    /// Record a freshly-committed first-hire worker in the SHARED cache,
    /// so a double-fired confirm replays it instead of minting a second
    /// teammate. No-op when the client omitted the key.
    pub(super) fn record(&self, tenant: TenantId, worker_id: WorkerId) {
        if let Some(key) = self.key.as_deref() {
            remember_idempotency(tenant, key, &self.digest, worker_id);
        }
    }
}

/// The `201` body returned on an idempotent replay of the first-hire
/// confirm. Mirrors the success body's worker-identifying fields and adds
/// `duplicate_of` + `idempotent` (matching the `/hire` `ok_idempotent`
/// disambiguation), so the SPA / audit view can tell a replay apart from a
/// fresh hire. The worker name + vertical echo the original request.
fn replay_response(
    worker_id: WorkerId,
    worker_name: &str,
    vertical_id: &str,
) -> (&'static str, String) {
    let id_str = worker_id.to_string();
    (
        "201 Created",
        json!({
            "worker_id": id_str,
            "duplicate_of": id_str,
            "name": worker_name,
            "vertical_id": vertical_id,
            "genesis_invoked": true,
            "genesis_fallback": false,
            "idempotent": true,
            "message": "Already hired (idempotent)",
        })
        .to_string(),
    )
}
