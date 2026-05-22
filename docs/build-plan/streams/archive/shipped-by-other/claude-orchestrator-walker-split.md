# claude-orchestrator-walker-split — split walker.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main e9ae619a on 2026-05-09 (conductor-authored after observing walker.rs at 499/500 LOC).

## Why

`crates/argo-orchestrator/src/walker.rs` is at 499 LOC on current main — 1 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Multiple handoff and policy briefs need to extend the walker (handoff-policy-store, handoff-stats-cli, sweep-rules); a preemptive split prevents future ceiling-refusals on those briefs.

Pure refactor; no behaviour change. Mirrors the proven split pattern from `claude-routes-post-split` (#932), `claude-routes-get-split` (#956), `claude-channels-push-voice-split` (#1087), `claude-doctor-channels-split` (#1088), `claude-agent-runner-turn-split` (#1097), `claude-tui-app-split` (#1102).

## Approach (concrete)

1. Read `walker.rs` end-to-end. Identify a self-contained section (typically helpers, sub-types, or a sub-concern that's mostly orthogonal to the file's main responsibility).
2. Move that section to `crates/argo-orchestrator/src/walker_<topic>.rs` (e.g., `walker_helpers.rs`, `walker_policy.rs`, depending on what's natural).
3. Re-export from `walker.rs` via `pub(crate) use` so call sites are unchanged.

Target after split: `walker.rs` ≤ 350 LOC (≥ 150 LOC headroom).

## Files you may touch

- `crates/argo-orchestrator/src/walker.rs` — narrowly: extract sibling, re-export
- `crates/argo-orchestrator/src/walker_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- `crates/argo-orchestrator/src/handoff.rs` — separate concern
- `crates/argo-orchestrator/src/persist.rs` — separate concern (own split-prep candidate)
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `walker.rs` ≤ 350 LOC after the split (≥ 150 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-orchestrator-walker-split`
- PR title: `orchestrator: split walker.rs below 500-LOC ceiling (claude-orchestrator-walker-split)`

## Notes for the implementer

- Pattern-match: read recently-merged `crates/argo-agent-runner/src/turn.rs` + sibling `turn_helpers.rs` (post-#1097) for a clean template.
- Pick one logical sub-concern, not a random LOC-count cut.
- Common candidates: helper free-functions, internal sub-types, state-machine transition tables.

## Unblocks

- Future handoff/policy/sweep briefs that extend walker.rs without hitting the ceiling.
