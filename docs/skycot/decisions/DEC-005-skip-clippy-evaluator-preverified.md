# DEC-005 — --skip-clippy authorized for evaluator-pre-verified PRs

- **Date**: 2026-05-24
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
The conductor may run `conductor-pre-merge-gate.sh --skip-clippy` when
admin-merging PRs that: (a) an evaluator round PRE-MERGE-PASS'd with fresh
cold cargo within ~6h on the PR head SHA, (b) are conductor-authored
mechanical extractions with conductor cargo-check exit-0, (c) are Track P
PRs, or (d) tenant-binding security wave / doc-only operator-categorized.

## Rationale
Full clippy gate was stalling under worker I/O contention, causing a 5-tick
merge-deferral pattern. The skip is safe when verification already happened.

## How to apply
Use --skip-clippy only within the authorized categories; full worker-side
verification (clippy + tests in PR body) is still required.
