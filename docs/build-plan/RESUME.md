# RESUME — fresh-session entry point

A new agent picking up SKYCOT (Argo) work reads this page first.

## What this repo is doing right now

Multi-agent swarm shipping v1.0 of SKYCOT (the operator-facing
product surface on top of the Argo core). Workers claim briefs from
`docs/build-plan/streams/`, ship PRs, and a single conductor process
reviews + merges + replenishes the queue.

**Codex is offline** since 2026-05-09 (out of credit). Workers run
on the `claude` side only.

**v1.0 ship gate**: `docs/skycot/demos/v1-mission-mvp.mp4` exists
AND `./scripts/smoke-skycot.sh` passes including BrowserExecutor +
Slack-approval scenarios.

## Read in this order (5 files, ~20 min)

1. **`docs/build-plan/BUILD_STATUS.md` — first ~90 lines.** The live
   ledger header. Operator amendments (queue thresholds, paused
   streams, strategic priority, active directives) are here and
   override every other doc. Memory and template defaults are NOT
   authoritative; the header is.

2. **`docs/build-plan/SWARM_PROTOCOL.md` — HARD RULES section + the
   iteration sequence.** Source of truth for: claim → PR → merge
   loop, hard rules, verification, stop condition, graceful exit.
   Re-read on every iteration.

3. **`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`** (conductor only).
   The 9-step tick prompt the conductor runs every loop iteration.

4. **`docs/build-plan/PRODUCT_ROADMAP.md`** (frontend / UX work).
   Operative plan for the post-MVP product-surface program (Tracks
   A–G). Read before touching any UI / persona / journey row.

5. **Active + queued operator directives**, named in the
   BUILD_STATUS.md header's "Live directives" block:
   - **ACTIVE**: `conductor-prompt-deployment-path-v3.md` (drives
     swarm to "deployable end-to-end" — overrides PROTOCOL_LOOP_PROMPT
     step-8 replenishment until §5 stop conditions met).
   - **QUEUED (succeed v3)**: `conductor-prompt-post-v3-directives.md`
     carries BLOCK 1 (SKILL-CANVAS-v1, Track L, 17 briefs) +
     BLOCK 2 (ENTERPRISE-PARITY-v1, Track M, 19+1 briefs, parallel
     to remaining L). Operator pastes each at its trigger. L0+M0
     design docs are `streams/L0-skill-canvas-track-overview.md`
     + `streams/M0-enterprise-parity-track-overview.md`.

## Your role — pick one

- **Worker** (most agents). Read `CONTRIBUTING_AGENT.md` + the
  worker prompt for your model (`PROTOCOL_WORKER_PROMPT_CLAUDE.md`).
  Pick a TODO brief, claim it via `scripts/claim-with-retry.sh`,
  implement, push a PR, exit gracefully after 2-4 PRs.

- **Conductor** (singleton — exactly one across all machines).
  Run `PROTOCOL_LOOP_PROMPT.md` on a 5-minute loop. Review PRs,
  merge clean ones, flip ledger rows, replenish when queue ≤25.
  **Never write feature code.** Mutation surface: PR review/merge,
  BUILD_STATUS.md edits, brief authoring in `streams/`.

- **Ad-hoc session**. If you weren't invoked as a swarm worker or
  conductor, you can ignore the build-plan layer. CLAUDE.md root
  rules still apply.

## What's NOT operative

The following are historical / superseded. Read only for context:

- `BUILD_PLAN_2WK.md` — original 14-day v1 sprint, completed
  tick-57 era. Retained for ship-gate definition.
- `BUILD_PLAN_V11.md` — backend phased plan; backend tracks
  substantively shipped. Live work is now mission-* briefs in
  `streams/` driven by BUILD_STATUS.md, not phase-based.
- `docs/plan/argo-build-plan.md` (v0.2) — core-platform architecture
  reference. Pre-Phase-A; consult only for original invariants.
- `conductor-advisory-{E0-E1,E2-E5,F-reconciliation,G-J}.md` — four
  advisories, all ACTIONED. Rows are inserted in BUILD_STATUS.md;
  see ledger for live state.

## Ground truth, by question

- "What should I work on?" → next TODO row in BUILD_STATUS.md whose
  brief file passes `scripts/check-brief-freshness.sh`.
- "What's the queue-depth threshold?" → BUILD_STATUS.md header
  (currently ≤25, raised by operator 2026-05-10).
- "What's the v1.0 ship gate?" → demo .mp4 + extended smoke per
  BUILD_STATUS.md header.
- "Is codex active?" → no, offline 2026-05-09. Skip codex side.
- "Is brief X stale?" → run `bash scripts/check-brief-freshness.sh
  <brief-id>` before claiming.
- "Where do I report what I shipped?" → BUILD_STATUS.md row flip +
  `status: <brief-id> in review (#<num>)` commit message.

## When the ledger and a doc disagree

**The ledger wins.** Hard Rule 1 from SWARM_PROTOCOL.md. The
BUILD_STATUS.md header is updated every tick; static docs drift.
If a static doc says one thing and the header says another, trust
the header and (if you are conductor) flag the drift for cleanup.

---

## Right-now snapshot (2026-05-13 ~02:04Z)

Refreshed during a resume-safety pass. The previous
conductor session (Claude lane A) was hitting weekly quota; this
snapshot was written from Claude lane B which is now itself
approaching the weekly cap. **A fresh session resuming
should read this whole section + the SWARM_PROTOCOL + the 5 files
named at the top of this doc, then start ticking.**

### Live state (origin/main)

- **HEAD**: `157edfcc` (status: claim J12-gateway-retrain-endpoints)
- **Queue counts** (run `grep -cE '^\| .* \| claude \| <STATE> '
  docs/build-plan/BUILD_STATUS.md` to verify):
  - TODO = **17**  (below 25 trigger but workers actively pulling)
  - CLAIMED = **4**  (briefs in active worker progress, no PR yet)
  - IN_REVIEW = **17**  (open PRs)
  - BLOCKED = **33**  (mostly dep-chain; many unblock-cascade as
    in-flight PRs land)
  - MERGED = **220**  (live ledger; +463 archived to
    `BUILD_STATUS_ARCHIVE_2026Q2.md` pre-#1300)

### What just landed (last ~4 hours conductor work)

Major push:
- 🎯 **K5 v1.0 ship-gate** MERGED (#1490) — deploy-smoke-test-e2e
- 🎉 **Track E5 100% complete** — all 8 audit briefs (a..h) MERGED
- 🎉 **Track E4 complete** — all worker-profile dialogs MERGED
- 🎉 **Track E2 mostly complete** — Step1–6 MERGED; E2-f/i CLAIMED
- 🔓 **Track J 7/8/9/10/13/15 endpoints** all MERGED; J11/J12 in flight
- 🚨 **4 unblock-swarm split-preps** all MERGED in <2h (#1524–#1527
  for sections/store-lib/doctor/routes-dispatch-post)
- 🎉 **Codex back online 2026-05-13** — first codex worker claimed
  `claude-approval-timeout-default-decision` (in active worker
  progress, no PR yet); `PROTOCOL_WORKER_PROMPT_CODEX.md` refreshed
  for resumption

### CLAIMED — workers in flight (no PR yet)

1. **`claude-approval-timeout-default-decision`** — codex worker,
   active. Used `ARGO_CLAIM_FORCE=1` to bypass boot.rs 498-LOC
   ceiling. Implementation in `/tmp/claude-approval-timeout-default-
   decision/` worktree. Cargo fmt mis-fire being reverted.
2. **`mission-enterprise-audit-full-text-search`** — pre-existing
   claim, stale (since 2026-05-12T16:45Z).
3. **`E2-f-hire-flow-structured-adjust`** — claude worker.
4. **`E2-i-hire-flow-day1-wrap-duty`** — claude worker.

### IN_REVIEW — 16 open PRs (all DIRTY or UNSTABLE, fix-recipes on file)

Two **MERGEABLE/UNSTABLE** (passing-but-CI-policy):
- **#1498** `mission-vulnerability-management-workflow` — fix-recipe
  posted: vuln-scan workflow PR introduces fires on its own lockfile
  touch + correctly flags pre-existing advisories (chicken-egg).
  Recipe: seed `vuln-waivers.toml` baseline OR `continue-on-error:
  true` for first run OR cron-only trigger.
- **#1506** `mission-cli-help-public-api-reference` — fix-recipe
  posted: drift CI workflow PR introduces fails on its own
  CI-vs-local terminal-width regen diff. Recipe: force `COLUMNS=120`
  + strip ANSI in the script.

Fourteen **DIRTY** (chain-rebase backlog from recent merges):
#1447, #1450, #1461, #1463, #1464, #1466, #1467, #1469, #1472,
#1482, #1494, #1503, #1504, #1511. Each has a fix-recipe comment;
standard pattern is `gh pr checkout <N> && git fetch origin main
&& git merge origin/main`, additive route-dispatch conflicts take
both sides. Stalest is #1461 (~12h, but the row is already
STALE-DEMOTED to TODO with salvage note — PR stays open).

### Worker / quota situation

- **Claude workers** (across 1+ machine, claude-worker-loop.sh):
  exhausted weekly cap mid-tick. Will resume on cap reset.
- **Codex** (operator-driven, single tab as of writing): live; one
  brief claimed, implementation ~80% done. May continue or pause.
- **This conductor session (Claude lane B)**: approaching weekly
  cap; this snapshot is the hand-off artifact.

### What a fresh conductor (Claude C) should do first

1. `cd "$ARGO_REPO_ROOT" && git fetch origin main && git reset --hard origin/main`
2. Read this `RESUME.md` first 100 lines, then `BUILD_STATUS.md`
   first 90 lines (header).
3. Run the 9-step conductor tick (or arm `/loop 5m` with the prompt
   from `PROTOCOL_LOOP_PROMPT.md`).
4. Expect codex worker to push `claude-approval-timeout-default-
   decision` PR soon — Tier-1 review + merge per the usual pattern.
5. The 14 DIRTY PRs need their workers to do `git fetch && git merge
   origin/main && git push` (recipe on each PR). Conductor's job is
   wait, not push — those workers are on quota-pause but their
   recipes are clear when they resume.

### Active directives (header is authoritative; do NOT trust this
list if it disagrees)

- **ACTIVE**: `conductor-prompt-deployment-path-v3.md` — §5 stop
  condition technically met by K5 #1490 first green run. Operator's
  call to formally close v3 and paste BLOCK 1 SKILL-CANVAS-v1.
- **QUEUED**: `conductor-prompt-post-v3-directives.md` BLOCK 1
  (SKILL-CANVAS-v1, Track L) + BLOCK 2 (ENTERPRISE-PARITY-v1,
  Track M) per `streams/L0-skill-canvas-track-overview.md` +
  `streams/M0-enterprise-parity-track-overview.md`.

### Pending operator decision (unactioned by conductor)

The operator pasted a tools/mission-dashboard improvement spec
(4 changes: cross-link, advisory tracker, archive-aware burn-up,
status-weight completeness). Conductor offered three execution
paths (author brief / implement directly / acknowledge that
operator will drive it). **No response yet** — current conductor
behaviour is to wait for operator confirmation rather than
implement Python operator-tooling unilaterally. Fresh conductor
inheriting this: leave it alone unless operator re-asks.

## When this file is stale

The conductor refreshes it during freshness passes. If you read
this file and the snapshot dates look weeks old, treat the
BUILD_STATUS.md header as authoritative and ignore the snapshot.

Last refreshed: 2026-05-13 (session hand-off; Claude
lane A → B → potentially C).
