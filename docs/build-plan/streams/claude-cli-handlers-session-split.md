# claude-cli-handlers-session-split — split session.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main e9ae619a on 2026-05-09 (conductor-authored after observing session.rs at 499/500 LOC).

## Why

`crates/argo-cli/src/handlers/session.rs` is at 499 LOC on current main — 1 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Any session-CLI extension (export, replay, fork, transcript) hits this ceiling immediately; a preemptive split prevents future ceiling-refusals.

Pure refactor; no behaviour change. Mirrors the proven split pattern from prior splits (#932, #956, #1087, #1088, #1097, #1102).

## Approach (concrete)

1. Read `handlers/session.rs` end-to-end. Identify a self-contained section.
2. Common candidates: per-subcommand handlers (if `argo session show/export/replay` are all in one file), formatting helpers, fork/clone logic.
3. Move that section to `crates/argo-cli/src/handlers/session_<topic>.rs`.
4. Re-export via `pub(crate) use`.

Target after split: `session.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/session.rs` — narrowly: extract sibling, re-export
- `crates/argo-cli/src/handlers/session_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line if needed

## Files you must NOT touch

- Other session-related handlers like `session_query.rs` if they exist as separate concerns
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `session.rs` ≤ 350 LOC after the split (≥ 150 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (no behaviour change)
- [ ] No public API changes

## Branch + PR

- Branch: `claude-cli-handlers-session-split`
- PR title: `cli: split handlers/session.rs below 500-LOC ceiling (claude-cli-handlers-session-split)`

## Notes for the implementer

- Pattern-match prior splits for the proven shape.
- Don't pick a random LOC cut — find a cohesive sub-concern.

## Unblocks

- Future `argo session <verb>` extensions that would otherwise hit the ceiling.
