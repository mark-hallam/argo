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

## Right-now snapshot (2026-05-13 ~03:30Z) — second hand-off in 1h, codex 3-for-3

Refreshed at 98.5% weekly cap, operator relaunching Claude again
(this is the SECOND hand-off in one hour — the prior account
shipped 8 merges then ran out). Codex worker has been the
workhorse since: 3-for-3 PRs since 2026-05-09 resumption (#1528
sweeper, #1533 forge-cron, #1537 HIPAA). **A fresh session
resuming should read this whole section + SWARM_PROTOCOL.md HARD
RULES + the 5 files at the top of this doc, then start ticking.**

### Live state (origin/main)

- **HEAD**: `7b590df9` (3 MERGED in one tick: #1536 + #1537 + #1538)
- **Queue counts**:
  - TODO = **12**  (below 25 trigger; replenish next tick if no merges)
  - CLAIMED = **6**  (2 codex + 4 claude; active worker progress, no PR yet)
  - IN_REVIEW = **19**  (open PRs; most are DIRTY chain-rebase)
  - BLOCKED = **41**  (mostly dep-chain; cascade-unblocks as PRs land)
  - MERGED = **241 live + 463 archived = 704 total**

### What just landed since the previous RESUME snapshot at `36a5904d`

Eight merges in ~1 hour:
- **#1529** J12-gateway-retrain-endpoints — 4 retrain endpoints
- **#1530** mission-dashboard-planning-visibility — your operator-spec dashboard work, 4 features shipped end-to-end
- **#1461** mission-encryption-at-rest-sqlcipher — final encryption-trio piece; **cascade-unblocked byok + rotation-cli-wiring**
- **#1533** mission-forge-cron-lowest-quality (**CODEX**) — Forge proactive cron tick
- **#1534** mission-voice-route-consistency — voice now uses unified channels dispatcher
- **#1536** mission-document-voice-chat-paper-only — v1.0 voice-status doc
- **#1537** mission-hipaa-path-baa (**CODEX**) — HIPAA design + startup-check
- **#1538** mission-rotation-cli-subsystem-wiring — coordinator spawns subsystem CLIs via current_exe()

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

**Codex** (2 active claims):
1. **`claude-error-dashboard-runbook`** — codex
2. **`claude-troubleshooting-guide-expand`** — codex

**Claude** (4 active claims):
3. **`mission-enterprise-audit-full-text-search`** — pre-existing claim, stale since 2026-05-12T16:45Z (worker may have given up or be holding)
4. **`mission-byok-enterprise-tenants`** — fresh claim post sqlcipher-MERGE cascade-unblock
5. **`E2-f-hire-flow-structured-adjust`** — claude worker
6. **`E2-i-hire-flow-day1-wrap-duty`** — claude worker

### Codex performance summary (since 2026-05-09 resumption)

- **3-for-3 production PRs shipped clean.** Codex pre-push gate
  (`scripts/codex-pre-push.sh`) is working correctly.
- **Acceptable use of `ARGO_CLAIM_FORCE=1`** for legitimate ceiling-
  headroom briefs (verified on #1528 where boot.rs was at 498 LOC
  and codex stayed -3 LOC net).
- **PR branch naming**: codex uses the brief slug verbatim, so a
  `claude-*` prefixed brief produces a `claude-*` branch even
  though the worker is codex. That's per the brief, not a bug.

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

- **Claude workers** (claude-worker-loop.sh on mac-host): holding
  4 active claims. Quota state fluid; workers may release/re-claim
  multiple times as caps reset.
- **Codex** (operator-driven tab): **3-for-3 shipped**. Currently
  holding 2 claims (`error-dashboard-runbook` + `troubleshooting-
  guide-expand`). Both are doc-shaped, should ship fast.
- **This conductor session (second conductor session this hour)**:
  **at 98.5% weekly cap**; operator relaunching now → this session
  ends imminently. This is the second resume-safety hand-off in
  ~1 hour.

### What a fresh conductor (next session) should do first

1. `cd "$ARGO_REPO_ROOT" && git fetch origin main && git reset --hard origin/main`
2. Read this `RESUME.md` first 100 lines, then `BUILD_STATUS.md`
   first 90 lines (header).
3. Run the 9-step conductor tick (or arm `/loop 5m` with the prompt
   from `PROTOCOL_LOOP_PROMPT.md`). **Note: cron loop currently
   CANCELLED** (operator cancelled `178d5bd0` mid-session). Re-arm
   if you want auto-ticks, otherwise drive manually.
4. **Expect codex PRs to land soon** for
   `claude-error-dashboard-runbook` + `claude-troubleshooting-guide-
   expand`. Both are doc-shaped; should ship in ~30 min each. Tier-1
   review + merge per usual.
5. **Replenishment trigger may fire** — claude_todo=12 < 25. The
   straight-forward 3-5 brief replenishment is fine; remaining
   high-confidence MISSION_BACKLOG [NEW] items getting thin so
   default to small / doc-shaped briefs and PR-deferrals from
   recent merges if the operator hasn't paused.
6. The 14 DIRTY PRs need their workers to do `git fetch && git merge
   origin/main && git push` (recipe on each PR). Conductor's job is
   to wait, not push. **Stalest is #1447 / #1450 at 24h+** — those
   may merit recovery-brief escalation per the af500037 amendment
   (>24h with multiple fix-request rounds).
7. The 2 UNSTABLE PRs (#1498 vuln-mgmt, #1506 cli-help-reference)
   are the chicken-egg pattern; fix-recipes posted — worker needs
   to either seed waivers or use `continue-on-error` for first run.

### Productivity since 2026-05-09 codex outage

Codex came back online 2026-05-13 and immediately shipped 3 clean
PRs (#1528 + #1533 + #1537). Claude workers have shipped 8+ MERGEs
in the last ~2 hours including the entire encryption-at-rest trio
(SQLCipher + credentials-toml + key-rotation) plus the rotation-cli
wiring follow-up. The swarm is in excellent shape going into the
hand-off.

### Pending operator decisions

None blocking. The mission-dashboard-planning-visibility work the
operator requested earlier MERGED via #1530. No active questions
on conductor's plate.

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

Last refreshed: 2026-05-13 ~03:30Z (second hand-off in 1h; 98.5%
weekly cap on this account, operator relaunching). Codex 3-for-3
since resumption. 8 merges in the prior hour. HEAD `7b590df9`.
