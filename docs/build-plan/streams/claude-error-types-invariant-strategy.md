# claude-error-types-invariant-strategy — widen `ArgoError::Invariant` to accept `String` payloads

**Owner**: claude. **Depends on**: `claude-error-types-consolidation` MERGED.
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-types-invariant-strategy`.

> SPLIT 3 of 4 from `claude-error-types-rewrite-callsites`. The 16 Invariant-variant rewrite sites can't land mechanically because `ArgoError::Invariant(&'static str)` cannot accept the `format!()` strings the audit recommends. This sub-brief makes the architectural decision: **widen `Invariant` to `Invariant(String)`** (matches Provider/Schema/Io pattern; smallest churn; no information loss).

## Scope

1. **Change `ArgoError::Invariant`** in `crates/argo-types/src/error.rs` from `Invariant(&'static str)` to `Invariant(String)`. This is the only public-API change in the brief — other Provider/Schema/Io variants already use `String`; this aligns Invariant with the workspace pattern.

2. **Update the existing call sites** that pass `&'static str` literals — these continue to compile because `&str` coerces to `String` via `.to_string()` or `.into()`. Audit how many sites need an `.into()`/`.to_string()` suffix (likely ~20-30 sites; modify in this PR).

3. **Apply the 16 audit-flagged Invariant rewrites** that the audit recommends — now they can use `format!()` strings.

4. **Update the doc comment** on the variant to note the widening and the rationale.

## Files you may touch

- `crates/argo-types/src/error.rs` — narrowly scoped: widen `Invariant` variant + update doc comment. ≤10 modified lines.
- Any `crates/argo-*/src/**/*.rs` file with an existing `ArgoError::Invariant(...)` call site that needs an `.into()` suffix (likely ~20-30 sites — automated by `cargo build` showing type errors). Pure mechanical.
- The 16 audit-flagged sites get their format!() rewrites.

## Files you must NOT touch

- `scripts/check-error-variant-usage.sh`.
- The Io / Schema / other-variant callsites — siblings handle those.
- `.github/workflows/ci.yml` — final sub-brief.

## Acceptance criteria

- [ ] `ArgoError::Invariant` accepts `String`.
- [ ] All existing `Invariant("static literal")` call sites updated to compile (typically `.to_string()` or `.into()` suffix).
- [ ] The 16 audit-flagged sites rewritten with `format!()` payloads per the audit.
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean.
- [ ] `bash scripts/check-error-variant-usage.sh` Invariant mismatches drop to 0.
- [ ] Every modified file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-error-types-invariant-strategy`
- PR title: `error: widen Invariant to String + apply audit rewrites (claude-error-types-invariant-strategy)`

## Notes for the implementer

- `Invariant(&'static str)` was originally chosen for zero-allocation panics; widening to `String` regresses that micro-optimization. The information value (proper error context) is worth it; document the trade-off in the PR body.
- The compile-error sites surface automatically — let `cargo build` find them; no audit needed.
- Conductor (CEO authorized 2026-05-13) chose **widen** over the 2 alternatives (add a `Storage(String)` variant; accept static-only with information loss). Don't second-guess; ship the widening.
