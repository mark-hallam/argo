# fix-pr-2323-rebase-gateway-endpoints — ditch + re-author the gateway endpoints brief from current main

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2323-rebase-gateway-endpoints`.

## Why this is P1

PR #2323 (`fix-gateway-workflows-and-skills-marketplace-endpoints-404`) has been stuck in a 6th-round merge stall. The PR adds `/api/v1/skills` + `/api/v1/workflows` top-level list endpoints. Repeated rebase attempts haven't landed; the underlying intent may have drifted on current main (other route refactors may have changed the mount path). Conductor T22-07 priority (b): "ditch the PR + re-author the gateway-endpoints brief from current main".

## Scope

1. **Diagnose**: read the PR body + last 3 evaluator handoffs that touched #2323 to understand why it's stuck.
2. **Verify on current main**: do the endpoints actually 404 today? Try `curl http://127.0.0.1:7400/api/v1/skills` and `curl http://127.0.0.1:7400/api/v1/workflows` against the running gateway (gateway is on port 7400 per #2283).
3. **If endpoints already work**: close #2323 STALE-FIXED; nothing more to do.
4. **If endpoints still 404**:
   - Close #2323 as superseded.
   - Find the canonical mount path on current main (`crates/argo-cli/src/handlers/gateway/router.rs` or similar).
   - Author a fresh brief from scratch, NOT a rebase: `fix-gateway-workflows-and-skills-list-endpoints-from-current-main-289d3077f`. May-touch list scoped to current router file + the 2 route handler files.
   - Acceptance: `curl /api/v1/skills` returns 200 with a JSON list; `curl /api/v1/workflows` returns 200 with a JSON list; tenant_id-bound (cross-tenant smoke test included).
5. **Report**: write 1-paragraph "why #2323 stalled" memo at `docs/skycot/evals/2026-05-25-pr-2323-stall-postmortem.md` so future conductors learn from it.

## Files you may touch

- `docs/skycot/evals/2026-05-25-pr-2323-stall-postmortem.md` — new.
- The new follow-up brief in `docs/build-plan/streams/` if needed.
- BUILD_STATUS.md to close #2323's row (if STALE-FIXED) or to mark superseded (if re-authored).

## Files you must NOT touch

- Anything under `crates/` directly — this is a meta-task (diagnose + re-author, do not implement).
- PR #2323's branch.

## Acceptance criteria

- [ ] PR #2323 either CLOSED (STALE-FIXED or SUPERSEDED) with operator-visible explanation in the close comment.
- [ ] If still needed, a fresh brief authored against current main `289d3077f` or later.
- [ ] Postmortem written.
- [ ] BUILD_STATUS.md row state correct.

## Branch + PR

- No code branch needed. This is a meta-task that produces docs + ledger updates only.

## Notes

- Closing the PR requires operator approval per the conductor protocol — leave a recommend-close comment, do not auto-close.
- If you re-author, the fresh brief is a SEPARATE TODO row — this brief's row flips to MERGED on producing the postmortem + new brief.
