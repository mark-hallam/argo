# ops-schema-lint-distinguish-leads-vs-lags-error-message — fix the schema-lint error text direction

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-schema-lint-distinguish-leads-vs-lags-error-message`.

## Why this is P4 (cosmetic, evaluator round-72 follow-up)

#2334 added the EXPECTED_SCHEMA_VERSION-vs-tip lint to `scripts/conductor-pre-merge-gate.sh`. Evaluator round-72 verified the lint fires correctly in 4 scenarios (baseline / lag / post-bump / reverse-divergence). The cosmetic finding: the error message always says `"lags behind"` even when EXPECTED > tip (reverse divergence — someone bumped EXPECTED downward, or someone added a migration but forgot to bump). The message direction should branch.

## Scope

~3-10 LOC bash change in `scripts/conductor-pre-merge-gate.sh` (or whichever file the lint logic lives in). When `EXPECTED < tip`, emit "EXPECTED_SCHEMA_VERSION lags behind migration tip (v$EXPECTED vs v$TIP)". When `EXPECTED > tip`, emit "EXPECTED_SCHEMA_VERSION leads migration tip (v$EXPECTED vs v$TIP) — did you bump EXPECTED without adding the migration?".

## Files you may touch

- `scripts/conductor-pre-merge-gate.sh` — the lint message.
- Possibly `scripts/check-expected-schema-version.sh` if that's where the logic lives — grep first.

## Files you must NOT touch

- Anything under `crates/`.

## Acceptance criteria

- [ ] Run lint with EXPECTED < tip → message says "lags behind".
- [ ] Run lint with EXPECTED > tip → message says "leads tip".
- [ ] Baseline (EXPECTED == tip) → exit 0, no message.
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` passes.

## Branch + PR

- Branch: `claude-ops-schema-lint-distinguish-leads-vs-lags-error-message` or `codex-…`.
- PR title: `ops(schema-lint): branch error message on lag vs lead direction (ops-schema-lint-distinguish-leads-vs-lags-error-message)`.

## Notes

- Fast-mode candidate. ~5 min wall.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
