# mission-claim-3-runtime-wiring-v1x — wire argo-routing::TieredRouter into agent loop (v1.x post-launch)

**Owner**: claude. **Sizing hint**: cross-crate integration + E2E fallback test.
**Depends on**: existing `argo-routing` crate + 9 provider manifests (all already shipped).
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-3-runtime-wiring-v1x`.
**Source**: 2026-05-18 audit P1 finding #1 — `argo-routing` crate exists but is NOT WIRED into the agent loop. Only consumer is the `argo route inspect|bench` CLI. Adaptive/cost-tier routing is paper-only.

**Operator decision (2026-05-18)**: "ship it" — accept claim 3 PAPER → PROVEN as v1.x scope. Public launch will not claim "automatic provider fallback"; this brief lands the runtime wiring as a v1.1 follow-up.

## Mission alignment

- **🎯 Mission claim 3** — "TOML provider manifests with cost-tier routing + fallback." The launch pitch currently must avoid claiming automatic fallback; this brief unblocks that pitch.
- **Reliability** (CLAUDE.md priority #1) — provider outage during a customer session = stuck bot. Fallback is the fix.
- **Commercial usability** (#5) — multi-LLM market; cost-tier control is a sales lever.

## Scope

P1 v1.x. Wire `argo-routing::TieredRouter` into `argo-agent-runner::dispatch` as the provider-selection chokepoint. Land an E2E cross-provider fallback test.

### What lands here

1. **Route selection in dispatch**:
   - `crates/argo-agent-runner/src/dispatch.rs` currently picks providers from a fixed chain.
   - Replace with: `let provider = TieredRouter::from_manifests(&manifests).select(&request_ctx)?;`
   - The router consults manifest tier metadata (cost-per-token + latency-budget + capability-match) + a runtime health-check (recent error rate).
2. **Fallback-on-error**:
   - `TieredRouter` has a `fallback_to_default_on_error` flag (already implemented at `crates/argo-routing/src/tiered.rs:107`).
   - Wire dispatch's error-path: on provider error → re-select with the failed provider excluded → retry.
   - Cap fallback attempts at 3 to avoid cascading failures.
3. **Observability**:
   - Emit `ProviderRouted { tier, provider, fallback_count }` audit rows so customers can verify what's being routed where.
4. **E2E test**:
   - `crates/argo-cli/tests/provider_fallback_smoke.rs` (NEW): drive a session through dispatch with a stub-fail provider in tier-1 → assert fallback to tier-2 provider → assert success.
5. **Cost-tier acceptance test**:
   - Configure 2 stub providers in tier-1 + tier-3 with different cost-per-token. Drive a "high-priority" session → assert tier-1 selected. Drive a "batch" session → assert tier-3 selected.

LOC: ~600.

## Files you may touch

- `crates/argo-agent-runner/Cargo.toml` — add `argo-routing` workspace dep. ≤4 modified LOC.
- `crates/argo-agent-runner/src/dispatch.rs` — wire `TieredRouter` at the provider-selection chokepoint. ≤120 modified LOC.
- `crates/argo-agent-runner/src/dispatch_fallback.rs` (NEW or extend) — fallback-on-error loop. ≤150 LOC.
- `crates/argo-audit/src/event.rs` — add `ProviderRouted` variant. ≤20 modified LOC.
- `crates/argo-cli/tests/provider_fallback_smoke.rs` (NEW). ≤180 LOC.
- `crates/argo-cli/tests/provider_cost_tier_smoke.rs` (NEW). ≤120 LOC.
- `docs/skycot/operator/provider-routing.md` (NEW or extend) — operator runbook for tuning tiers + interpreting `ProviderRouted` audit rows. ≤80 LOC.

## Files you must NOT touch

- `argo-routing` crate internals — pure consumer.
- Provider manifest TOMLs (`manifests/providers/*.toml`) — read-only consumer.
- `argo route inspect|bench` CLI — read-only adjacent.

## Acceptance criteria

- [ ] Agent loop dispatches via `TieredRouter`, not a fixed chain.
- [ ] On primary-provider error: fallback kicks in, retry succeeds, dispatch completes.
- [ ] `ProviderRouted` audit row emitted on every provider pick.
- [ ] `crates/argo-cli/tests/provider_fallback_smoke.rs` passes — assert tier-2 selected when tier-1 stub-fails.
- [ ] `crates/argo-cli/tests/provider_cost_tier_smoke.rs` passes — assert tier-1 selected for high-priority, tier-3 for batch.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Operator runbook documents tuning + observability.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-claim-3-runtime-wiring-v1x`
- Title: `feat(agent-runner): wire TieredRouter into dispatch + fallback-on-error (mission-claim-3-runtime-wiring-v1x) — claim-3 PAPER → PROVEN`
- Worker verification: workspace clippy + tests + manual smoke with 2 stub providers.

## Hidden constraints

- **No change to provider manifest TOML schema.** All routing intelligence is in the Rust side; manifests stay declarative.
- **Fallback cap at 3 attempts.** Prevents cascading retries from masking systemic outages.
- **Cost-tier acceptance test uses stub providers.** Don't burn real API tokens in CI.
- **`ProviderRouted` audit row must NOT contain user content.** Just routing metadata (tier + provider + fallback count). Customer data stays in `ToolResult` payloads.
- **Closes claim-3 PAPER status.** When this lands + tests pass + audit row visible, claim 3 is runtime-PROVEN.
