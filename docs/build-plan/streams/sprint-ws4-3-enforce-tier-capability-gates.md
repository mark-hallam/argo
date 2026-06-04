# sprint-ws4-3-enforce-tier-capability-gates

**Owner**: claude or codex (**Mac/Linux lane** — needs cargo compile + tests; the Windows
Account-B infra lane cannot compile argo-cli, see memory `windows-cannot-build-argo-cli-cryptoki`).
**Depends on**: nothing hard. Closes the "Free→mock is SOFT" finding from WS5-4.
**Authored against**: origin/main `d699a56c8` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws4-3-enforce-tier-capability-gates`.

## Why

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS4-3, **FLAGGED unverified**): the
tier config (`docs/skycot/pricing/v1-tiers.toml`) defines feature flags (`provider_real_keys`,
`provider_mock`, `sso`, `channel_*`, …) and quotas (`max_workers`, `max_actions_per_month`),
**but it is not verified that these actually gate the runtime.**

WS5-4 verification confirmed the gap (`docs/skycot/deploy/abuse-and-cost-controls.md` §5):

> `rg provider_real_keys crates/` returns **only** `v1-tiers.toml` + tests — the flag is
> **never checked on the dispatch path.** "Free stays on mock" rests on the per-request
> default being `"mock"` (`routes_verticals_run.rs:56,60`) + the absence of a saved real key,
> NOT on a hard tier gate. A Free user who saves a real provider key and passes
> `provider: "anthropic"` is not blocked by tier logic today.

Same likely holds for `max_workers` (hire-time enforcement unverified) and the paid
`channel_*` flags. This brief makes the tier flags **load-bearing**.

## What

Wire the tier feature flags + quotas into the runtime so the Free tier is actually bounded:

1. **`provider_real_keys` → block real provider-key save/use on Free.** At the provider-key
   save path (`routes_onboarding.rs` provider-key / `routes_settings_credentials*.rs`) and/or
   the provider-selection path, consult the tenant's tier; if the tier lacks
   `provider_real_keys`, reject saving a real key (or force `provider="mock"` at dispatch).
   This is the load-bearing one (closes the WS5-4 finding).
2. **`max_workers` → block hire past the tier cap.** At first-hire / worker-create
   (`routes_onboarding_genesis.rs` / the hire path), count the tenant's active workers and
   reject past `max_workers` (0 = unlimited).
3. **`channel_*` → gate paid channels.** Where channels are enabled per tenant, check the
   tier flag.

Reuse the existing tier loader (`argo_identity::tier::TierConfig`) + the tenant→tier lookup
(`workspace.tier` / `usage_endpoint.rs` reads it already). Add ONE small capability-check
helper (e.g. `tier_allows(tenant, feature) -> bool`) and call it at the gates — do NOT scatter
ad-hoc tier string comparisons.

## May-touch (implementer confirms at claim time)
- A new small helper near `argo-identity` / the gateway (the `tier_allows` check).
- The provider-key save + provider-selection paths.
- The hire / worker-create path (`max_workers`).
- Tests (unit per gate + an integration test: a Free tenant is blocked from a real key / a
  2nd worker; a Pro tenant is allowed).
- Possibly the `GET /api/v1/tenant/usage` surface to report tier capabilities.

## Acceptance criteria
- [ ] A Free-tier tenant cannot save/use a real provider key (rejected OR forced to mock) —
      integration test proves it; mirrors the WS5-4 finding being closed.
- [ ] A Free-tier tenant cannot hire past `max_workers`; a Pro tenant can up to its cap.
- [ ] One canonical `tier_allows`-style helper; no scattered tier string compares (grep clean).
- [ ] `rg provider_real_keys crates/` now shows a **runtime** reader, not just the toml + tests.
- [ ] Unit + integration tests GREEN (paste the pass tail); `cargo clippy` clean.
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- The mock provider default itself (already in place).
- Billing checkout / Stripe (WS4-1 / WS4-2).
- The anon LLM-dollar ceiling (`sprint-ws5-4b` — different axis: anon vs tier).

## Parallel-surface check
- `rg 'fn tier_allows|tier_allows|has_feature' crates/` — confirm no existing helper before
  adding; extend if one exists.
- `rg 'TierConfig|workspace.tier' crates/` — REUSE the loader + the tenant→tier lookup.

## Source
WS5-4 verification (`docs/skycot/deploy/abuse-and-cost-controls.md` §5). Audit
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS4-3, flagged unverified).
