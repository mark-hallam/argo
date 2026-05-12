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

5. **Active operator directive**, if one is named in the
   BUILD_STATUS.md header's "Live directives" line. Currently:
   `conductor-prompt-deployment-path-v3.md` (drives the swarm to
   "deployable end-to-end" — overrides PROTOCOL_LOOP_PROMPT step-8
   normal replenishment until its §5 stop conditions are met).

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

Last refreshed: 2026-05-12. If this file is stale, it's the
conductor's job to refresh it during the next freshness pass.
