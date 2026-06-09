# split-eight-over-loc-gate-blockers

Authored against: origin/main cb7d4cfb23d993d758d8d61c987d3b3bd73bee71 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh split-eight-over-loc-gate-blockers

## Goal

Bring the 8 files currently over the 500-LOC hard ceiling back under it via
the established mechanical sibling-extraction pattern, so gate 3 (LOC
budget) of `conductor-pre-merge-gate.sh` stops failing for EVERY Rust PR.

## Why now (conductor evidence 2026-06-10)

Both full-gate runs this morning (PRs #2765 and #2770, isolated worktrees)
failed gate 3 on the same pre-existing list — none of it introduced by
either PR (LOC identical on origin/main, verified per file):

```
532 crates/argo-skill/src/self_improve_cron.rs
514 crates/argo-genesis/src/hire_gate_tests.rs
545 crates/argo-cli/src/handlers/gateway/routes_duties_tests.rs
502 crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs
515 crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs
515 crates/argo-cli/src/handlers/gateway/mod.rs            (516 after #2765's +1 module decl)
511 crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs
516 crates/argo-cli/src/handlers/gateway/boot_spawns.rs
```

A permanently-red gate trains conductors to judge around it (this morning's
merges carried per-file LOC-diff justification by hand). Root fix is the
split. NB `gateway/mod.rs` + `boot_spawns.rs` were split before
(`claude-gateway-boot-split-routes-v2`, #1801, 2026-05-17) and have crept
back — when splitting them this time, leave a `// LOC headroom: keep new
routes in sibling modules` comment at top as the cheap tripwire.

## How (per file, the established pattern)

Mechanical extraction only — no behavior change, no new pub surfaces beyond
`pub(super)`/`pub(crate)` of the moved items (drift-prevention: extend, don't
parallel). Precedents: `routes_approvals_json_helpers.rs` (#2265),
`routes_pitch_ask_public.rs` split-by-purpose, the v2 boot split (#1801).

- test files (`*_tests.rs`, `hire_gate_tests.rs`): split into `_tests2.rs` /
  scenario-grouped siblings (existing convention, e.g. `routes_worker_memory_tests2`).
- `self_improve_cron.rs`: extract the tick helpers to `self_improve_cron_helpers.rs`.
- `routes_pitch_ask.rs`: extract response-shaping helpers to a sibling
  (mirror `routes_pitch_ask_public.rs` which is already separate).
- `routes_genesis_stream.rs`: payload types already live in
  `routes_genesis_stream_payload.rs` — move the SSE frame helpers there or a
  new `_frames.rs` sibling.
- `gateway/mod.rs` + `boot_spawns.rs`: re-run the v2 pattern on whatever
  accreted since #1801.

May be claimed as ONE brief or split per-crate by the worker into stacked
PRs (preferred if any single diff would exceed ~400 LOC moved).

## May touch

The 8 files above + their new sibling modules + the crate `mod.rs` wiring
lines. Nothing else.

## Acceptance / verifier

1. `bash scripts/conductor-pre-merge-gate.sh --skip-clippy --skip-tests`
   gate-3 block reports ZERO files over 500 (paste the gate-3 output).
2. `cargo check -p argo-skill -p argo-genesis -p argo-cli` exit 0.
3. `cargo test -p argo-genesis hire_gate` + `-p argo-cli routes_duties`
   green (the moved tests still run — count unchanged or higher).
4. No new `pub ` items at crate root (grep diff for `^+pub ` → only
   `pub(super)`/`pub(crate)` allowed).

## Size / owner

~8 × mechanical move, est. 150-400 LOC moved per file, near-zero logic risk.
Owner: claude (codex-eligible, fast-mode suits it).
