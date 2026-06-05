# uf-genesis-author-default-on (UF Phase-C · C2)

**Owner**: claude.
**Depends on**: none (no real dep — set TODO immediately per operator 2026-06-05).
**Authored against**: origin/main `58a35a9f9` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-author-default-on`.

## Why
The rich LLM-author pass already exists but is flag-OFF (`GENESIS_LLM_AUTHOR_ENV`, mirroring `ARGO_GENESIS_LLM_CLASSIFIER`), fail-safe to the template. Default it **ON when a provider key is present** — the DEC-025 full-synthesis intent — so generated teammates get the richer author pass by default, still falling back to the template when there's no key or on provider failure.

## Grep-before-write (verified 2026-06-05)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_author.rs` (~:32) — the provider-touching half, flag-gated by `GENESIS_LLM_AUTHOR_ENV`. Flip the DEFAULT to on-when-key-present; keep the env var as an explicit override AND keep the fail-safe-to-template path intact.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_genesis_author.rs` (the default selection ~:32). Provider-key-present → author pass on by default; no key OR provider error → template (unchanged fail-safe).

## Acceptance
With a provider key set, a genesis author runs the LLM pass by default (no env needed); with no key OR on provider failure, it falls back to the template (no crash, no regression); the env flag still overrides.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (genesis author-default row) + regen HTML.

## Effort
~1 brief (S).
