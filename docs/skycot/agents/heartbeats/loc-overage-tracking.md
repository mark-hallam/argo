# LOC overage tracking — MVP overnight window 2026-05-25→26

Authorized by operator 2026-05-25T15:55Z via BUILD_STATUS.md OVERNIGHT DIRECTIVE §10 (memory `feedback_loc_overage_550_soft_ceiling_mvp_window_2026_05_25.md`).

**Soft ceiling**: 500 LOC → 550 LOC for files modified by `mission-genesis-*` / `pitch-*` / `fix-pr-22*-rebase*` PRs during window.
**Hard ceiling**: 550 LOC (files >550 still hit gate-3 LOC budget block).
**Window**: 2026-05-25T15:40Z → 2026-05-26T22:00Z AEST.

## Tracking format
One line per breach: `<file_path> @ <LOC count> in #<PR> (<brief-slug>)`

## Breached files

_(none yet — conductor ticks append as PRs merge with 500-550 LOC files)_

## End-of-window action

At T20-00Z 2026-05-26, conductor authors one `fix-split-<file>-post-mvp` brief per breached file. Operator prioritizes these post-launch.
