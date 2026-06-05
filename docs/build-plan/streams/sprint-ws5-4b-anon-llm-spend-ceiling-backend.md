# sprint-ws5-4b-anon-llm-spend-ceiling-backend

**Owner**: claude or codex (**Mac/Linux lane** — needs cargo compile + tests, which the
Windows Account-B infra lane cannot run; see memory `windows-cannot-build-argo-cli-cryptoki`).
**Depends on**: nothing hard; complements WS5-4 (`deploy/waf/` edge limits) + WS4-3 (tier gate).
**Authored against**: origin/main `07ff2ddfc` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws5-4b-anon-llm-spend-ceiling-backend`.

## Why

WS5-4 (`docs/skycot/deploy/abuse-and-cost-controls.md`) verified that the **edge + gateway
rate limiters cap request VOLUME** but there is **no hard global anon LLM-DOLLAR ceiling**.
The Genesis authoring path (`crates/argo-genesis/src/author.rs`) has zero
budget/spend/cost/quota references — it calls the operator's provider keys with no per-anon-IP
dollar cap. Voice/pitch are similar. A single determined anonymous IP (or a botnet under the
edge rate limit) can still run up real LLM spend on the operator's keys.

The per-(authenticated)-tenant budget subsystem (`turn_budget.rs`, `handlers/budget*.rs`)
does NOT cover pre-auth / anonymous traffic.

## What

A cumulative **anonymous LLM-spend guard**: track $ spent per anon key (IP and/or anon
tenant) over a rolling window, and when the ceiling is hit, **fail closed** on the LLM call
(or force the mock provider) with a clear 429/402-style response. Wire it into the anon LLM
entry points (Genesis classify/draft/author, voice, pitch).

Design sketch (implementer refines):
- A spend ledger keyed by anon identity (reuse the IP-resolution the gateway already does for
  `IpRateLimiter`; for anon tenants, the workspace-default tenant marker). Persist to a small
  table (additive migration) or an in-memory rolling counter with a TTL — match the existing
  `tenant_usage` / budget patterns rather than inventing a parallel store.
- A config block `[gateway.anon_spend]` (mirror `sections_rate_limit.rs`): `enabled`,
  `ceiling_usd`, `window_secs`, `on_exceed = "force_mock" | "reject"`.
- A check at the anon LLM dispatch boundary that consults the ledger before the provider call
  and records the post-call cost (reuse the existing cost accounting that feeds `costs*.rs`).
- Free tier already SHOULD be on mock (see WS4-3); this guard is the backstop for the
  operator-key paths (Genesis/voice/pitch) that run regardless of tier.

## May-touch (implementer confirms at claim time)
- A new small module near `argo-agent-runner` / the gateway LLM dispatch (grep the Genesis +
  voice + pitch provider-call sites first; do NOT add a parallel limiter — extend the budget
  or rate-limit surface).
- `argo-config` (the new `[gateway.anon_spend]` section, sibling to `sections_rate_limit.rs`).
- An additive migration IF a persisted ledger is chosen (next free slot; verify at claim).
- Tests (unit + an integration test asserting the ceiling fails closed / forces mock).

## Acceptance criteria
- [ ] Anon LLM calls past the configured `ceiling_usd` within `window_secs` are rejected OR
      forced to mock (per `on_exceed`), with a stable JSON error envelope.
- [ ] Genesis + voice + pitch anon paths consult the guard before the provider call.
- [ ] No parallel spend store — reuses/extends the existing budget/cost accounting.
- [ ] `[gateway.anon_spend]` config with sane defaults (`enabled=true`, a conservative
      `ceiling_usd`, `window_secs`, `on_exceed="force_mock"`).
- [ ] Unit + integration tests GREEN (report the pass tail); `cargo clippy` clean.
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`,
      additive migration only.

## Out of scope
- The edge WAF (shipped in WS5-4 `deploy/waf/`).
- The tier→capability hard gate / Free real-key block (WS4-3).

## Parallel-surface check
- `rg 'anon_spend|AnonSpend|spend_ceiling' crates/` — none today; new surface.
- `rg 'IpRateLimiter|turn_budget|tenant_usage' crates/` — REUSE these; extend, don't fork.

## Source
WS5-4 (`sprint-ws5-4-global-anon-abuse-and-llm-cost-cap`) +
`docs/skycot/deploy/abuse-and-cost-controls.md` §6. Audit
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-4, flagged unverified).
