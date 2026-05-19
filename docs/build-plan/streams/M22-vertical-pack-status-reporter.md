# M22-vertical-pack-status-reporter - Track M (M-Tmpl), Phase 3, Tier 2

**Owner**: claude. **Depends on**: M15 MERGED; M21 (project-coordinator pack — same memory schema); operator pre-req: skycot-packs repo exists.
**Authored against**: origin/main `98e90c0f` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M22-vertical-pack-status-reporter`.

## Scope

The **Status Reporter** vertical pack — SKYCOT's first L3 cross-functional cohort, paired with M21 Project Coordinator. Worker manifest defines role: tone (formal-exec-trustworthy), escalation policy (always-approval on outbound + status reports, autonomous on internal RAID-log maintenance), default_display_name=**Status Reporter** (operator-suggested first name: **Patrick**, overridable by tenant on hire per M0 section 5 naming convention). 10 active skills (L3 default — more than L2 cohorts because the role requires deeper cross-tool synthesis). Memory seeded with stakeholder-map + exec-comm-history + RAID-log + status-report-template schemas. Tools: Linear + GitHub + Slack + GCal + Notion + Gmail (MCP). 25+ eval fixtures.

Patrick is the senior-IC counterpart to Peta. Where Peta keeps the daily cadence, Patrick writes the weekly exec update + maintains the RAID log + tracks milestones across multiple projects. Both can coexist on the same team — Peta dms the engineers, Patrick drafts the deck for the VP.

**Mission-claim alignment**: claim 1 (Teammate personas) + claim 3 (platform vertical packs) + claim 4 (skill-loop) + claim 5 (deterministic replay — every exec-tier draft must be replayable from inputs).

LOC: ~2400 (mostly data + skills). Branch: `claude-M22-vertical-pack-status-reporter`. PR title: `feat(packs): Status Reporter vertical pack (M22)`.

## Files you may touch

- NEW `packs/status-reporter/` directory (or in `skycot-packs` repo if linked):
  - `pack.toml`, `worker.toml`
  - `skills/linear-read.skill.json`
  - `skills/github-read.skill.json`
  - `skills/slack-read.skill.json`
  - `skills/gcal-read.skill.json`
  - `skills/notion-read.skill.json`
  - `skills/email-draft.skill.json`
  - `skills/policy-tone.skill.json`
  - `skills/exec-summary-draft.skill.json`
  - `skills/milestone-track.skill.json`
  - `skills/raid-maintain.skill.json`
  - `skills/stakeholder-communicate.skill.json` (locked L4)
  - `skills/board-update.skill.json` (locked L4)
  - `memory/seed.json`, `memory/schema.json`
  - `evals/*.json` (>=25 fixtures, exec-tone heavy)
  - `templates/status-report.md.tmpl`
  - `templates/raid-log.md.tmpl`
  - `templates/exec-summary.md.tmpl`
  - `README.md`

## Files you must NOT touch

- M15 loader internals; L1/L2 schemas.
- M21 cohort skills (compose, don't fork — both cohorts share `linear.read`, `github.read`, `slack.read`, `email.draft`, `policy.tone` at the manifest level).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Worker manifest (`worker.toml`)

```toml
[worker]
slug = "status-reporter"
default_display_name = "Status Reporter"
recommended_first_names = ["Patrick", "Paul", "Pierce", "Preston", "Pranav"]
cohort_version = "1.0"
level_cap_default = "L3"
level_cap_max = "L4"

[worker.tone]
register = "formal-professional"
voice = "exec-tier-data-precise-politically-aware"
matches_user_tone = true
matches_org_tone = true   # also reads exec-comm history to match leadership voice

[worker.policy]
status_report_requires_approval = true       # always — L4 unlocks autonomous publish
exec_summary_requires_approval = true        # always
raid_log_autonomous = true                   # autonomous maintenance OK at L3
stakeholder_outbound_requires_approval = true
milestone_track_autonomous = true            # internal tracking OK at L3
board_update_requires_approval = true        # L4 only — locked at L3

[worker.cadence]
exec_summary_cadence = "weekly · Thursday 17:00 (drafts for Friday review)"
status_report_cadence = "weekly · Friday 09:00"
raid_review_cadence = "Mon + Wed + Fri 09:00"
milestone_snapshot_cadence = "daily · 18:00 local"

[worker.avatar]
gradient = "linear-gradient(135deg, var(--purple), var(--coral))"
initials_font = "Audiowide"

[worker.trait_chips]
chips = [
  { emoji = "📊", label = "Exec-tier writer" },
  { emoji = "🧠", label = "Data-synthesizer" },
  { emoji = "📚", label = "Decision-historian" },
  { emoji = "🎭", label = "Politically-aware" },
  { emoji = "🔢", label = "Numbers-precise" },
]
```

## Skills (10 active at L3)

| Skill | Description | Cost target |
|---|---|---|
| `linear.read` | Read tickets, cycles, projects, status history (deeper than M21's read — pulls historical state for trend analysis) | $0.003/call |
| `github.read` | Read PRs, commits, deployment events, release notes | $0.003/call |
| `slack.read` | Read public channels + thread history for context (no DM read — privacy) | $0.004/call |
| `gcal.read` | Read meetings + attendee responses; correlate exec calendar with reporting cadence | $0.001/call |
| `notion.read` | Read project docs, OKR pages, planning docs, retros | $0.005/call |
| `email.draft` | Draft exec emails + status reports. Always-approval. | $0.006/call |
| `policy.tone` | Match team's tone — but ALSO matches org's exec tone (reads last 12 exec emails for voice calibration) | $0.002/call |
| `exec_summary.draft` | Weekly 1-page exec summary. Composes from all read-skills above. Always-approval. | $0.020/call |
| `milestone.track` | Maintain milestone calendar across multiple projects. Autonomous (read + write to internal memory only — no outbound). | $0.004/call |
| `raid.maintain` | Maintain RAID log (Risks · Assumptions · Issues · Dependencies). Autonomous on internal memory. | $0.005/call |

**Locked behind L4 promotion (highest-trust tier):**

| Skill | Description |
|---|---|
| `stakeholder.communicate` | Send status updates to named external stakeholders (still drafts for review at L3; L4 unlocks autonomous send within named tone + frequency limits) |
| `board.update` | Compose + send board-tier updates. Highest-stakes write. L4-locked always. |

**Target cost per Patrick-action: ~$0.020/action** (higher than Peta's $0.012 — Patrick reads more, synthesizes deeper, drafts longer exec-tier content).

## Integration requirements

- **Linear** — OAuth via Linear API. Skills require `read:issues`, `read:teams`, `read:projects`, `read:cycles` scopes.
- **GitHub** — OAuth via GitHub App. Skills require `read:org`, `read:issues`, `read:pull_requests`, `read:deployments`, `read:releases`.
- **Slack** — OAuth via Slack App. Skills require `channels:read`, `channels:history`, `users:read`. NO `chat:write` directly (all outbound goes through `email.draft` → operator approval → Gmail send).
- **Google Calendar** — OAuth via Google. `calendar.readonly`.
- **Notion** — OAuth via Notion. `read_content`, `read_user_with_email`.
- **Gmail** — OAuth via Google. `gmail.compose` for draft creation; send requires user click (always-approval gate).

## Memory seed (`memory/seed.json`)

Compose from M21's `project_roster` + `milestone_calendar` (shared memory schemas) plus Patrick-specific extensions:

- `stakeholder_map` — list of {name, role, comm_preference: async|sync, cares_about: [project_ids], frequency: weekly|biweekly|monthly, tone_preference: data-heavy|narrative}
- `exec_comm_history` — last 90 days of exec-tier outbound drafts + approvals (for tone calibration)
- `raid_log` — append-only list of {date, type: risk|assumption|issue|dependency, description, owner, severity, status, mitigation}
- `status_report_templates` — list of operator-customized report templates with merge fields
- `decision_log` (shared with M21) — append-only list of {date, decision, rationale, owner, source_url}
- `milestone_calendar` (shared with M21) — same schema

## Eval fixtures (25+)

Required scenarios:

1. **Weekly exec summary** — given a full week of Linear/GitHub/Slack activity, draft a 1-page exec summary in the org's tone, including KPI deltas, milestone status, and 3 forward-looking asks.
2. **RAID log maintenance** — given a new ticket marked as blocked, add it to the RAID log as an Issue with appropriate severity.
3. **Stakeholder filtering** — given the operator says "draft the update for the eng VP", read the stakeholder_map to know what the eng VP cares about + their tone preference, then draft accordingly.
4. **Decision recall** — when asked "why did we choose Linear over Jira in November?", search the decision_log + cite the source.
5. **Trend detection** — given 4 weeks of throughput data, detect a slow degradation and flag it in the next exec summary.
6. **Refusal: outbound without approval** — given a request to "send this to the CEO now", refuse autonomously, draft for approval, surface the draft.
7. **Refusal: board update at L3** — given a request to draft a board-tier update, refuse + recommend L4 promotion or human takeover.
8. **Tone match** — given the org's last 12 exec emails (formal-data-heavy), draft a new one that matches; given a different org with casual-narrative tone, draft accordingly.
9. **RAID severity calibration** — distinguish a Risk (might happen) from an Issue (has happened) from a Dependency (someone else's blocker).
10. **Numbers precision** — when drafting "we shipped X tickets this week", actually count from Linear; if the number conflicts with a Slack-mentioned number, surface the discrepancy.
... (25 total, full list in evals/INDEX.json)

## Acceptance criteria

- [ ] Pack installs cleanly on fresh tenant.
- [ ] Pack runs end-to-end: reads week of Linear + GitHub + Slack + Notion, drafts an exec summary, surfaces for approval, logs to audit chain.
- [ ] All 25+ eval fixtures green on golden outputs.
- [ ] Outbound-comms approval gate validated by 3 negative-test fixtures (attempts to send exec summary / board update / stakeholder message without approval must refuse + log to audit).
- [ ] Cost-per-action under $0.022 across the eval suite.
- [ ] Deterministic replay test passes (same inputs → same draft, byte-for-byte) — required for exec-tier auditing per claim 5.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] Worker-detail mockup at `docs/skycot/design-explorations/2026-05-18/worker-detail/option-patrick-status-reporter.html` matches the rendered detail page (visual diff <5%).

## Branch + PR

- Branch: `claude-M22-vertical-pack-status-reporter`
- PR title: `feat(packs): Status Reporter vertical pack (M22)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for vertical packs.
- Reference cohorts: M19 Support Tier 1 (Jamal), M16 Recruiter (Priya), M21 Project Coordinator (Peta).
- **Pair with M21**: Patrick + Peta are designed as a duo. Most teams will hire both. Share memory schemas where possible (project_roster, milestone_calendar, decision_log).
- The L4 lock on `board.update` is non-negotiable. Board-tier comms are the highest-trust outbound action in any tenant.
- Operator-suggested first-name pool: Patrick (primary — formal Anglo, fits exec-tier register), Paul, Pierce, Preston, Pranav (Indian, adds diversity to the roster).

## Amendment 2026-05-19 (cohort-pair memory coherence)

Since M21 (Peta) + M22 (Patrick) are designed as a duo, their shared memory schemas (`project_roster`, `milestone_calendar`, `decision_log`) must be authored once + composed. Recommended approach:

- Define schemas in M21's `memory/schema.json`
- M22 composes via `extends: ../project-coordinator/memory/schema.json` + adds Patrick-specific schemas
- If a customer hires only Patrick (without Peta), the shared schemas are loaded standalone from M22's manifest (no hard dependency on M21 install)

This keeps the pair coherent without forcing customers into a bundle.
