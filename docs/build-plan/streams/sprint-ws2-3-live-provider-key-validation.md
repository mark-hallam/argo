# sprint-ws2-3-live-provider-key-validation

**Owner**: claude or codex. **Backend — so "real work works" for a fresh user.**
**Depends on**: none.
**Authored against**: origin/main `5c6a82366` on `2026-06-04` (verified per gap-audit WS2: `validate_key_shape` in `routes_onboarding_helpers.rs` is shape-only, no live check).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws2-3-live-provider-key-validation`.

## Why

`POST /api/v1/onboarding/provider-key` currently validates a saved provider key with
`validate_key_shape` (prefix/length only — `routes_onboarding_helpers.rs:~141`). So a fresh
user can save a typo'd/revoked key, get a green checkmark, then have their first real
(non-mock) worker run silently fail. For a paying-ready product, the key should be
validated with a REAL lightweight provider call at save time so the user knows immediately.

## What

Augment the provider-key save path with a real one-token validation BEFORE (or alongside)
storing:
- On save, make a cheap real call against the provider (e.g. a models-list / a 1-token
  completion) using the submitted key, via the existing `argo-provider` abstraction (reuse
  it — do NOT hand-roll HTTP per provider). 
- Valid → store + return success. Invalid/unauthorized → reject with a clear, honest error
  (NO THEATER — never a green success on a bad key).
- Keep the shape pre-check as a fast first gate (skip the network call on obviously-malformed
  input). Bound the validation call (timeout) so a slow provider doesn't hang the save.
- Preserve the per-tenant master-key encryption on store (unchanged).

Re-verify the save handler + the provider abstraction at claim time
(`rg 'validate_key_shape|provider.key|wrap_secret' crates/`). Gate the live call behind the
real provider only (don't network-call in mock/test mode).

## May-touch
- `routes_onboarding_helpers.rs` (the provider-key save path) + a small validation helper
- the provider abstraction call (reuse `argo-provider`)
- `crates/argo-cli/tests/` — test: invalid key → rejected; valid (mock provider) → stored
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

## Acceptance criteria
- [ ] Saving an invalid/unauthorized key returns a clear error + does NOT store it.
- [ ] A valid key stores + returns success (encryption preserved).
- [ ] The live call is bounded (timeout) + skipped in mock/test mode (no flaky network in CI).
- [ ] `cargo clippy -p argo-cli` clean; touched + new tests GREEN.
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- The fresh-tenant baseline seed (WS2-1, shipped) + the empty states (WS2-2, shipped).

## Parallel-surface check
- `rg 'validate_key|provider.*validate' crates/` — extend the canonical save path; no parallel validator.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS2-3).
