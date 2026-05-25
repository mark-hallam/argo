# Status Reporter vertical pack

The Status Reporter pack adds Patrick, the executive-status counterpart to
the Project Coordinator pack. Patrick reads delivery systems, keeps the RAID
log current, tracks milestones, and drafts weekly executive updates while
keeping every executive or stakeholder outbound behind approval.

- **Worker slug**: `status-reporter`
- **Suggested first name**: Patrick, with Paul, Pierce, Preston, and Pranav
  as operator-overridable alternatives on hire.
- **Cohort**: Project & Ops, paired with `packs/project-coordinator`.
- **Default level cap**: L3. `stakeholder-communicate` and `board-update`
  are L4-locked.

## Mission claim alignment

- **Claim 1**: teammate personas for an executive-status role.
- **Claim 3**: platform vertical packs.
- **Claim 4**: skill-loop and cross-tool synthesis.
- **Claim 5**: deterministic replay for executive-tier drafts.

## Skills

| Skill | Purpose | Cost target |
|---|---|---|
| `linear-read` | Delivery state, cycles, blockers, throughput trends | $0.003/call |
| `github-read` | PRs, commits, deployments, releases | $0.003/call |
| `slack-read` | Public-channel context only; no DMs | $0.004/call |
| `gcal-read` | Executive calendar context and report cadence | $0.001/call |
| `notion-read` | OKRs, project docs, retros, decisions | $0.005/call |
| `email-draft` | Gmail draft creation; no send | $0.006/call |
| `policy-tone` | Executive tone calibration from approved history | $0.002/call |
| `exec-summary-draft` | Weekly one-page executive summary | $0.020/call |
| `milestone-track` | Internal milestone snapshots | $0.004/call |
| `raid-maintain` | Internal RAID-log maintenance | $0.005/call |

Locked behind L4: `stakeholder-communicate` and `board-update`.

## Approval gate

The pack is conservative by default. Status reports, executive summaries,
stakeholder messages, board updates, and any external communication require
explicit operator approval. RAID-log and milestone maintenance are internal
memory updates and can run autonomously at L3.

Three negative fixtures (`sr-006`, `sr-007`, `sr-024`) validate the refusal
path for unapproved outbound or board-tier work.

## Memory schema

This pack composes the shared Project Coordinator memory contract for
`project_roster`, `milestone_calendar`, and `decision_log`, then adds:

- `stakeholder_map`
- `exec_comm_history`
- `raid_log`
- `status_report_templates`

The empty seed is intentional; production tenants populate these collections
from real Linear, GitHub, Slack, Google Calendar, Notion, and Gmail accounts.

## Eval suite

`evals/INDEX.json` declares 26 scenarios, including deterministic replay
fixtures for executive summary and RAID-log output. `evals/golden.json` holds
the golden prompts and expected outcomes used by the pack lint path.

## Templates

- `templates/exec-summary.md.tmpl`
- `templates/status-report.md.tmpl`
- `templates/raid-log.md.tmpl`

All templates include a source appendix and an explicit approval reminder.

## Install smoke

```bash
argo packs dev-link status-reporter packs/status-reporter --json
argo skill pack lint packs/status-reporter
```

The pack ships with `dry_run = true`; operators disable it only after
connector scopes, stakeholder ownership, and approval routing are validated.
