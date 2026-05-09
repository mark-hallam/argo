# claude-tui-app-split — split argo-tui/src/app.rs to free TUI extension headroom

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — unblocks multiple TUI briefs.**

**Authored against**: origin/main 6b22a1ac on 2026-05-09 (conductor-authored after pre-claim ceiling guard refused `claude-tui-alert-notifications` on `app.rs` at 498/500 LOC).

## Why

`crates/argo-tui/src/app.rs` is at 498 LOC on current main — 2 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. The pre-claim ceiling guard correctly refuses any brief whose may-touch list includes `app.rs`, including `claude-tui-alert-notifications` and any future TUI extension that wires a new key handler or overlay state.

Pure refactor; no behaviour change. Mirrors the proven split pattern from `claude-routes-post-split` (#932), `claude-routes-get-split` (#956), `claude-channels-push-voice-split` (#1087), `claude-doctor-channels-split` (#1088), and `claude-agent-runner-turn-split` (#1097).

## Approach (concrete)

`app.rs` likely contains the top-level `App` struct + multiple cohesive concerns:
- State machine (mode/overlay enums)
- Key dispatch
- Tick/refresh helpers
- Overlay state init/teardown

Two viable splits:

**Option A (preferred)**: keep `App` + key dispatch in `app.rs`, extract one cohesive concern to a sibling.
- Identify the largest extractable section by `wc -l`
- Move to `app_<topic>.rs` sibling (e.g., `app_overlays.rs` or `app_state.rs`)
- Re-export via `pub(crate) use` so call sites are unchanged

**Option B**: extract overlay-init/teardown helpers (likely the largest cohesive group).

Target after split: `app.rs` ≤ 350 LOC (≥ 150 LOC headroom for future TUI extensions).

## Files you may touch

- `crates/argo-tui/src/app.rs` — narrowly: extract sibling, re-export
- `crates/argo-tui/src/app_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod` line if needed

## Files you must NOT touch

- The `App` public API — refactor only
- Any test file other than to verify no behaviour change
- `input.rs` (separate split-prep `claude-tui-input-split` covers it)
- Any overlay implementation file (`overlays_*.rs`) — refactor only the
  init/teardown wiring in `app.rs`

## Acceptance criteria

- [ ] `app.rs` ≤ 350 LOC after the split (≥ 150 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-tui-app-split`
- PR title: `tui: split app.rs below 500-LOC ceiling (claude-tui-app-split)`

## Notes for the implementer

- Pattern-match on prior splits — read `crates/argo-cli/src/handlers/gateway/routes.rs` (the orchestrator post-#932 split) and its sibling files for the proven shape.
- The TUI key dispatch is core hot path; do not move it.
- If the natural split is overlay-init helpers, name the sibling `app_overlays.rs` to mirror the existing `overlays_*.rs` filename pattern.

## Unblocks

- **claude-tui-alert-notifications** — currently refused by pre-claim ceiling guard (app.rs at 498/500). After this lands, becomes claimable.
- Any future TUI extension that wires new key handlers, overlay states, or tick refresh logic.
