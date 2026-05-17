# Post-incident review template

Blameless. Factual. Written within 5 business days of resolution.
Filed at `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`.

## Header

- **Incident id**: `<YYYY-MM-DD>-<slug>`
- **Severity**: P0 / P1 / P2
- **Detected at** / **Resolved at** (UTC): `<ISO-8601>` / `<ISO-8601>`
- **Duration**: `<HH:MM>` — **Scope**: tenants / connectors / regions
- **Author** / **Reviewer**: `<handle>` / `<handle>`

## What happened

Two to four sentences in plain English. No role attribution.

## Timeline (UTC)

| Time | Event |
|------|-------|
| `<HH:MM:SS>` | Detection signal fires (source). |
| `<HH:MM:SS>` | Incident declared by `<operator>`. |
| `<HH:MM:SS>` | First containment action: `<action>`. |
| `<HH:MM:SS>` | Root cause identified. |
| `<HH:MM:SS>` | Mitigation applied; verified resolved. |
| `<HH:MM:SS>` | Customer comms sent. |

## 5 whys

1. Why did `<symptom>` happen? → `<answer-1>`
2. Why did `<answer-1>` happen? → `<answer-2>`
3. Why did `<answer-2>` happen? → `<answer-3>`
4. Why did `<answer-3>` happen? → `<answer-4>`
5. Why did `<answer-4>` happen? → `<root cause>`

Stop at a structural cause. "Operator error" without the missing
safeguard is not a root cause.

## Action items

No owner + no deadline = wish.

| Item | Owner | Deadline | Tracker (`BUILD_STATUS.md` row or `MISSION_BACKLOG.md` row) |
|------|-------|----------|--------|
| `<concrete change>` | `<handle>` | `<YYYY-MM-DD>` | `<row-id>` |

## Lessons learned

Forward-looking and generalizable. One short paragraph.
