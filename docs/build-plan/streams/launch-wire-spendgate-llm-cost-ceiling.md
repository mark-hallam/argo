# launch-wire-spendgate-llm-cost-ceiling

**Owner**: claude or codex — **COST-SAFETY P1 → conductor adversarial review before merge.**
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-wire-spendgate-llm-cost-ceiling

## Problem (launch-readiness audit 2026-06-12, OPERATOR-LOCKED launch blocker)

`SpendGate` (the hard `[budget.ceiling]` LLM-spend enforcement wrapper) is
built and unit-tested but has **ZERO construction sites outside its own
crate** — `build_transport` never wraps it, so nothing stops a stranger (or
a runaway loop) burning unbounded LLM dollars on day 1. Rate limits bound
requests, not cost. The abuse-controls doc marks the anonymous-spend
ceiling layer "MISSING (5-4b)" with no brief on file. Operator constraint:
"we cannot lose money on this."

## Fix

Wire `SpendGate` into the live transport path (`build_transport` or the
turn loop seam — grep for where per-teammate spend attribution [DEC-038]
already hooks, REUSE that seam) with: (1) a sane default per-tenant monthly
ceiling (config `[budget.ceiling]`, default ON in production profile, e.g.
$20/tenant/month until billing overrides); (2) a global anonymous/free-tier
ceiling (closes ws5-4b); (3) ceiling-hit → honest 402/429-class structured
error + audit row, NEVER a silent failure; (4) operator override via the
existing `argo tenant quota` path.

## Acceptance
1. Test: a tenant at its ceiling gets the structured refusal + audit row;
   spend does NOT increment past the cap (would FAIL today — gate unwired).
2. Default ceilings active in production profile; dev/demo unchanged unless
   configured. 3. Existing spend-attribution tests green; clippy clean.

~120-200 LOC. **🔴 P1 — the open-wallet hole; hard launch blocker.**
