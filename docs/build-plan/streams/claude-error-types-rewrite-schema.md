# claude-error-types-rewrite-schema — rewrite ~84 Schema-variant callsites per ERROR_VARIANT_AUDIT.md

**Owner**: claude. **Depends on**: `claude-error-types-consolidation` MERGED.
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-types-rewrite-schema`.

> SPLIT 2 of 4 from `claude-error-types-rewrite-callsites`. Schema-variant rewrites (~84 sites).

## Scope

Mechanically rewrite every `ArgoError::Schema(...)` callsite flagged in `ERROR_VARIANT_AUDIT.md`. Most preserve the `format!()` string (Schema accepts `String`); a minority should switch to `Tool { tool, msg }` or `RoutingFailed { ... }` per the audit row's recommendation.

**Not in scope**: Io rewrites (sibling sub-brief); Invariant rewrites (sibling sub-brief; needs strategy); CI flip (final sub-brief).

## Files you may touch

- Any `crates/argo-*/src/**/*.rs` file whose row in `ERROR_VARIANT_AUDIT.md` has `current = Schema`. Variant-swap only.

## Files you must NOT touch

- `crates/argo-types/src/error.rs`.
- Any row whose `current != Schema`.
- `.github/workflows/ci.yml`.

## Acceptance criteria

- [ ] Every Schema row in audit rewritten.
- [ ] `bash scripts/check-error-variant-usage.sh` reports 0 Schema mismatches.
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean.
- [ ] No public API change.
- [ ] Every modified file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-error-types-rewrite-schema`
- PR title: `error: rewrite Schema-variant callsites per audit (claude-error-types-rewrite-schema)`

## Notes for the implementer

- ~84 sites. Spans gateway, store, runtime, audit. Heavier than Io in concentration per file.
- Schema-to-Tool conversions need to read the `tool` name from context (the rationale column in the audit guides this). If unclear, leave the row as Schema and note in PR body for a follow-up.
