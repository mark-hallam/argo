# DEC-015 — Decision register ships as ADR files first

- **Date**: 2026-05-29
- **Decided by**: Mark Hallam (operator) + conductor recommendation
- **Status**: active
- **Supersedes**: —

## Decision
Implement the operator decision register as append-only ADR markdown files
under `docs/skycot/decisions/` (this directory) with a README index. Do NOT
build a SQLite-backed register yet — only graduate to a DB if the file
approach hurts (querying, cross-referencing, or a product surface needs it).

## Rationale
Operator asked whether a decision register is a good idea + how to implement.
Conductor recommended files-first: greppable, diffable, loadable at session
start, zero infra. The DB version is a real future product feature ("your AI
workforce's decision log") but premature now.

## How to apply
- Conductor loads `docs/skycot/decisions/README.md` at session start.
- New operator decisions get an ADR + index line at decision-time (same turn).
- Conflict-flagging: before acting on scope/process/routing requests, grep
  the register; surface conflicts with active decisions for explicit override.
- Complements auto-memory (preferences/feedback) — this captures scoped
  commitments + supersession chains.
