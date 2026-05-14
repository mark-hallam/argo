# M9-eval-regression-detection - Track M (M-Eval), Phase 2, Tier 1

**Owner**: claude. **Depends on**: nothing (uses existing argo-eval).
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M9-eval-regression-detection`.

## Scope

Compare eval results between two skill versions on the same fixture set. Statistical regression detection: per-fixture pass/fail + aggregate score + significance test. AB-promote gate calls this; refuses promotion on detected regression unless override flag set.

LOC: ~900. Branch: `claude-M9-eval-regression-detection`. PR title: `feat(eval): cross-version regression detection (M9)`.

## Files you may touch

- NEW crates/argo-eval/src/regression.rs
- NEW crates/argo-eval/src/regression_tests.rs
- crates/argo-skill/src/ab_test.rs - TOUCH (wire regression detector into AB-promote gate; <=30 modified lines)

## Files you must NOT touch

- Existing argo-eval runner/streaming machinery (consume APIs).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Two fixture sets, deliberate regression detected.
- [ ] No false positives on noise-level variation.
- [ ] AB-promote gate refuses regression unless override.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M9-eval-regression-detection`
- PR title: `feat(eval): cross-version regression detection (M9)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 4 (Skills are learned - safe-learning loop).


## Amendment 2026-05-14 (may-touch widening)

Worker self-BLOCKED on initial claim noting that may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
