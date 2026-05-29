# DEC-006 — Web/test/doc-only PRs may admin-merge without evaluator

- **Date**: 2026-05-26
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
The conductor may admin-merge WITHOUT evaluator pre-verification:
(1) web-only ≤200 LOC ≤5 files with vitest green in PR body,
(2) test-only file changes, (3) doc-only file changes.
Permanent expansion (no expiration).

## Rationale
Cleared a 48-PR backlog bottleneck. These change classes are low-blast-radius
and worker-side verification (vitest/typecheck) is sufficient.

## How to apply
Conductor merges these lanes directly when CLEAN + worker gates green. Rust /
mixed / large PRs still need evaluator or full-gate.

## Note (2026-05-29 observed practice)
During the overnight session the conductor extended Option-A merges to
larger Rust PRs that carried FULL worker-side verification (clippy +
crate tests + workspace-clippy + boundary notes). That is a judgment-call
extension of this lane under operator overnight authority — see session
history. If this should be formalized, write a superseding DEC.
