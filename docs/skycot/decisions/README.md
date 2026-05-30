# SKYCOT Decision Register

Append-only log of **operator decisions** — the scoped commitments, scope
changes, and process authorizations Mark Hallam makes that aren't
derivable from the code or git history. This is the **source of truth**
for "what did we decide, when, why, and what did it supersede."

## Why this exists

The agent swarm's context resets between sessions. Operator decisions
made in chat (e.g. "Pitch Agent is now MVP scope", "newest-wins on
competing PRs", "Sonnet floor, no Haiku") get scattered across CLAUDE.md,
auto-memory, ledger notes, and chat history — and some get lost (the
Pitch/Genesis MVP scope-creep was never documented in the phase plan,
which misaligned the timeline). This register fixes three things:

1. **Source of truth** — one queryable place for every locked decision.
2. **Conflict-flagging** — the conductor reads this index at session
   start (like CLAUDE.md). When a new request contradicts a locked
   decision, the conductor STOPS and surfaces it:
   *"This conflicts with DEC-014 (you locked X on <date> because Y) —
   confirm you want to override?"*
3. **Timeline integrity** — scope changes are captured at decision-time,
   not painfully reconstructed later.

## How it works (ADR pattern)

- One file per decision: `DEC-NNN-short-slug.md`.
- This README is the **index** — one line per decision, newest first.
- Decisions are **append-only**. To change a decision, write a NEW one
  that `Supersedes:` the old; mark the old `Status: superseded-by DEC-MMM`.
- Never delete a decision file. Reversal is itself a decision.

## Decision lifecycle

`active` → (`superseded-by DEC-NNN` | `reversed-by DEC-NNN`)

## Conductor protocol

- Load this index at session start.
- Before acting on any operator request that touches scope, process,
  routing, or a previously-locked area: grep the register. If the
  request conflicts with an `active` decision, surface the conflict and
  get explicit override confirmation before proceeding.
- When the operator makes a new locked call, write the ADR immediately
  (same turn) + add the index line.

## When this graduates to a database

Stays as files until the file approach hurts (querying, cross-referencing,
or a product surface needs it). At that point it backs onto the existing
`argo-store` SQLite schema (additive migration, tenant-scoped, optionally
hash-chained via the audit primitive) — a `decisions` table that doubles
as a product feature ("your AI workforce's decision log"). Not before.

---

## Index (newest first)

| ID | Date | Decision | Status |
|----|------|----------|--------|
| [DEC-024](DEC-024-build-agents-opus-no-downtiering.md) | 2026-05-30 | Conductor build-agents run on Opus 4.8 — no task-based down-tiering to Sonnet (quality/drift-avoidance over speed/cost); distinct from DEC-013/014 product routing | active |
| [DEC-023](DEC-023-continuous-autonomous-priority-rank-order.md) | 2026-05-30 | Continuous autonomous execution through a fixed P1–P5 priority order (de-theatre → voice+Genesis wow → real delegation → self-improvement → enterprise governance); operating model locked = conductor + on-demand build-agents + auto-resume, swarm-daemon retired for now (auditor/evaluator on-demand) | active |
| [DEC-022](DEC-022-sprint-2-scope-full-program.md) | 2026-05-30 | Sprint-2 scope = full program: all four tracks (go-live+verify / kill-all-theatre / multi-worker orchestration [Supervisor] / broad backlog) + real voice | active |
| [DEC-021](DEC-021-voice-vendor-openai-for-launch-selectable.md) | 2026-05-30 | Voice vendor = OpenAI for launch (discrete STT `/v1/audio/transcriptions` + TTS `/v1/audio/speech`); vendor-selectable (`ARGO_VOICE_PROVIDER`) + user-selectable (`voice` workspace_settings); Realtime socket = later upgrade | active |
| [DEC-020](DEC-020-autonomous-overnight-build-out-auto-resume.md) | 2026-05-29 | Autonomous overnight build-out + auto-resume authorized: conductor + self-committing build-agents + launchd `com.skycot.buildout-resume` tick; off-switch `~/.argo/buildout-stop` | active |
| [DEC-019](DEC-019-beyond-the-demo-build-out-whole-platform.md) | 2026-05-29 | Beyond the demo — build out the whole platform, remove ALL theatre, drive to the mission; supersedes demo-prep as the active program | active |
| [DEC-018](DEC-018-gtm-agent-grant-second-hero-teammate.md) | 2026-05-29 | GTM teammate "Grant" is the 2nd real working teammate; owns inbound growth as a responsibility; `/gtm` console + `POST /api/v1/gtm/draft-followup`, truth-bounded, approval-gated | active |
| [DEC-017](DEC-017-teammate-eval-harness-synthetic-golden-companies-first.md) | 2026-05-29 | Teammate-eval harness: synthetic golden-companies first, historical replay a later seam; Portia=reference, Mariah=first synthetic | active |
| [DEC-016](DEC-016-pitch-agent-persona-portia-female.md) | 2026-05-29 | Pitch Agent persona is "Portia" (female); P-name convention | active |
| [DEC-015](DEC-015-decision-register-adr-files-first.md) | 2026-05-29 | Decision register ships as ADR markdown files first; DB only if it earns it | active |
| [DEC-014](DEC-014-model-catalog-opus-4-8-sonnet-4-6.md) | 2026-05-29 | Model catalog = Opus 4.8 (frontier, 1M) / Sonnet 4.6 (1M) / Haiku 4.5; Opus 4.8 is the fixed-policy default | active |
| [DEC-013](DEC-013-sonnet-floor-no-haiku.md) | 2026-05-29 | Tier step-down floor is Sonnet 4.6 — never Haiku; Opus 4.8 for complex | active |
| [DEC-012](DEC-012-claude-code-cli-dev-demo-routing.md) | 2026-05-29 | Dev/demo LLM routes through Claude Code subscription (claude-code-cli provider) for cost-control; production flips to anthropic API | active |
| [DEC-011](DEC-011-newest-wins-competing-prs.md) | 2026-05-28 | Competing PRs on the same surface: newest-wins, close older | active |
| [DEC-010](DEC-010-genesis-mvp-must-have.md) | 2026-05-28 | Genesis (AI-teammate onboarding studio) is MVP-must-have, not v1.x-deferred | active |
| [DEC-009](DEC-009-computer-use-mvp-must-have.md) | 2026-05-28 | Computer-use is MVP-must-have (was already macOS-first per DEC-002) | active |
| [DEC-008](DEC-008-pitch-agent-in-mvp-scope.md) | 2026-05-24 | Pitch Agent is IN v1.0 MVP scope (originally beyond-v1.0) | active |
| [DEC-007](DEC-007-no-theater-in-demo.md) | 2026-05-24 | NO THEATER in the v1.0 demo MP4 — real backend, no mock data at runtime | active |
| [DEC-006](DEC-006-web-test-doc-merge-lane.md) | 2026-05-26 | Web-only ≤200 LOC ≤5 files / test-only / doc-only PRs may admin-merge without evaluator pre-verification | active |
| [DEC-005](DEC-005-skip-clippy-evaluator-preverified.md) | 2026-05-24 | --skip-clippy authorized on conductor-pre-merge-gate for evaluator-pre-verified PRs | active |
| [DEC-004](DEC-004-vocabulary-schedule-not-duty.md) | 2026-05-23 | User-facing vocab: "Schedule" not "Duty"; code/SQL keep "duty" | active |
| [DEC-003](DEC-003-melbourne-not-sydney.md) | 2026-05-18 | SKYCOT is built in Melbourne, not Sydney — brand/marketing copy | active |
| [DEC-002](DEC-002-macos-first-computer-use.md) | 2026-05-14 | macOS is the v1 computer-use path; Linux/Windows/E2B deferred-v1.x | active |
| [DEC-001](DEC-001-enterprise-not-consumer.md) | 2026-05-14 | Enterprise focus, not consumer — connector priority + scope decisions | active |

_DEC-001..DEC-018 were backfilled 2026-05-29 from CLAUDE.md, auto-memory, and
session history (a reconstruction of decisions already made). DEC-019 onward are
captured at decision-time, as intended._
