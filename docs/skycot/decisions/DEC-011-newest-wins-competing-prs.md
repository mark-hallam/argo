# DEC-011 — Newest-wins on competing PRs

- **Date**: 2026-05-28
- **Decided by**: Mark Hallam (operator)
- **Status**: active
- **Supersedes**: —

## Decision
When multiple open PRs target the same surface with different approaches
(competing pair/trio), the NEWEST PR wins — close the older sibling(s) with
a SUPERSEDED-BY-NEWER rationale.

## Rationale
The newest PR has the most current context (later main state, more recent
audit findings). Avoids merge-order ambiguity + duplicate work.

## How to apply
During PR triage, detect same-surface competing PRs, keep newest open, close
older with citation. Examples handled under this rule: #2370 vs #2494,
#2474/#2567/#2591 settings-h1 trio.
