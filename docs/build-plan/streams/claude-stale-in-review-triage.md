# claude-stale-in-review-triage — investigate 18 stale IN_REVIEW PRs and produce a conductor action report

**Owner**: claude. **Depends on**: nothing. **Phase A swarm hygiene — high-leverage one-shot.**

**Authored against**: origin/main 48991cdb on 2026-05-09 (conductor-authored after observing 18 IN_REVIEW PRs >2h stale on the dashboard with no recent worker activity).

## Why

18 PRs are currently IN_REVIEW with no worker push in 2-5 hours. The conductor's per-tick stale-IN_REVIEW sweep handles them mechanically (af500037 demote-to-TODO), but mechanical sweep is a blunt instrument: some of these PRs may be **fully ready to merge** and just need a verification pass. Sweeping them would waste hours of worker work, force a re-claim/re-rebase cycle, and lose merge-window time.

This brief delegates the **investigation** half of the sweep to a worker, leaving the **action** (merge/sweep/hold) to the conductor. The conductor saves the per-PR verification context-cost; the swarm gets faster turnaround on PRs that are silently mergeable.

**Not in scope**: this brief does NOT merge any PR (HARD RULE — only the conductor merges). It produces a structured report.

## Approach (concrete)

Iterate the 18 stale IN_REVIEW PRs (snapshot below). For each:

1. **Fetch PR state**: `gh pr view <num> --json state,mergeable,mergeStateStatus,headRefOid,updatedAt,statusCheckRollup,comments`.
2. **Check last activity**: was the most recent event a worker push, a conductor comment, or a force-push rebase? If conductor commented after worker push, that's a "pending fix-request"; if worker rebased after conductor comment, the fix-request is addressed.
3. **Crate-scoped Tier 2 verification on a worktree**: `git worktree add /tmp/triage-pr<num> origin/<branch>`, run targeted clippy + tests for the may-touch crate(s) only. ~30s-2min per PR with shared target dir.
4. **Classify** into one of:
   - `READY-TO-MERGE` — mergeable=CLEAN, Tier 2 verification passed locally, no pending fix-request, no conflicts.
   - `NEEDS-REBASE` — mergeable=CONFLICTING. Worker would need to rebase + re-push. Conductor decision: sweep (force fresh worker) vs. wait.
   - `NEEDS-WORKER-FIX` — has a pending fix-request comment (conductor comment newer than last worker push). Worker needs to address.
   - `LOCAL-VERIFICATION-FAILED` — clippy or targeted tests fail on the worktree. Either real regression or post-merge drift in the base; mark for conductor inspection.
   - `BOUNDARY-NOTE-FLAKES` — Tier 2 only fails on the documented post-#1039 strict-mode flakes (or KNOWN_FLAKES.md entries). Implementation actually clean.

5. **Write the report**: a single `docs/build-plan/STALE_IN_REVIEW_TRIAGE_<YYYY-MM-DD>.md` with one section per PR. Each section: PR number, title, classification, evidence (last-push vs last-comment timestamps, mergeable status, Tier 2 result), suggested conductor action.

## Files you may touch

- `docs/build-plan/STALE_IN_REVIEW_TRIAGE_2026-05-09.md` (NEW, ≤ 800 LOC — long but bounded by 18 PRs × ~40 LOC each)

## Files you must NOT touch

- Any source code in `crates/`
- Any other markdown in `docs/`
- Any of the IN_REVIEW PRs themselves (no rebases, no force-pushes, no comments — investigation only)
- `BUILD_STATUS.md` (conductor flips status based on this report)

## PR snapshot to triage (18 PRs as of 2026-05-09T14:13Z)

| PR | Brief | Stale (min) |
|---|---|---|
| #995 | claude-mcp-server-health-check | 172 |
| #1082 | claude-mcp-server-rate-limit | 172 |
| #1070 | claude-worker-okr-cli | 162 |
| #1076 | claude-worker-team-roster-cli | 311 |
| #1071 | claude-worker-success-metrics-vis | 325 |
| #989  | claude-genesis-memory-schema-builder | 167 |
| #1075 | claude-genesis-iteration-cycle | 311 |
| #1074 | claude-tenant-quota-enforcement-stricter | 312 |
| #1077 | claude-rate-limit-per-route | 307 |
| #1072 | claude-error-types-consolidation | 315 |
| #1078 | claude-config-loader-overhaul | 305 |
| #1084 | claude-latency-percentiles-tracking | 145 |
| #1083 | claude-incident-postmortem-template | 146 |
| #1085 | claude-runbook-quickref-html | 137 |
| #1086 | claude-runbook-skycot-split | 137 |
| #1087 | claude-channels-push-voice-split | 126 |
| #1088 | claude-doctor-channels-split | 121 |
| #1089 | claude-cu-executor-extract-helper | (added) |

## Acceptance criteria

- [ ] All 18 PRs classified into one of the 5 buckets above
- [ ] Each entry has: PR number, title, classification, evidence (last-push timestamp, last-comment timestamp + author, mergeable status, Tier 2 result with test counts), 1-line suggested conductor action
- [ ] Worktrees cleaned up after each PR's verification (`git worktree remove --force`)
- [ ] No PR-side modifications (no comments, no rebases, no force-pushes)
- [ ] Report file ≤ 800 LOC (each entry concise, ~40 LOC max)
- [ ] Final summary at top of report: bucket counts (e.g., "READY-TO-MERGE: 4, NEEDS-REBASE: 6, ...")

## Branch + PR

- Branch: `claude-stale-in-review-triage`
- PR title: `docs: stale IN_REVIEW triage report 2026-05-09 (claude-stale-in-review-triage)`

## Notes for the implementer

- This is mostly bash + `gh` + Python data-extraction work; minimal cargo time. Each Tier 2 verification reuses the shared target dir, so most should be warm-cache fast.
- For `NEEDS-WORKER-FIX` cases, **read** the fix-request comment but **do not respond** on the PR. Just summarize the requested change in the report.
- For `LOCAL-VERIFICATION-FAILED`, capture the failing test names and (if obvious) the suspected cause — but don't try to fix anything.
- Use `scripts/conductor-status.sh` for the live snapshot; the table above may have shifted by the time this brief runs.
- `KNOWN_FLAKES.md` is the reference for what counts as a `BOUNDARY-NOTE-FLAKES` classification. If `claude-known-flakes-refresh` has merged before this runs, use the updated list.

Use the conductor's pre-existing tooling (don't reimplement):
- `bash scripts/conductor-status.sh --json` for the live stale list
- `bash scripts/check-pre-claim-ceiling.sh <brief-id>` to verify any may-touch ceiling concerns
- Worktree per PR; clean up with `git worktree remove --force`

## Unblocks

- The conductor's next stale-IN_REVIEW sweep tick. With this report in hand, the conductor can do focused merges (READY-TO-MERGE), targeted sweeps (NEEDS-REBASE), and per-PR worker-fix-request escalations (NEEDS-WORKER-FIX) — instead of one mechanical demote-everything pass.
- Saves an estimated 4-6 PRs from being needlessly swept (those that are READY-TO-MERGE but appeared stale only because the conductor hasn't gotten to them).
