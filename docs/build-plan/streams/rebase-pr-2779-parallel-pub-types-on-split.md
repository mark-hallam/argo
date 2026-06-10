# rebase-pr-2779-parallel-pub-types-on-split

Authored against: origin/main ca46b97f1e42d9d093d275763b55a04add3d65c3 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh rebase-pr-2779-parallel-pub-types-on-split

## Goal

Rebase PR #2779 (`fix-parallel-pub-types-gate-enforce`) onto current main and
resolve the structural conflict #2774's split introduced, preserving the PR's
FULL intent. Intent re-validated 2026-06-10 and STILL load-bearing: the
parallel-pub-types gate runs without `--enforce` (silent no-op), and 6
duplicate pub types live on main — restoring the gate is a P1.

## Why now

PR #2779 was authored against the pre-split monolithic
`routes_pitch_ask.rs`; #2774 then split that file into `routes_pitch_ask.rs` +
`routes_pitch_ask_shaping.rs`, so the branch now conflicts (3 regions in
`routes_pitch_ask.rs`, incl. the `PitchChannel` canonicalization and an
import region touching `evaluate_claim`/`ClaimCandidate`). Conductor declined
to hand-resolve unattended (risk of silently dropping intent); the original
author has the context.

## How

1. `git fetch origin claude-fix-parallel-pub-types-gate-enforce && git checkout`
   it (or a worktree); `git rebase origin/main`.
2. Resolve `routes_pitch_ask.rs` against the POST-split layout: keep the
   `#[path="routes_pitch_ask_shaping.rs"] mod shaping;` and the
   `shaping::`/`ChannelKind`/`build_canonical_hit_payload` imports main now
   has; re-apply #2779's `PitchChannel` canonicalization
   (`pub use argo_capability::PitchChannel;` replacing the local enum) on top.
   Re-apply any OTHER #2779 import/logic change (evaluate_claim path) onto the
   split layout WITHOUT dropping it — diff #2779's original commit to recover
   its full intent.
3. Keep all 6 canonicalizations + the `--enforce` flag at
   `conductor-pre-merge-gate.sh:168` + the allowlist entries.
4. Force-push with lease to the same branch.

## Acceptance / verifier

1. `python3 scripts/lint-parallel-pub-types.py --enforce` exits 0 on the
   rebased head (zero duplicates — proves intent preserved).
2. Full `conductor-pre-merge-gate.sh --pr 2779` PASS (compile + clippy + tests).
3. The gate harness case (a synthetic duplicate FAILS the gate) still present.

## Size / owner

Rebase + resolve ~1 file. Owner: claude. 🔴 P1 — restores a drift gate the
protocol assumes is active.
