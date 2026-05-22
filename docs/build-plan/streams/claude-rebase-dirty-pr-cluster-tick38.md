# claude-rebase-dirty-pr-cluster-tick38

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `8683d94d1` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-dirty-pr-cluster-tick38`.

🔴 **P0 CLUSTER REBASE.** Source: auditor T09-40 §4 P0 #2. 5 open PRs went DIRTY in the tick-38 merge cascade (~3-4h ago) and never recovered: **#2016, #2017, #2018, #2021, #2022**. Their ledger rows were demoted to TODO with salvage notes. Branches preserved on remote. Sweep them as a single coordinated rebase pass — either rebase + admin-merge (preferred) or close-as-stale with a re-authored follow-up brief.

## Scope

For each of the 5 stale-DIRTY PRs, attempt this exact sequence:

```bash
git fetch origin main
git checkout <branch>
git rebase origin/main
# If conflicts auto-resolve → push-with-lease
# If conflicts are mechanical (drive-by SQL fix duplicates, AuditKind variant additions, page.tsx LOC shifts) → resolve + push
# If conflicts are genuinely tangled (intent has moved) → close-as-stale, re-author follow-up brief
```

**Known conflict patterns (from earlier triage):**

- **#2017** + **#2021** include drive-by edits to `crates/argo-store/migrations/47__skill_canary.sql` — DROP these during rebase. PR #2025 (claude-migration-comment-aware-splitter) shipped the proper repo-wide fix in tick-40; the drive-by patches are now duplicates.
- **#2016** (TestRunHistoryPanel) conflicts on `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` post-#2013 extraction. Resolution: keep both — the +37 LOC TestRunHistoryPanel additions are mechanical against the smaller post-#2013 file.
- **#2018** (hire idempotency) conflicts on `web/lib/api/workers.ts` + `routes_workers_hire.rs`. Both are additive; preserve both sides.
- **#2021** (test-run gateway-persist) conflicts on `crates/argo-audit/src/event.rs` AuditKind enum — keep both variant additions. Also possible migration slot collision (it added 129/v77; tick-46 may have shifted that).
- **#2022** (concurrent-edit conflict-banner phase-3) conflicts on `crates/argo-store/src/migrations/v43.rs` — accept `theirs` (PR #2025's splitter already shipped).

Per-PR judgment:
- **Easy rebase** (#2016, #2017, #2018): rebase + push-with-lease + admin-merge in this brief.
- **Likely-easy** (#2022): same. Phase 3/3 of the wave-15 builder cluster — load-bearing to formally close that cluster.
- **Maybe-tangled** (#2021): 17 files, 1352 LOC, includes its own migration 129/v77 — if migration slot now collides with #2034 (which also shipped v77 — verify), close-as-stale + re-author with slot renumbering as a sibling brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- Each of the 5 branches in turn — work the rebase locally in a fresh worktree.
- `docs/build-plan/BUILD_STATUS.md` — narrowly scoped: flip the 5 demoted-DIRTY rows back to IN_REVIEW (or MERGED on the ones that merge cleanly).
- `streams/claude-rebase-dirty-pr-cluster-tick38.md` — this brief's own row.

## Files you must NOT touch

- The 5 branches' source code beyond the rebase resolution itself. The intent of each PR is locked.
- Any other PR or brief.
- Any production code outside the conflict resolutions.

## Acceptance criteria

For each of the 5 PRs (#2016, #2017, #2018, #2021, #2022):

- [ ] Attempted rebase against current `origin/main`.
- [ ] If rebase succeeded + conflicts resolved + tests pass:
  - [ ] `git push --force-with-lease origin <branch>` succeeded.
  - [ ] PR re-flagged as MERGEABLE / CLEAN.
  - [ ] PR admin-merged after `bash scripts/conductor-pre-merge-gate.sh` exits 0.
  - [ ] BUILD_STATUS.md row flipped to MERGED with the PR number.
- [ ] If rebase failed (genuinely tangled — intent has moved):
  - [ ] PR commented with "Closing as stale; follow-up brief at `streams/<new-brief>.md`."
  - [ ] PR closed (operator-authorized via THIS brief — confirm in PR body of THIS PR).
  - [ ] Follow-up brief authored at `streams/claude-followup-pr-<num>-<short-desc>.md`.
  - [ ] BUILD_STATUS row updated to TODO with reference to the follow-up.
- [ ] PR body of THIS brief includes a per-PR row: number / outcome (MERGED / CLOSED-AS-STALE) / one-line reason.
- [ ] No regression on any unrelated test suite.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-rebase-dirty-pr-cluster-tick38`
- PR title: `recovery: rebase 5-PR tick-38 DIRTY cluster (claude-rebase-dirty-pr-cluster-tick38)`

## Notes for the implementer

- ~50-150 LOC for THIS brief's own PR (mostly: the markdown PR body summarizing each rebase outcome, plus the 5 ledger row updates). The actual rebase work happens in the existing branches.
- This is OPERATOR-AUTHORIZED to close PRs that genuinely can't be rebased. The protocol's "do not close PRs" rule has a carve-out for recovery briefs — see SWARM_PROTOCOL.md HARD RULE 10's pairing with the rebase-PR-N precedent (#911 / #905 / #894 / #924).
- Start with **#2017** — the smallest conflict surface (drop the drive-by SQL fix). If that's a clean rebase, the pattern is proven for #2021's drive-by too.
- The 5 PRs together represent ~3000 LOC of worker work that already passed verification. Each conflict is mechanical post-cascade. Don't redo verification — trust the workers' PR-body acceptance ticks; only re-run targeted tests if the rebase materially changed file content.
- **Take this brief slowly — do NOT batch-merge 5 PRs in 30 seconds.** The original cascade is what created this mess. Rebase → admin-merge → wait for GitHub to settle → rebase next. Allow ~2 min per PR.
- If migration slot 129/v77 is now occupied by #2034, #2021's slot bump is mandatory. Look at `crates/argo-store/migrations/` after rebase to confirm.
