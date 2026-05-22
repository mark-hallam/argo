# claude-agent-runner-turn-split — split turn.rs to unblock canary + future runner work

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — unblocks #1066.**

**Authored against**: origin/main 64b4b6d1 on 2026-05-09 (conductor-authored after #1066 HARD RULE 6 violation: skill-canary-deployment pushed turn.rs from 496 to 553 LOC).

## Why

`crates/argo-agent-runner/src/turn.rs` is at 496 LOC on current main —
4 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. PR #1066
(`claude-skill-canary-deployment`) added 57 LOC for canary routing
and pushed it to 553 LOC, violating the rule. The PR is held with a
conductor Tier 2 hold pending structural resolution.

The right fix is to split turn.rs and re-pull headroom before #1066
or any future runner work attempts to extend it. Pure refactor; no
behaviour change.

## Approach (concrete)

`turn.rs` likely contains the agent-runner turn-loop entry point plus
helpers (state-machine transitions, error mapping, audit-row stamping,
canary-routing hook stub if present). Split into:

- `turn.rs` — keep the public `run_turn()` entry point + the top-level
  state-machine; ≤ 300 LOC.
- `turn_helpers.rs` (NEW) — extract the helper functions (state
  transitions, audit stamping, error mapping). ≤ 250 LOC.

Re-export the helpers from `turn.rs` via `pub(crate) use` so call
sites in argo-agent-runner stay unchanged.

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` — narrowly: extract helpers,
  re-export
- `crates/argo-agent-runner/src/turn_helpers.rs` (NEW, ≤ 280 LOC)
- `crates/argo-agent-runner/src/lib.rs` — narrowly: 1 `mod turn_helpers`
  declaration if needed (likely re-exported via turn.rs already)

## Files you must NOT touch

- The agent-runner orchestration logic itself — refactor only
- Any test file other than to verify no behaviour change
- The `apply_canary_routing` function added by #1066 — that's #1066's
  may-touch, not this brief's. After this brief lands, #1066 rebases
  onto the post-split main with the fresh headroom.

## Acceptance criteria

- [ ] `turn.rs` ≤ 300 LOC after the split (≥ 200 LOC headroom for
      future work)
- [ ] Extracted sibling ≤ 280 LOC
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings`
      clean
- [ ] `cargo test -p argo-agent-runner` clean (no behaviour change)
- [ ] No public API changes; existing call sites in argo-agent-runner
      and downstream callers unchanged
- [ ] Every modified file ≤ 500 LOC

## Branch + PR

- Branch: `claude-agent-runner-turn-split`
- PR title: `agent-runner: split turn.rs below 500-LOC ceiling (claude-agent-runner-turn-split)`

## Notes for the implementer

- Pattern-match on prior splits in this codebase: `claude-routes-post-split`
  (#932) and `claude-routes-get-split` (#956) are good templates —
  pure-refactor extracts that preserve public API via re-exports. Read
  routes_post.rs (now 17 LOC after #932) and its 5 sibling files to
  see the pattern.
- The `apply_canary_routing` function on #1066's branch may already
  be partially extracted; do not import that — start from clean main.
- If turn.rs has private helpers that are only called from turn.rs
  itself, move them to turn_helpers.rs as `pub(super)` so they stay
  module-private but cross-file accessible. If they're called from
  elsewhere in the crate, use `pub(crate)`.

## Unblocks

- **#1066 claude-skill-canary-deployment** — currently held with
  conductor Tier 2 hold (turn.rs at 553/500). After this brief lands,
  worker rebases #1066 onto post-split main; canary addition fits
  comfortably in the ~200 LOC of new headroom.
- All future agent-runner work that extends the turn-loop primitives.
