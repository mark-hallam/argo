# pricing-per-teammate-quantity-sync

**Owner**: claude or codex — **BILLING → conductor adversarial review before merge.**
**Depends on**: none (independent of overage; composes with it).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-per-teammate-quantity-sync

## Problem (DEC-053: "$49 per NAMED teammate, pause stops billing")

Checkout hardcodes `line_items quantity=1` (`checkout.rs:54`) — one flat
subscription per tenant, fully decoupled from teammate count. The audit
graded this the billing HARD-BLOCKER: the public pricing promise has no
code.

## Fix

Sync the Pro subscription item quantity to the tenant's count of ACTIVE
(non-paused, non-deleted) teammates: on hire/unpause → quantity+1, on
pause/delete → quantity-1 (Stripe subscription-item update with
`proration_behavior` per Stripe defaults; document the proration choice in
the PR). Initial checkout uses the current active count (min 1). Reuse the
existing worker lifecycle seams (freeze/pause + hire chokepoint — grep
`Freeze worker` + `create_worker_from_draft`); webhook reconciliation:
on invoice events, re-assert quantity == active count (drift heals).
Free-tier tenants unaffected. Audit row per quantity change (who/why).

## Acceptance
1. Tests: hire → quantity increments (mock Stripe client); pause →
   decrements; never below 1 while subscribed; reconciliation heals an
   injected drift; every change audited.
2. Existing billing/webhook tests green; clippy clean.

~120-200 LOC. **🔴 P1 — makes the headline pricing promise true;
public-flip blocker.**
