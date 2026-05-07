# Mission load-bearing wire #6 — wire Judge into forge_optimize auto-promote

**Owner**: claude. **Depends on**: claude-skill-quality-judge (#886), argo-judge crate present, mission-wire-judge-on-skill-creation merged (mirrors integration shape).

> SOURCE: deep-research findings tick 198, Section 1, item #7.
> Forge today auto-promotes the best-scoring iteration regardless
> of judge verdict, and the documented `auto_promote_threshold = 0.85`
> is not enforced. This brief enforces both.

## Scope

[`crates/argo-cli/src/handlers/forge_optimize.rs`](crates/argo-cli/src/handlers/forge_optimize.rs)
exposes `run_with_events_and_run_id()` as the canonical optimize
loop: iter-0 baseline, mutator emits N variants, eval scores them,
the highest-score variant becomes the candidate. Today the
candidate is promoted unconditionally — no judge consult, no
threshold gate.

This brief inserts two checks before promotion:

1. **Judge gate**. Same Judge trait as
   `mission-wire-judge-on-skill-creation`; verdict below
   `ARGO_JUDGE_FORGE_FLOOR` (default 0.7, higher than the proposal
   floor since promotion is more consequential) → promotion
   refused, run marked `judge_blocked`, audit row written.
2. **Threshold gate**. The eval-score must be ≥
   `auto_promote_threshold` (env override
   `ARGO_FORGE_PROMOTE_THRESHOLD`, default 0.85). Sub-threshold →
   promotion refused, run marked `below_threshold`.

Both gates fail-closed (refusal preserves the previous active
version). Operators can manually promote via existing CLI if
they choose to override.

## Files you may touch

- `crates/argo-cli/src/handlers/forge_optimize.rs` — narrowly:
  insert two gate checks before the promote call; emit audit rows;
  keep file under 500 LOC (currently ~435; this adds maybe 60)
- `crates/argo-cli/src/handlers/forge_optimize_judge.rs` (NEW,
  sibling) — judge-call helper + threshold-check helper +
  audit-row writer
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE
  `pub(crate) mod forge_optimize_judge;` line
- `crates/argo-cli/tests/forge_optimize_judge_smoke.rs` (NEW) —
  integration smoke: forge run with a mock judge that returns
  rejection; assert no promotion + audit row written

## Files you must NOT touch

- `crates/argo-judge/**` (locked)
- `crates/argo-skill/src/store_vertical.rs` — separate brief
- The Forge trait stub (`StubForge`) — separate concern; this
  brief targets the working `forge_optimize.rs` path

## Acceptance criteria

- [ ] Judge gate runs after eval scoring, before promote call
- [ ] Judge verdict < floor → no promotion, audit row
      `forge_promote_judge_blocked` with verdict
- [ ] Threshold gate: eval-score < `auto_promote_threshold` → no
      promotion, audit row `forge_promote_below_threshold`
- [ ] Both gates pass → promote as today
- [ ] Floor and threshold env-overridable
- [ ] Existing forge tests pass unchanged (back-compat)
- [ ] 5 new unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-wire-judge-on-forge-promote`
- PR title: `forge: judge + threshold gates on auto-promote (mission-wire-judge-on-forge-promote)`

## Notes for the implementer

- Mirror the integration shape from `mission-wire-judge-on-skill-creation`.
  If the helper module shape converged, this brief can reuse it
  via re-export.
- DO NOT touch the Forge trait stub. The trait is a separate
  technical-debt item flagged in MISSION_BACKLOG.md item #84.
- Fail-closed on judge availability for promotion is deliberate
  (opposite of skill creation). Promotion is consequential; if
  the judge is degraded, defer the promotion until the judge is
  available rather than promote with no quality check.
