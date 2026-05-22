# claude-orchestrator-persist-split — split persist.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main d564971d on 2026-05-09 (conductor-authored after observing persist.rs at 499/500 LOC).

## Why

`crates/argo-orchestrator/src/persist.rs` is at 499 LOC on current main — 1 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Audit / queue / handoff persist briefs touch this. Preemptive split prevents future ceiling-refusals.

Note: there's already a `persist_audit.rs` sibling (added by #958), so the project's accepted pattern for persist-extension is the `persist_<topic>.rs` sibling shape.

Pure refactor; mirrors proven pattern.

## Approach (concrete)

1. Read `persist.rs` end-to-end. Identify a cohesive section (likely per-table writers or helpers).
2. Move to `crates/argo-orchestrator/src/persist_<topic>.rs` (mirroring the existing `persist_audit.rs` convention).
3. Re-export via `pub(crate) use`.

Target: `persist.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-orchestrator/src/persist.rs` — narrowly: extract sibling, re-export
- `crates/argo-orchestrator/src/persist_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- `crates/argo-orchestrator/src/persist_audit.rs` — separate sibling, already exists
- `crates/argo-orchestrator/src/walker.rs` / `handoff.rs` — separate concerns
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `persist.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-orchestrator-persist-split`
- PR title: `orchestrator: split persist.rs below 500-LOC ceiling (claude-orchestrator-persist-split)`

## Unblocks

Future audit/queue/handoff persist extensions.
