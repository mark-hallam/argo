# Operator runbook

The canonical operator runbook is
[`docs/skycot/RUNBOOK.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
— 10 sections from install through skill review + handoff + audit.
This page is the navigation overview.

## When to use this runbook

- You've installed Argo and want a steady-state operator playbook.
- You're investigating a stuck worker or a missed approval.
- You need the `argo worker actions` command catalog.

## Sections in the canonical runbook

| Section | Covers |
|---|---|
| Install | Fresh-machine setup |
| Bootstrap | Workspace init + `argo doctor` |
| Genesis | Charter → vertical generation |
| Approvals | Channel setup + steady-state response |
| First action | End-to-end approve / execute |
| Skill learning | Successful trajectory → SkillCandidate |
| Handoff | Worker-to-worker delegation |
| Audit | Chain query + replay |
| Marketplace | Vertical + skill publish / pull |
| Next steps | Where the platform is heading |

## Day-2 operator checklist

```bash
# Health
argo doctor --json
argo doctor channels
argo doctor watch --interval 60s

# Pending approval queue
argo worker actions list --state pending --json

# Spend
argo budget show <worker-id> --json
argo budget recent --json
argo budget anomalies --json
argo budget by-skill --top 10 --json

# Audit
argo audit query --decision approved --since 1d
argo audit replay <session-id>
argo audit report --since 7d --format markdown --output weekly.md
```

## Incident response

If you suspect a worker has done something it shouldn't have:

1. `argo worker pause <id>` — pause the worker (no new turns).
2. `argo worker actions cancel <id>` — cancel any pending /
   approved actions.
3. `argo audit replay <session-id>` — walk the chain.
4. `argo recover <session-id>` — get LLM-assisted post-mortem
   suggestions (advisory only).
5. Once root cause is clear: `argo worker resume <id>`.

If the worker was wrong about a skill:

```bash
argo skill quality-recompute <name>
argo skill review reject <id>
argo feedback down <skill-id>
```

The negative feedback flows through `should_propose_with_feedback`
so the next proposal of that skill blacklists.

## Compliance and redaction

```bash
argo audit export --since 90d --format ndjson --gdpr-redact-emails \
  --bundle-out compliance/2026-Q1.tar.gz
argo gdpr delete --subject user@example.com --confirm
```

GDPR delete redacts (not deletes) across 6 surfaces — memory,
audit, skills, channels, workers, sessions — preserving audit
chain integrity.

## Backup + restore

```bash
argo backup create --output workspace.tar.gz
argo backup verify workspace.tar.gz
argo restore workspace.tar.gz
```

Backups exclude credentials by default. Use `--include-credentials`
only when restoring to a trusted environment.

## Next

- [Operations → Approvals and audit](./approvals-and-audit.md)
- [Operations → Failure modes](./failure-modes.md)
