# pricing-wire-overage-report-and-cap-enforcement

**Owner**: claude or codex — **P1 BILLING → conductor adversarial review before merge.**
**Depends on**: #2855 (launch-wire-spendgate-llm-cost-ceiling, NEEDS-FIX) for part (b)'s real-enforcement option.
**Authored against**: origin/main `c4285661a` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-wire-overage-report-and-cap-enforcement

## Problem (adversarial review of #2859, MERGE-WITH-FOLLOWUP)

#2859 shipped the DEC-053 Pro overage **library + meter + cap-storage**
surfaces (all tenant-safe, RBAC owner-gated, money-math correct, idempotent).
Two non-blocking over-claims remain — the surface is read-honest and
write-safe so they did not block merge, but they must close before overage
billing is called LIVE:

1. **The Stripe-metered-overage push path is entirely unwired.**
   `argo_billing::overage::report_overage_usage` has **zero callers
   repo-wide** — no scheduled job, tick, CLI, or route invokes it. The
   function is correct and idempotent (stable `Idempotency-Key` on
   `(tenant_id, period_start)` + cumulative `action=set`), it just never
   runs. Today the system bills **nothing** in overage.

2. **The hard cap's "stops usage" claim is theater relative to the actual
   wire.** Doc comments (`crates/argo-billing/src/overage.rs:30-34`,
   `crates/argo-identity/src/tier.rs` `default_monthly_cap_usd` doc, the
   `v1-tiers.toml` comment) assert "SpendGate stops usage at the cap." But
   `SpendGate` has no production construction (only tests/doctest) and reads
   a provider-level budget, not the `tenant_quota.max_usd_micros` this PR
   writes. The ONLY live reader of `max_usd_micros` is
   `ensure_tenant_quota_allows` (`worker_actions_auto_approval.rs:311-336`),
   which runs **only in the AutoApprove branch** (`:269`) against
   `estimated_cost_micros(decl)` that is **`0` when the executor declares no
   budget** (`:338-341`). Human-approved + `requires_approval==false`
   actions bypass the cap entirely. So the cap is partial, not the
   "SpendGate stops usage" mechanism the comments name (DEC-029 NO-THEATER).

## Fix

**(a) Wire the overage-report job.** Add a periodic per-tenant job (duty /
cron / tick) that reads `STRIPE_PRICE_PRO_OVERAGE` + the live `tenant_usage`
action meter and calls `report_overage_usage` for each Pro tenant with
period-to-date units. The function exists and is idempotent — this is pure
wiring. Re-runs must stay no-ops (verify the key is stable across a period;
see the belt-and-suspenders note below).

**(b) Make the hard cap actually halt.** Either extend cap enforcement
(`ensure_tenant_quota_allows`) beyond the AutoApprove-only branch and close
the `estimated_cost_micros == 0` hole, OR wire the real
`SpendGate`/`BudgetEnforcer` against `tenant_quota.max_usd_micros`. This part
depends on #2855 landing (single choke-point in `build_transport`); prefer
that path so cap + LLM-spend ceiling share one enforcement seam.

**(c) De-theater the doc comments NOW (cheap, do first).** Soften
`overage.rs:30-34` + `tier.rs` + the TOML comment so they describe what is
wired ("the cap is stored and surfaced; enforcement lands with (b)/#2855")
rather than asserting "SpendGate stops usage" — which is not yet true.

## Belt-and-suspenders note (from the reviewer, for part (a))

The idempotency key is keyed on `period_start` only, so if a tenant's
`tenant_usage` row resets mid-period (re-provision), the cumulative `set`
would re-assert the new (lower) total. Confirm that's intended before going
live — a mid-period reset should not let a tenant escape accrued overage.

## Acceptance
1. `report_overage_usage` has at least one production caller; a test proves
   a Pro tenant over allowance reports `(used − allowance)` units exactly,
   once, idempotently.
2. A tenant at its cap is actually STOPPED across the human-approved and
   `requires_approval==false` paths (not just AutoApprove with non-zero
   estimate) — a test proves usage halts at the cap.
3. No code comment asserts enforcement that isn't wired (DEC-029).
4. Existing argo-billing / argo-identity tests green; clippy clean.

~150-250 LOC. **🔴 P1 — overage billing is not LIVE until this lands.**
