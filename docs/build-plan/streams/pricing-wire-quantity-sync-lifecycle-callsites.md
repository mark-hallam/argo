# pricing-wire-quantity-sync-lifecycle-callsites

**Owner**: claude or codex — **P1 BILLING (DEC-053 — completes per-teammate seat billing).**
**Depends on**: #2868 (`pricing-per-teammate-quantity-sync` — the engine; MERGED when this is claimable).
**Authored against**: origin/main `a65d66298` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-wire-quantity-sync-lifecycle-callsites

## Problem

#2868 landed the seat-quantity engine (`argo_billing::quantity_sync::sync_seat_quantity`
— declarative, idempotent, no-op-skipping, prorated) plus the
initial-checkout count wiring. But the LIFECYCLE call sites are not wired:
a hire after checkout does not raise the billed quantity, a pause/freeze
does not lower it, and the invoice webhook does not reconcile drift. Until
these ~10-line call sites land, "$49 per named teammate, pause stops
billing" is only true at first checkout. (#2868's PR body explicitly
scoped these as the follow-up so the Stripe-in-the-hot-path failure
handling gets its own review — this brief.)

## Required change

Wire `sync_seat_quantity(client, store, tenant, subscription_id, reason)`
at:

1. **Hire** — `routes_workers_hire.rs` (after the worker row commits;
   reuse the same post-commit seam as `bridge_hire_duties`).
   `reason = Hire`.
2. **Freeze / unfreeze** — `routes_worker_freeze.rs` (both directions).
   `reason = Pause` / `Unpause`.
3. **Archive/delete** — the worker-archive route (find via
   `rg "archived_at" crates/argo-cli/src/handlers/gateway/`).
   `reason = Archive`.
4. **Invoice webhook reconciliation** — the Stripe webhook handler
   (`rg "invoice" crates/argo-cli/src/handlers/gateway/billing*.rs`); on
   `invoice.upcoming` or `invoice.created` for a subscription, re-assert.
   `reason = InvoiceReconcile`.

Failure handling (the load-bearing design question this brief owns):

- Lifecycle sites are **best-effort**: a Stripe outage must NOT fail the
  hire/pause/archive (the customer action wins); log + emit the
  `QuantityChange`-shaped audit row with the error, and rely on the
  webhook reconcile to heal. NEVER hold a customer mutation hostage to
  Stripe availability.
- The webhook reconcile is the **drift healer**: it re-asserts the
  absolute count, so any missed lifecycle sync converges by the next
  invoice boundary at the latest.
- Resolve `subscription_id` from the tenant's OWN stored billing mapping
  (the same source checkout/webhook already use) — never from request
  input; a caller must not be able to pair tenant A with subscription B.

Audit: write the returned `QuantityChange` as a tenant-scoped audit row
(grep the existing billing audit kinds first — extend, don't parallel).

## Verify

- Unit/integration tests per site: hire raises quantity, freeze lowers,
  unfreeze restores, archive lowers, webhook converges a drifted quantity;
  Stripe-failure on hire still commits the hire + writes the failure audit.
- `cargo test -p argo-cli -p argo-billing` green; crate clippy clean.

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`
- `crates/argo-cli/src/handlers/gateway/routes_worker_freeze.rs`
- the worker-archive route + the Stripe webhook handler in `gateway/billing*.rs`
- `crates/argo-billing/src/quantity_sync.rs` (only if a helper is missing)
