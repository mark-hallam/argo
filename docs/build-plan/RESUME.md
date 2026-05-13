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

## Right-now snapshot (2026-05-13 ~02:30Z) — Claude lane B → C hand-off, codex ACTIVE

Refreshed at 98% weekly cap by the operator
relaunching Claude (this session ends imminently). The previous
lane A snapshot lives in git history `3688c48c` if you need
diff context. **A fresh session resuming should read this whole
section + SWARM_PROTOCOL.md HARD RULES + the 5 files at the top
of this doc, then start ticking.**

### Live state (origin/main)

- **HEAD**: `81f9a672` (status: claim J12-gateway-retrain-endpoints)
- **Queue counts** (run `grep -cE '^\| .* \| claude \| <STATE> '
  docs/build-plan/BUILD_STATUS.md` to verify):
  - TODO = **17**  (below 25 trigger; workers actively pulling)
  - CLAIMED = **8**  (6 claude + 2 codex; briefs in active worker
    progress, no PR yet)
  - IN_REVIEW = **17**  (16 open PRs + 1 row mismatch ≈ same thing)
  - BLOCKED = **33**  (mostly dep-chain; many unblock-cascade as
    in-flight PRs land)
  - MERGED = **233**  (live ledger; +463 archived to
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

**Codex** (2 active claims after shipping #1528):
1. **`claude-error-dashboard-runbook`** — codex (claimed post-#1528)
2. **`claude-troubleshooting-guide-expand`** — codex (claimed post-#1528)

**Claude** (6 active claims):
3. **`mission-doc-executor-gsuite`** — re-claimed after quota release
4. **`mission-encryption-at-rest-sqlcipher`** — re-claimed after quota release
5. **`J12-gateway-retrain-endpoints`** — fresh claim (cascade-unblocked from J10)
6. **`mission-enterprise-audit-full-text-search`** — pre-existing claim, stale (since 2026-05-12T16:45Z)
7. **`E2-f-hire-flow-structured-adjust`** — claude worker
8. **`E2-i-hire-flow-day1-wrap-duty`** — claude worker

### Recently shipped (since previous RESUME snapshot at 3688c48c)

- **#1528** `claude-approval-timeout-default-decision` MERGED by **codex** — first codex PR since 2026-05-09 outage. 10 files +406/-13. Gateway approval-expiry sweeper + `argo doctor approvals --since`. boot.rs net -3 LOC (stayed under ceiling). Used `ARGO_CLAIM_FORCE=1` for the ceiling override; codex pre-push gate clean. Owner column flipped TODO→codex to reflect actual shipper.

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
  released some claims around the cap then re-claimed when cap reset
  partially refreshed. Currently holding 6 active claims (mission/J/E
  briefs). Quota state fluid.
- **Codex** (operator-driven tab): **fully active**. Shipped #1528
  cleanly, then immediately claimed 2 more briefs
  (`claude-error-dashboard-runbook` + `claude-troubleshooting-guide-expand`).
  Codex pre-push gate confirmed working.
- **This conductor session (Claude lane B)**: **at 98% weekly
  cap**; operator relaunching Claude → this session ends imminently.
  This snapshot is the hand-off artifact for whatever account picks
  up next.

### What a fresh conductor (Claude C/D) should do first

1. `cd "$ARGO_REPO_ROOT" && git fetch origin main && git reset --hard origin/main`
2. Read this `RESUME.md` first 100 lines, then `BUILD_STATUS.md`
   first 90 lines (header).
3. Run the 9-step conductor tick (or arm `/loop 5m` with the prompt
   from `PROTOCOL_LOOP_PROMPT.md`).
4. **Expect codex PRs to land** for `claude-error-dashboard-runbook`
   and `claude-troubleshooting-guide-expand` — codex was active when
   this snapshot was written. Tier-1 review + merge per the usual
   pattern. PRs will be on `claude-*` prefixed branches (codex used
   the brief slug as branch name on its last PR — slug, not family).
5. The 14 DIRTY PRs need their workers to do `git fetch && git merge
   origin/main && git push` (recipe on each PR). Conductor's job is
   to wait, not push — workers will resume from their quota gates.
6. The 2 UNSTABLE PRs (#1498 vuln-mgmt, #1506 cli-help-reference)
   are the chicken-egg pattern — fix-recipes posted; worker needs to
   either seed waivers or use `continue-on-error` for first run.

### Codex worker status / encouragement

The codex pre-push gate (`scripts/codex-pre-push.sh`) is **working
correctly** — verified via #1528. Workers calling
`ARGO_CLAIM_FORCE=1` for legitimate ceiling-headroom briefs is the
documented escape hatch and is fine to allow.

If codex pushes a PR on a `claude-*` branch (codex used the brief
slug verbatim, which has a `claude-` prefix), that's not a bug —
the new codex worker prompt at `PROTOCOL_WORKER_PROMPT_CODEX.md`
says "branch prefix: `codex-` unless the brief specifies otherwise"
and codex correctly inherited the brief's preferred name.

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

Last refreshed: 2026-05-13 ~02:30Z (Claude lane B at 98% weekly,
operator relaunching → next session inherits). Codex active and
producing.
