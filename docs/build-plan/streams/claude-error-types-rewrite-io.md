# claude-error-types-rewrite-io — rewrite ~338 Io-variant callsites per ERROR_VARIANT_AUDIT.md

**Owner**: claude. **Depends on**: `claude-error-types-consolidation` MERGED (audit report shipped).
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-types-rewrite-io`.

> SPLIT 1 of 4 from the DEEP-DRIFT `claude-error-types-rewrite-callsites` (438 sites, unreviewable as one PR). This sub-brief handles only the Io-variant rewrites — the largest cluster (~338 sites).

## Scope

Mechanically rewrite every `ArgoError::Io(...)` callsite flagged in `docs/skycot/ERROR_VARIANT_AUDIT.md` whose recommended variant is the auto-conversion path. Most of these become `?`-via-`From<std::io::Error> for ArgoError` instead of explicit `.map_err(|e| ArgoError::Io(e.to_string()))`. The audit report rows the implementer reads have `path:line + current + recommended + rationale` triples — apply verbatim.

**Not in scope**: Schema rewrites (separate sub-brief); Invariant rewrites (separate sub-brief; needs conductor strategy decision); flipping CI to fail (final sub-brief). Do NOT touch any non-Io row.

## Files you may touch

- Any `crates/argo-*/src/**/*.rs` file whose row in `ERROR_VARIANT_AUDIT.md` has `current = Io`. Variant-swap edits only; message strings byte-identical.

## Files you must NOT touch

- `crates/argo-types/src/error.rs` — contract is locked.
- `scripts/check-error-variant-usage.sh` — locked.
- `docs/skycot/ERROR_VARIANT_AUDIT.md` — read-only input.
- Any row whose `current != Io` — skipped, handled by sibling sub-briefs.
- `.github/workflows/ci.yml` — CI flip is the final sub-brief.

## Acceptance criteria

- [ ] Every Io row in `ERROR_VARIANT_AUDIT.md` rewritten to the recommended variant.
- [ ] `bash scripts/check-error-variant-usage.sh --variant=io` reports 0 mismatches.
- [ ] Non-Io rows still flagged (those are siblings' problem).
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] No public API change.
- [ ] Every modified file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-error-types-rewrite-io`
- PR title: `error: rewrite Io-variant callsites per audit (claude-error-types-rewrite-io)`

## Notes for the implementer

- Pure mechanical — read the audit row, swap the variant, message verbatim.
- If `scripts/check-error-variant-usage.sh` doesn't accept a `--variant=io` filter, run the unfiltered script and assert the Io-row count drops to 0 while other rows remain unchanged. Document the choice in PR body.
- Heavy file count — expect 100+ files touched. Each file change is 1-3 lines.
