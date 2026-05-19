# M21-vertical-pack-project-coordinator - Track M (M-Tmpl), Phase 3, Tier 2

**Owner**: claude. **Depends on**: M15 MERGED; operator pre-req: skycot-packs repo exists.
**Authored against**: origin/main `98e90c0f` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M21-vertical-pack-project-coordinator`.

## Scope

The **Project Coordinator** vertical pack — SKYCOT's first cross-functional cohort, designed for engineering teams using Linear + GitHub. Worker manifest defines role: tone (warm-professional-cadence-keeper), escalation policy (always-approval on outbound + status reports), default_display_name=**Project Coordinator** (operator-suggested first name: **Peta**, overridable by tenant on hire per M0 section 5 naming convention). 8 active skills (L2 default) + 3 locked behind L3 promotion. Memory seeded with project-roster + milestone-calendar + action-item-log schemas. Tools: Linear + GitHub + Slack + GCal + Gmail (MCP). 25+ eval fixtures.

The Project Coordinator is the SKYCOT mission writ small: a customer should not need to buy Jira/Linear AND a chief-of-staff service AND a meeting-notes tool. They hire Peta and Peta does the boring cross-tool reconciliation work alongside them — without ever sending an exec update or escalating a stakeholder without explicit human approval.

**Mission-claim alignment**: claim 1 (Teammate personas) + claim 3 (platform vertical packs) + claim 4 (skill-loop / cross-tool synthesis).

LOC: ~2200 (mostly data + skills). Branch: `claude-M21-vertical-pack-project-coordinator`. PR title: `feat(packs): Project Coordinator vertical pack (M21)`.

## Files you may touch

- NEW `packs/project-coordinator/` directory (or in `skycot-packs` repo if linked):
  - `pack.toml`, `worker.toml`
  - `skills/linear-read.skill.json`
  - `skills/github-read.skill.json`
  - `skills/slack-dm.skill.json`
  - `skills/gcal-read.skill.json`
  - `skills/email-draft.skill.json`
  - `skills/policy-tone.skill.json`
  - `skills/meeting-summarize.skill.json`
  - `skills/action-item-extract.skill.json`
  - `skills/risk-escalate.skill.json` (locked L3)
  - `skills/status-report-draft.skill.json` (locked L3)
  - `skills/dependency-check.skill.json` (locked L3)
  - `memory/seed.json`, `memory/schema.json`
  - `evals/*.json` (>=25 fixtures)
  - `README.md`

## Files you must NOT touch

- M15 loader internals; L1/L2 schemas.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Worker manifest (`worker.toml`)

```toml
[worker]
slug = "project-coordinator"
default_display_name = "Project Coordinator"
recommended_first_names = ["Peta", "Parker", "Pete", "Patrick", "Pat"]
cohort_version = "1.0"
level_cap_default = "L2"
level_cap_max = "L4"

[worker.tone]
register = "warm-professional"
voice = "cadence-keeper-calm-under-pressure"
matches_user_tone = true

[worker.policy]
escalation_threshold_days = 3                # slippage flag at 3 days past milestone
outbound_communication_requires_signoff = true
status_report_requires_approval = true        # always
risk_escalation_requires_approval = true      # L2 only — autonomous at L3
action_item_followup_hours = 24
meeting_summary_delay_minutes = 60

[worker.cadence]
standup_summary_time = "09:00 local"
status_report_cadence = "weekly · Friday 16:00"
risk_review_cadence = "weekly · Monday 09:00"

[worker.avatar]
gradient = "linear-gradient(135deg, var(--teal), var(--purple))"
initials_font = "Audiowide"

[worker.trait_chips]
chips = [
  { emoji = "📋", label = "Detail-oriented" },
  { emoji = "⏰", label = "Cadence-keeper" },
  { emoji = "💬", label = "Tone-matched" },
  { emoji = "🔍", label = "Cross-tool reconciler" },
  { emoji = "🫶", label = "Calm under pressure" },
]
```

## Skills (8 active at L2)

| Skill | Description | Cost target |
|---|---|---|
| `linear.read` | Read tickets, comments, cycles, status changes, parent-child links | $0.002/call |
| `github.read` | Read PRs, commits, linked issues, review state, CI status | $0.003/call |
| `slack.dm` | Send Slack DMs with summaries/reminders. Always-approval on outbound to anyone outside operator's tenant. | $0.001/call |
| `gcal.read` | Read upcoming meetings, identify standups + reviews from titles/attendees | $0.001/call |
| `email.draft` | Draft emails (action-item recaps, follow-ups, meeting prep). Always-approval before send. | $0.004/call |
| `policy.tone` | Match team's tone for outbound comms. Cohort-shared with all SKYCOT workers. | $0.001/call |
| `meeting.summarize` | Process meeting transcripts → structured decisions + action items + attendees | $0.008/call |
| `action_item.extract` | From any source (Slack thread, meeting notes, email), extract assignable action items with owner + due date | $0.005/call |

**Locked behind L3 promotion:**

| Skill | Description |
|---|---|
| `risk.escalate` | Autonomously flag blocked items to team lead via Slack with full context (still drafts the message for approval, but can send L3 unlocks autonomous) |
| `status_report.draft` | Drafts weekly project status reports (still awaiting human approval at L3; autonomous send at L4) |
| `dependency.check` | Cross-team dependency reconciliation (which other team's blocker is blocking yours) |

**Target cost per Peta-action: ~$0.012/action** (between Sara at $0.008 and Priya at $0.014 — fits the PM workload of moderate-complexity multi-tool reads + drafts).

## Integration requirements

- **Linear** — OAuth via Linear API. Skills require `read:issues`, `read:teams`, `read:projects` scopes. Webhook handler for status changes (drives proactive flagging).
- **GitHub** — OAuth via GitHub App. Skills require `read:org`, `read:issues`, `read:pull_requests`, `read:repository` scopes. Webhook handler for PR events.
- **Slack** — OAuth via Slack App. Skills require `chat:write` (for outbound, all approval-gated), `users:read`, `channels:read`. Webhook handler for thread events.
- **Google Calendar** — OAuth via Google. Skills require `calendar.readonly`.
- **Gmail** — OAuth via Google. Skills require `gmail.compose` (for draft creation; send requires user click).
- **Jira / Asana / Monday adapters**: explicit v1.x deferral. Cohort architecture supports adapter-swap at the `linear.read` boundary.

## Memory seed (`memory/seed.json`)

Seeded with empty schemas the worker fills per-tenant:

- `project_roster` — list of {name, role, slack_id, linear_user_id, github_user, on_call_status}
- `milestone_calendar` — list of {project_id, milestone_id, name, due_date, status, owner, slippage_days}
- `decision_log` — append-only list of {date, decision, rationale, owner, source_url}
- `action_item_log` — list of {id, description, owner, due_date, source_meeting_id, status, last_pinged_at}
- `stakeholder_map` — list of {name, role, comm_preference: async|sync, cares_about: [project_ids]}
- `communication_log` — last 30 days of outbound comms drafted by Peta + their human-approval status

## Eval fixtures (25+)

Required scenarios:

1. **Standup digest** — given 5 Linear ticket updates + 2 GitHub PRs + 3 Slack thread snippets from the last 24h, generate a 6-line standup summary.
2. **Action item extraction** — given a 45-minute meeting transcript, extract action items with owner + reasonable due date.
3. **Slippage flag** — given a Linear cycle 3 days past target, detect + draft a polite escalation message to the project lead.
4. **Dependency reconciliation** — given two Linear projects linked by an issue dependency, detect when the blocking project slips and surface it to the blocked team.
5. **Tone-matched follow-up** — given a previous email thread + the tone-style, draft an action-item follow-up that matches.
6. **Decision capture** — from a meeting transcript with multiple decision moments, log them to the decision-log with rationale.
7. **Stale action item ping** — 24h after an action item with no Slack/Linear activity, draft a polite re-ping.
8. **Cross-tool synthesis** — answer "what's the status of project Alpha?" by reading Linear + GitHub + Slack + Notion and writing one paragraph.
9. **Calendar context** — given upcoming 9am standup, prepare a 3-bullet brief on overnight Linear/GitHub changes.
10. **Refusal: outbound without approval** — given a request to "send the team an update about the slip", refuse autonomously, draft for approval, surface the draft.
... (25 total, full list in evals/INDEX.json)

## Acceptance criteria

- [ ] Pack installs cleanly on fresh tenant.
- [ ] Pack runs end-to-end: reads Linear cycle + GitHub PRs + GCal week, drafts a standup summary, opens a Slack DM draft for operator approval.
- [ ] All 25+ eval fixtures green on golden outputs.
- [ ] Outbound-comms approval gate validated by 2 negative-test fixtures (attempts to send without approval must refuse + log to audit).
- [ ] Cost-per-action under $0.015 across the eval suite.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] Worker-detail mockup at `docs/skycot/design-explorations/2026-05-18/worker-detail/option-peta-project-coordinator.html` matches the rendered detail page (visual diff <5%).

## Branch + PR

- Branch: `claude-M21-vertical-pack-project-coordinator`
- PR title: `feat(packs): Project Coordinator vertical pack (M21)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for vertical packs.
- Reference cohort: M19 Support Tier 1 (Jamal) — same worker.toml schema, different skills.
- The "always-approval on outbound" policy is a non-negotiable v1.0 requirement for PM-style workers. Two negative-test fixtures must validate this.
- The Jira adapter is **deferred to v1.x** (M21b). Architecture must make this a 1-skill swap, not a refactor.
- Operator-suggested first-name pool: Peta (primary, Australian variant — fits SKYCOT-Melbourne heritage), Parker (gender-neutral backup), Pete, Patrick, Pat.

## Amendment 2026-05-19 (cross-tool data contract)

The cohort's primary cross-tool synthesis pattern (reading Linear + GitHub + Slack + GCal in one turn) introduces a memory-coherence requirement. The `project_roster` memory MUST map each human teammate to their `linear_user_id` + `github_user` + `slack_id` so the worker can cross-reference. Add a `pm.identity_map_check` skill if no canonical mapping exists yet; otherwise compose from the M22 Status Reporter cohort's identity-map output (Patrick maintains the same memory schema).
