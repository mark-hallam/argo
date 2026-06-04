# DEC-034 — Paying-ready 1-week sprint + conductor-driven run-mode (no cron)

- **Date:** 2026-06-04
- **Status:** active
- **Supersedes:** DEC-033 (overnight-build-toward-funding-meeting-Thursday) — the investor
  prototype engagement is DROPPED (operator: the cost is more than budgeted; pursue
  organic traction instead). DEC-032 (deployment-readiness) folds into WS5 of this sprint.

## Context

The operator assessed SKYCOT as "more an initial POC demo than an actual MVP" and dropped
the paid investor prototype path in favour of building to paying-ready in-house, with about a week
of build time available. Goal: get real users testing, a
waitlist, enterprise EOIs/MOUs, and a product the operator would put a paying customer on.

## Decision

### 1. Scope (operator-locked)
One-week sprint: **POC → fully-public self-serve SaaS + an enterprise tier.** Wedge = the
**platform loop** (stranger signs up → gets their OWN empty workspace → Genesis hires a real
teammate → it does real work → approval → audited → can pay). Six workstreams: WS1 real
front-door/per-user-tenant, WS2 the loop in a fresh tenant, WS3 de-mock the SPA, WS4 self-serve
billing + enterprise tier, WS5 go-public on AWS, WS6 funnel (landing + waitlist + enterprise EOI).
Plan: `docs/skycot/SPRINT-paying-ready-2026-06-04.md`; evidence backlog:
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md`. Infra: AWS (ap-southeast-2, AWS credits),
skycot.com.

### 2. Run-mode — "I drive it (no cron)" (operator-locked)
The operator chose conductor-driven over the automated cron tick. Therefore:
- `skycot-conductor-20min` and `skycot-evaluator-hourly` scheduled tasks **STAY DISABLED** —
  do NOT arm them (avoids the interactive-vs-cron ledger-collision).
- The **worker-supervisor swarm builds 24/7** (launchd `com.skycot.worker-supervisor`).
- The **conductor = an interactive Claude `/loop` session** that drives: merges, queue
  replenishment, ledger-truthing, a **twice-daily live stranger-signup quality gate**, and an
  **adversarial multi-agent review on the 3 riskiest landings only** — (a) tenant isolation,
  (b) billing, (c) the public perimeter (AWS IaC / prod secrets / anon-abuse + LLM-cost caps).
- Overnight / hands-off = operator invokes `/loop`; the conductor self-paces via ScheduleWakeup
  and posts a swarm heartbeat each tick.
- After Friday: collapse to one account on the Mac.

### 3. Conductor disciplines (operator-directed this session)
- **TODO queue stays sprint-only.** Only `sprint-*` briefs (from the paying-ready roadmap) are
  authored/queued this week; old-roadmap briefs (demo-MP4 / UX-audit / post-MVP) are NOT re-queued.
- **Off-sprint / superseded open PRs are closed on sight** (with a recoverable reason), not merged.
- **PR backlog (> ~6 open) → drain it:** either fast-merge the safe ones directly or spawn a
  parallel triage/review workflow (read-only reviewers → drain plan → conductor executes merges).
- **Every queued brief is verified against current `main` and dated before a worker can claim it**
  (no stale/incorrect specs; freshness header re-stamped on re-verification).
- **Source-of-truth ledgers kept in-band:** `BUILD_STATUS.md` (every tick), `FEATURE_MATRIX.md`
  + its generated `feature-matrix.html` (`python3 scripts/render-feature-matrix.py`, same commit
  as a shipped capability), and this decision register (a locked call → an ADR + index line in the
  same turn). Brief acceptance criteria now include the FEATURE_MATRIX row update.

## Consequences
- A fresh conductor session MUST read this before acting: do not arm the cron, keep the queue
  sprint-only, drain PR backlogs, verify briefs against main.
- The investor deliverables (DEC-033) are not pursued; effort goes to the 6 workstreams.

## Evidence
Operator chat 2026-06-04 (sprint kickoff + AskUserQuestion answers: "SaaS self-serve + enterprise
tier", "the platform loop", "Fully public", "I drive it (no cron)"); the PR-backlog-drain directive;
the close-off-sprint-PRs directive; the brief-freshness directive. Memory:
`session_resume_2026_06_04_sprint_day0.md`.
