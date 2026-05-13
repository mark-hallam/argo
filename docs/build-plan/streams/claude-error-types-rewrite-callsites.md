# claude-error-types-rewrite-callsites — apply ERROR_VARIANT_AUDIT.md mechanically

**Owner**: claude. **Depends on**: `claude-error-types-consolidation` MERGED (audit report + doc-comment + CI warning step). **Phase B consistency narrow.**

**Authored against**: origin/main b3949a73 on 2026-05-09 (split-out from the original combined `claude-error-types-consolidation` brief).

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: two structural issues. (1) **Scale 15× brief estimate** — `bash scripts/check-error-variant-usage.sh` reports 438 live mismatches across 232 files in 13 crates (Io 338, Schema 84, Invariant 16); the brief estimated 10-30 and itself flags "consider splitting into per-crate sub-PRs" if >50. A single 438-edit PR is unreviewable. (2) **Invariant rewrites uncompilable as mechanical swaps** — `ArgoError::Invariant(&'static str)` cannot accept the `format!()` strings used at the 16 flagged sites; brief explicitly forbids widening the variant. Acceptance criteria "0 mismatches" + "flip CI to fail" cannot both be reached without conductor input. **Recommended split**: (a) `claude-error-types-rewrite-io` (~338 sites, mostly `?`-via-`From<io::Error>`); (b) `claude-error-types-rewrite-schema` (~84 sites, `Schema(format!…)`); (c) `claude-error-types-invariant-strategy` — conductor decides: widen `Invariant` to `String`, add a `Storage(String)` variant, or accept static-only Invariant with information loss; (d) `claude-error-types-flip-ci` flips warning→fail once 0 mismatches reached.

## Why

The audit phase (`claude-error-types-consolidation`) produced `docs/skycot/ERROR_VARIANT_AUDIT.md` listing every mismatched `ArgoError::*` callsite in the workspace with a recommended variant + 1-line rationale. This brief mechanically applies those recommendations and flips the CI guard from warning to fail.

## Approach (concrete)

1. **Read `docs/skycot/ERROR_VARIANT_AUDIT.md`** — every row is `path:line + current variant + recommended variant + rationale`.
2. **For each row** — flip the variant. Keep the message string byte-identical (so log-grep doesn't break). Edits are single-line variant swaps.
3. **Re-run `scripts/check-error-variant-usage.sh`** — should report 0 mismatches.
4. **Flip CI step from warning-only to fail-on-mismatch** in `.github/workflows/ci.yml`.

## Files you may touch

- Any `crates/argo-*/src/**/*.rs` file appearing in the audit report — narrowly: variant-swap edits only, message strings unchanged.
- `.github/workflows/ci.yml` — narrowly: flip the audit step from warning to fail.

## Files you must NOT touch

- `crates/argo-error/src/lib.rs` itself — the contract is fixed by the audit brief.
- `scripts/check-error-variant-usage.sh` — already shipped by the audit brief.
- `docs/skycot/ERROR_VARIANT_AUDIT.md` — read-only input; only updated as a side-effect of `--regenerate` if needed.
- Any test file beyond `cargo test --workspace` validation runs.

## Acceptance criteria

- [ ] Every mismatched callsite from `ERROR_VARIANT_AUDIT.md` rewritten to the recommended variant
- [ ] `scripts/check-error-variant-usage.sh` reports 0 mismatches
- [ ] CI step flipped from warning to fail
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No public API change (variant names + signatures unchanged; only callsite variants flipped)
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-error-types-rewrite-callsites`
- PR title: `error: rewrite callsites per ERROR_VARIANT_AUDIT.md + flip CI to fail (claude-error-types-rewrite-callsites)`

## Notes for the implementer

- This brief is **mechanical only**. If a row in the audit report is ambiguous, leave it and add a comment to the report; do not invent semantics.
- Workspace-wide clippy + tests are in acceptance because rewrites span multiple crates; budget for the cold workspace build.
- If the audit report has > 50 rows, consider splitting into per-crate sub-PRs — but most pre-audit estimates suggest 10-30 mismatches total.
- Keep the PR diff a clean variant-swap: `ArgoError::Internal(...)` → `ArgoError::Provider(...)` with no other content changes per line.
