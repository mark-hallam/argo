# pricing-stripe-metered-overage-and-caps

**Owner**: claude or codex — **BILLING (DEC-034 risky surface) → conductor adversarial review before merge.**
**Depends on**: launch-wire-tier-quota-and-usage-metering MERGED (the meter is the data source).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-stripe-metered-overage-and-caps

## Problem (DEC-053 Option D, Pro tier)

Pro includes a usage allowance; beyond it usage must bill as **metered
overage at token-cost × 1.4** with a customer-visible self-set hard cap.
Today checkout is one flat subscription; no metered price, no overage
reporting, no customer-facing cap control.

## Fix

(1) `v1-tiers.toml [pro]`: add `included_actions`, `overage_multiplier =
1.4`, `default_monthly_cap_usd` — the single source of truth (DEC-035).
(2) A Stripe **metered price** alongside the flat $49 (operator creates it
in the dashboard; code reads `STRIPE_PRICE_PRO_OVERAGE`); a usage-report
job (cron/duty or post-action hook — reuse the tenant_usage + DEC-038
spend data, NO parallel counter) pushes overage units to Stripe usage
records, idempotent (record-id keyed). (3) Customer-visible cap: surface
the existing per-tenant max_usd quota on /billing (read + set,
owner-RBAC); SpendGate (sibling brief) enforces it — billing NEVER exceeds
the cap because usage stops at the cap (the DEC-053 invariant: stop, don't
surprise-bill). (4) Honest /billing meter: included vs used vs overage.

## Acceptance
1. Tests: usage ≤ allowance reports 0 overage; usage over reports
   (used-allowance) units exactly once (idempotency pin); cap reached →
   SpendGate refusal, no further overage units.
2. Webhook/tier flows unchanged (existing billing tests green); clippy +
   vitest + tsc clean. Stripe round-trip itself is operator-verified later
   on the test-card run (note it in the PR as UNVERIFIED-LIVE).

~150-250 LOC. **🔴 P1 — the Pro margin mechanics; public-flip blocker
(NOT a gated-cohort blocker).**
