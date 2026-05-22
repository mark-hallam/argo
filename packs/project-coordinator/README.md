# Project Coordinator vertical pack

SKYCOT's first cross-functional cohort. Reads Linear + GitHub + Slack +
Google Calendar, drafts cadence summaries and action-item recaps, and
**never** sends outbound communications without explicit human approval.

- **Worker slug**: `project-coordinator`
- **Suggested first name**: Peta (Australian variant — SKYCOT is built
  in Melbourne). Operator-overridable on hire.
- **Cohort**: Project & Ops (with M22 Status Reporter / Patrick).
- **Default level cap**: L2 (8 active skills). L3 unlocks `risk.escalate`,
  `status_report.draft`, `dependency.check` — still always-approval at
  L3. L4 unlocks autonomous outbound send (operator opt-in).

## Mission claim alignment

- **Claim 1** — Teammate personas (Peta is the first PM-style teammate).
- **Claim 3** — Platform vertical packs (cross-functional, not vertical).
- **Claim 4** — Skill-loop / cross-tool synthesis (Linear + GitHub +
  Slack + GCal in one turn).

## Why this pack matters

A customer should not need to buy Jira/Linear AND a chief-of-staff
service AND a meeting-notes tool. They hire Peta and Peta does the
cross-tool reconciliation work alongside them.

## Skills (8 active at L2)

| Skill | Purpose | Cost target |
|---|---|---|
| `linear-read` | Tickets, comments, cycles, parent-child links | $0.002/call |
| `github-read` | PRs, commits, linked issues, review + CI state | $0.003/call |
| `slack-dm` | Draft DMs; **always-approval before send** | $0.001/call |
| `gcal-read` | Upcoming meetings; classify standup/review/1:1 | $0.001/call |
| `email-draft` | Gmail drafts; **never sends** (operator clicks Send) | $0.004/call |
| `policy-tone` | Cohort-shared tone matching | $0.001/call |
| `meeting-summarize` | Transcript → decisions + action items + roster | $0.008/call |
| `action-item-extract` | Slack/meeting/email → assignable items | $0.005/call |

Locked behind L3 promotion: `risk-escalate`, `status-report-draft`,
`dependency-check`.

**Target cost per Peta-action: ~$0.012** (between Sara at $0.008 and
Priya at $0.014). Budget ceiling per action: $0.015.

## Approval gate (non-negotiable)

`outbound_communication_requires_signoff = true` in `worker.toml`. All
outbound (Slack DMs, emails, status reports, risk callouts) — even to
teammates inside the operator's tenant — refuses send without an
approval record and logs the refusal to audit.

Two negative-test eval fixtures (`pc-010`, `pc-011`) validate the gate.

## Memory schema

Six collections seeded empty; populated per-tenant on first run:

- `project_roster` — identity map (Linear + GitHub + Slack ids per
  teammate). **Required** for cross-tool synthesis.
- `milestone_calendar` — milestones + slip status.
- `decision_log` — append-only decisions captured from meetings/threads.
- `action_item_log` — open action items.
- `stakeholder_map` — who cares about what; sync vs async preference.
- `communication_log` — 30-day rolling history of outbound drafts +
  approval status (used for tone learning + audit).

## Integrations

- **Linear** — OAuth, read-only (`read:issues`, `read:teams`, `read:projects`).
- **GitHub** — GitHub App, read-only (`read:org`, `read:issues`,
  `read:pull_requests`, `read:repository`).
- **Slack** — OAuth (`chat:write` is approval-gated, plus `users:read`,
  `channels:read`).
- **Google Calendar** — OAuth (`calendar.readonly`).
- **Gmail** — OAuth (`gmail.compose` only; send stays with the operator).

Jira / Asana / Monday adapters are **deferred to v1.x** (M21b). The
cohort architecture makes this a 1-skill swap at the `linear-read`
boundary, not a refactor.

## Eval suite

`evals/golden.json` has 26 fixtures across the 11 skills. `evals/INDEX.json`
lists scenarios and tags the negative tests.

## Acceptance criteria (from brief)

- [x] Pack manifest + worker manifest
- [x] 8 active skills (L2) + 3 locked skills (L3)
- [x] Memory schema + empty seed
- [x] 25+ eval fixtures (26 shipped)
- [x] Two negative tests for outbound-approval gate
- [x] No file > 500 LOC (largest file is `evals/golden.json` at ~30 lines)
- [x] Cohort matches worker-detail mockup at
  `docs/skycot/design-explorations/2026-05-18/worker-detail/option-peta-project-coordinator.html`

End-to-end install/run testing happens when the M15 pack loader picks
this up at runtime — that's a separate brief.

## Notes for hiring operators

- Worker installs in `dry_run = true`. Disable dry-run after validating
  Linear OAuth scopes, identity-map coverage, and the outbound-approval
  routing surface.
- Trait chips in `worker.toml` drive the worker-detail card.
- Recommended first names: Peta, Parker, Pete, Patrick, Pat.
