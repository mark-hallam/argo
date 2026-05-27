# Round 98 — Process drift sweep — empty-commit failure mode + ghost-stamp 6-round carry

**Date**: 2026-05-27T05:00Z (≈15:00 AEST)
**Round**: 98
**Pillar/Claim**: Process / observability (drift detection)
**Verdict**: 🟡 **PARTIAL — P1**

## Setup

Standard process-drift sweep:

```
bash scripts/check-unread-handoffs.sh
bash scripts/check-gateway-freshness.sh
bash scripts/check-findings-only-fix-by.sh
```

Plus a `git log --since="2 hours ago"` scan for empty-content commits claiming ledger/brief work.

## Observed

### Runtime — FRESH

- **Gateway**: `curl http://127.0.0.1:7400/health` → `schema_version: 83, uptime_seconds: 7951`. `check-gateway-freshness.sh` → `gateway_schema=v83 codebase_latest=v83 drift=0` STATUS FRESH.
- **Halt sentinels**: both ABSENT (`/tmp/argo-stop` and `~/.argo/stop` neither exist). Swarm ACTIVE.
- **Worker pool**: `pgrep -fl "claude-worker-loop"` returns 10 active bash loops + claude-CLI instances. Workers are running and claiming.
- **Findings**: 4 within-budget (`eval-cron-duty-fires-agent-runner-end-to-end`, `eval-workflow-execution-runs-real-step`, `mission-hire-audit-payload-shape-validation`, `mission-cu-macos-cross-tenant-capability-test`); 0 due-today; 0 overdue.
- **Drift sweep**: `check-stale-claim-drift.py` not invoked this round (read-only conductor scope) but no obvious staleness in the last-2h merge cadence.
- **Open PRs**: 31. Merge-train cadence healthy — git log shows ~12 transitions (CLAIMED ↔ IN_REVIEW ↔ MERGED) in the last ~3h.

### Process drift — NEW empty-commit failure mode

`git show --stat 6006cc192` (the conductor commit titled `ledger+brief: fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed (R97 P0 anti-theater)`) shows **zero file changes**. Full detail in [`round-98-r97-p0-substrate-defect-unactioned.md`](round-98-r97-p0-substrate-defect-unactioned.md).

This is a new drift class — distinct from ghost-stamping. Ghost-stamps are conductor SHARED.md entries with a stale timestamp; an empty-content commit is a *git plumbing* failure that ships to origin/main and pretends to land work. The commit message is load-bearing for downstream automation (`check-unread-handoffs.sh` accepts the commit as proof of ledger movement; conductor next-tick assumes the brief is in the queue).

Suspected root cause (read-only inference; cannot verify without conductor-tab observation):
- Conductor authored the brief inline (no PR/worker) per its 6-fix mechanical-author pattern from 2026-05-26T18:51Z.
- `git add docs/build-plan/streams/<brief>.md docs/build-plan/BUILD_STATUS.md` either selected zero matching paths (typo? stale glob?) OR the brief file was created in a subdirectory the `git add` didn't reach.
- `git commit -m "ledger+brief: …"` ran against an empty staging area; pre-commit hooks didn't reject (no `--no-verify` here — the gate doesn't check "files actually changed").
- `git push` succeeded because the commit object is valid (no diff is not an error).

### Ghost-stamp pattern — 6th round of carry

`check-unread-handoffs.sh` STDERR continues to surface 4 ghost-stamped conductor entries on SHARED.md (the 5-round pattern flagged in R93-R97 is now 6 rounds):

```
GHOST STAMP: 2026-05-26T15:08:41Z [conductor] tick T15-08 …
  nearest conductor.json commit: 2026-05-27T01:18:18+10:00 (577s away; window=300s)
GHOST STAMP: 2026-05-26T18:51:00Z [conductor] mechanical-author batch complete …
  nearest conductor.json commit: 2026-05-27T04:34:53+10:00 (967s away; window=300s)
GHOST STAMP: 2026-05-26T18:59:00Z [conductor] mechanical-author follow-up …
  nearest conductor.json commit: 2026-05-27T05:07:26+10:00 (506s away; window=300s)
GHOST STAMP: 2026-05-27T08:30:00Z [conductor] tick T08-30 interactive — MERGED #2434 …
  nearest conductor.json commit: 2026-05-27T15:08:12+10:00 (12108s away; window=300s)
⚠️ ghost-stamped conductor entries detected (4)
```

The handoffs script otherwise reports: `✅ No unread handoffs from other agents (cutoff 2026-05-27T05:07:00Z)`. So evaluator → conductor signal is clear; the noise is purely conductor's own stale-stamped entries.

### Findings & gateway — clean

No regressions vs R97.

## Anti-theater check

The empty-commit finding is verifiable from git plumbing alone: `git show --stat <sha>` lists no files. Cannot be argued away. Triple-confirmed against (a) working-tree `git status` showing the brief as `??` untracked, (b) `grep` of BUILD_STATUS.md returning zero hits for the brief name, (c) `gh pr list` returning no PR matching the brief.

The ghost-stamp finding is verifiable from the script output above + comparing `[conductor]` line timestamps against `git log heartbeats/conductor.json` commit times.

## Gap / follow-up brief candidates

### 🟡 P1 NEW — `ops-conductor-pre-merge-gate-block-empty-commits-claiming-ledger-or-brief`

Pre-commit (or pre-push) hook on the operator's clone: if commit message matches `^(ledger|brief|ledger\+brief|status):` AND `git diff-tree --no-commit-id --name-only -r HEAD` is empty, REJECT with `ERROR: commit message claims ledger/brief/status work but commit contains no file changes. Stage the brief and ledger row, then re-commit.`

Why this matters: the empty commit pattern is invisible to all current monitoring — `check-unread-handoffs.sh` treats it as success; `git log` shows the commit message which downstream agents trust; the conductor's own next-tick handoff scan doesn't catch it. The only way it surfaces is when the evaluator notices the substrate defect hasn't moved 24h later. This brief would close that loop.

Scope: ~60 LOC bash hook + 2 fixture commits + 1 lint exclusion (genuine no-op commits like `git commit --allow-empty -m "ci: trigger" pass an explicit allowlist).

### 🟡 P2 NEW — `ops-evaluator-handoff-detect-untracked-brief-files`

Extend `scripts/check-unread-handoffs.sh` to also surface `git status --short docs/build-plan/streams/` rows starting with `??` (untracked) whose filename matches a brief recommended in the most recent evaluator handoff line on SHARED.md. Catches the R97→R98 drift one tick earlier. Scope ~40 LOC.

### 🟢 P3 NEW (carry from R97) — `ops-conductor-tick-emit-current-utc-not-self-stamp`

Conductor tick currently writes `[conductor] tick TXX-YY` where XX-YY is the *scheduled-fire* timestamp, not the actual entry-write time. When the conductor process drifts (interactive overlay, mid-tick file-lock contention), the entry can be stamped 5-12 minutes off from the conductor.json commit timestamp. Recommend stamping with `date -u +%FT%TZ` at write time. Scope ~3 LOC. Pattern now 6 rounds (R93/R94/R95/R96/R97/R98).

## Operator surface

Empty-commit failure mode is the more urgent of the two — it directly caused the 24h substrate-defect carry surfaced in the sibling report. P1 priority recommended; the fix is small and the failure mode is dangerous because it bypasses every existing monitoring path.
