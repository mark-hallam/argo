# Mission load-bearing wire — Reconcile MISSION_BACKLOG status tags vs BUILD_STATUS ledger

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `1e01c3a7` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #276 (in spirit). `MISSION_BACKLOG.md` has ~60 lines still tagged `[NEW]` whose underlying work has already shipped or has a brief. The ledger (`BUILD_STATUS.md`) is authoritative per HARD RULE 1, but workers reading the backlog may waste a claim attempt on a "new" item that's actually MERGED. Sweep and reconcile. **Tier-Doc / ledger hygiene.**

## Scope

Single-purpose reconciliation pass:

1. **Enumerate every `[NEW]` row** in `docs/build-plan/MISSION_BACKLOG.md` (currently ~60).
2. **For each, search the ledger** for matching `mission-<slug>` rows:
   - If a MERGED row exists → flip tag to `[SHIPPED]` and reference the PR number in a trailing comment: `[SHIPPED]  ← #1479`
   - If a TODO / CLAIMED / IN_REVIEW / BLOCKED row exists → flip tag to `[BRIEF: mission-<slug>]`
   - If genuinely no row exists AND the topic still feels operative → keep `[NEW]`
   - If genuinely no row exists AND the topic is stale (e.g. references concepts that were superseded) → flip to `[STALE]` with a one-line note pointing at the superseding row
3. **Append a short reconciliation log** at the end of `MISSION_BACKLOG.md` (last 20 lines) recording the date + the count of each transition: `Reconciled YYYY-MM-DD: N [NEW]→[SHIPPED], M [NEW]→[BRIEF:…], K [NEW]→[STALE]`.

NOT in scope: editing `BUILD_STATUS.md` (read-only reference); inventing new briefs for genuinely-orphan backlog items; changing the backlog's numbering scheme; deleting backlog entries.

## Files you may touch

- `docs/build-plan/MISSION_BACKLOG.md` (≤ 800 LOC after — currently ~700)

## Files you must NOT touch

- `docs/build-plan/BUILD_STATUS.md` — authoritative; the backlog reconciles TO the ledger, never the other way
- Any `streams/*.md` brief
- Any code
- Any other docs

## Acceptance criteria

- [ ] Every previous `[NEW]` row has either: (a) flipped to `[SHIPPED]` / `[BRIEF: …]` / `[STALE]`, or (b) kept `[NEW]` with a deliberate decision
- [ ] PR numbers cited for every `[SHIPPED]` transition
- [ ] Brief slugs cited for every `[BRIEF: …]` transition
- [ ] Reconciliation log appended at the file foot
- [ ] Zero edits to BUILD_STATUS.md or to any code file (verify via `git diff --stat`)
- [ ] All touched files < 500 LOC (the backlog file is large but is its own measurement; sub-section additions should be minimal)
- [ ] BUILD_STATUS.md row updated (this brief's own row)

## Branch + PR

- Branch: `claude-mission-reconcile-backlog-tags-vs-ledger`
- PR title: `docs: reconcile MISSION_BACKLOG status tags against ledger (mission-reconcile-backlog-tags-vs-ledger)`

## Notes

- This is **read-the-ledger, edit-the-backlog**. The reconciler reads `BUILD_STATUS.md` for ground truth and edits `MISSION_BACKLOG.md` accordingly. The author should NOT introduce contradictions.
- Recommended approach: `grep -E 'mission-<slug>' docs/build-plan/BUILD_STATUS.md` for each backlog item's slug, then route per the rules above. A small helper script in `/tmp` is fine but does not need to be committed.
- Worker may find conceptual matches without exact slug overlap (e.g. backlog "key rotation" → ledger `mission-automated-key-rotation`). Use judgment; record any non-obvious mapping in the trailing comment.
- Edge case: some backlog topics have been **partially** shipped (e.g. design-doc only, no implementation). Tag those `[BRIEF: <slug>]` if a follow-up brief exists, else `[NEW]` with a comment noting the partial state.
- If you finish the sweep in <300 lines of diff, that's expected and good — most of the work is reading, not writing.
