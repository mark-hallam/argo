# cu5-spa-approve-and-execute-held-actions

**Owner**: claude or codex (web + thin gateway).
**Depends on**: cu2 (dispatch) for the execute half.
**Authored against**: origin/main `5f81ae776` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu5-spa-approve-and-execute-held-actions

## Goal (DEC-050 G5)

Close the human-in-the-loop loop IN the product: when a teammate's action is
held (probation / approval-gated), the operator approves it in the SPA and it
EXECUTES — no CLI, no manual re-issue.

## Why now (live 2026-06-11)

The only unblock today is `argo session approve --tool <t> <session-id>` +
manually re-issuing the turn (CLI-only, new-session-per-Run-now makes it
awkward). The approval-first story is the differentiator but isn't completable
through the UI.

## Fix

1. Surface held tool-dispatches / approval-gated actions in the Approvals inbox
   as approvable items (they already emit distinct audit kinds — probation vs
   escalation).
2. An "Approve & run" action that: records the session grant (or graduates
   probation for that worker, per policy) AND re-dispatches the held action via
   the cu2 inline dispatcher — so approval → real execution, in one click.
3. Audit both the approval and the resulting execution (hash-chained).

## Acceptance / verifier

1. Live: Run-now → action held → operator clicks Approve in /approvals →
   action executes → real artifact + audit rows for approve + execute.
2. Cross-tenant: operator A cannot approve tenant B's held action.
3. vitest + `cargo test -p argo-cli` green.

## Size / owner / priority

~150-250 LOC. **🟠 P2 — DEC-050 G5, the approval→work loop.** Makes the safety
spine a usable workflow, not just a CLI escape hatch.
