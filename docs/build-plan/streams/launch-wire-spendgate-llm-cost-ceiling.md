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

## Fix — SINGLE CHOKE POINT (revised 2026-06-12 after #2855 BLOCK)

⚠️ **#2855 attempt #1 BLOCKED (adversarial review):** wiring the gate only
at the turn-loop seam left SIX other live LLM paths UNWRAPPED — incl. the
internet-facing `/api/v1/pitch/ask` (`routes_pitch_ask_llm.rs:303`), Genesis
(`routes_genesis_stream_llm.rs:279`), ghostwriter, orchestration-team, and
the autonomous distillation loop (`provider_distillation_caller.rs:123`).

**Mandated approach: ONE choke point.** Wrap `SpendGate` INSIDE
`build_transport` (or the transport it returns) so EVERY
`build_transport(...).complete(...)` site — turn loop, pitch, genesis,
ghostwriter, orchestration-team, voice, distillation, forge, eval/judge —
is gated by construction. No path can forget it. Thread the requesting
tenant/session; pre-auth/anonymous calls MUST map to a real **capped
anonymous tenant** (closes ws5-4b). Avoid double-wrap.

Keep from attempt #1 (SOUND): production-default non-zero ceilings
($1/$10/$100 session/day/month), fail-CLOSED on store-read error,
structured `BudgetExceeded` + `budget.ceiling_blocked` audit, per-tenant
isolation. **Recommended:** pass a real per-request cost estimate (not the
flat 4¢ projection) so the call that CROSSES the ceiling is itself blocked.

## Acceptance
1. **Coverage:** every `build_transport(...).complete` path gated — a test
   or checked-in grep proving pitch + genesis + distillation refuse at
   ceiling (FAILs on #2855 attempt #1 — those were unwrapped).
2. A tenant at its ceiling gets the structured refusal + audit row; spend
   does NOT increment past the cap.
3. Anonymous/pre-auth path maps to a capped anonymous tenant + is gated.
4. Default ceilings active in production; dev/demo unchanged. Existing
   spend-attribution tests green; clippy clean.

~120-200 LOC. **🔴 P1 — the open-wallet hole; hard launch blocker.**
