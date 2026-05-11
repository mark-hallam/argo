# Mission load-bearing wire — `ops-on-call` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/`); existing PagerDuty MCP connector (`crates/argo-mcp-connector-pagerduty/` — verify in first 5min); existing Slack approval roundtrip (MERGED).
**Authored against**: origin/main `805deb35` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-ops-on-call`.

> SOURCE: MISSION_BACKLOG.md Section 11 #158 — "Ops-on-call vertical." Adds a vertical for first-line incident triage: read pages, gather context, suggest runbooks, escalate to the human on-call when blocked.

## Scope

NEW `verticals/ops-on-call/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "ops-on-call"`, version `0.1.0`
   - `[role]` persona = experienced SRE who knows when to wake the human
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ pagerduty = {...}, slack = {...} }` (PD optional if connector not configured)
   - `[escalation].human_approval_required` = `["restart_service", "rollback_deploy", "page_secondary"]`
   - `[escalation].escalate_after_failures` = 1 (very conservative — fail fast to human)

2. `prompts/system.md` (≤ 200 LOC) — covers: triage protocol (acknowledge < 5min, gather metrics + recent deploys + last alert in same component, runbook lookup), explicit "when to wake the human" criteria.

3. `prompts/persona.md` (≤ 100 LOC) — SRE persona, no-bullshit tone, prioritises clarity over completeness in pages.

4. `skills/triage_page.md` — given a PD incident payload, classify (transient / runbook-applicable / human-required) + initial response.

5. `skills/find_runbook.md` — search known-runbook locations for the symptom.

6. `skills/draft_incident_update.md` — produce a status-page-shaped update for the customer-facing channel.

7. `skills/escalate_to_human.md` — fire the Slack ping with a 5-line summary + suggested next action.

8. `memory/schema.toml` — namespaces: `recent_incidents`, `runbooks_seen`, `escalation_contacts`.

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-runbook, 2 needs-escalation, 2 ambiguous-symptom.

10. `auto_approval.toml` — read-only skills auto-approved; any mutation (restart/rollback/page) requires human.

11. `inventory.json` — example runbooks for v1 demo.

This is NOT in scope: shipping PagerDuty / Datadog / Opsgenie MCP connectors (separate briefs); paging-policy editing UI (operator-only); auto-mitigation (the v1 vertical is read-only-then-escalate).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/ops-on-call/vertical.toml` (NEW)
- `verticals/ops-on-call/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/ops-on-call/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/ops-on-call/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/ops-on-call/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/ops-on-call/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/ops-on-call/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/ops-on-call/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `ops-on-call`
- [ ] `argo vertical lint ops-on-call` reports no errors
- [ ] `argo vertical show ops-on-call --json` returns valid envelope
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates ALL mutation skills behind human approval
- [ ] `escalate_after_failures = 1` (conservative escalation)
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-ops-on-call`
- PR title: `vertical: ops-on-call (mission-vertical-ops-on-call)`

## Notes for the implementer

- The "when to wake the human" criteria in system.md is the most important content — be specific (data-loss-risk, customer-facing-impact >X users, security-relevant, paging policy says secondary, etc.).
- For eval/seed.jsonl: avoid real company / customer names. Use generic "Service A returns 503 / DB connection saturated" patterns.
- The escalate_after_failures=1 is intentional — ops verticals should never silently retry the wrong thing. Document this in PR body.
- The skill `restart_service` is named but auto_approval-gated; the actual restart-command execution lives in the connector + executor, not in this vertical.
