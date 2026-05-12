# 14-day build plan — SKYCOT v1

> **STATUS: ARCHIVED 2026-05-12.** The original 14-day v1 sprint
> completed in the tick-57 era. This file is retained for the v1.0
> ship-gate definition (final section) and for historical context.
> Streams A–H described below are NOT live work. Live plan:
> `BUILD_PLAN_V11.md` (backend) + `PRODUCT_ROADMAP.md` (product
> surface) + `BUILD_STATUS.md` (live ledger).

**Mission**: see [`docs/skycot/MISSION.md`](../skycot/MISSION.md).

**Deadline**: 14 working days from PR-merge of this plan.

**Method**: parallel work streams across two agent families
(Claude + Codex). Some streams are sequential by necessity (the
critical path). Most can run concurrently.

**Coordination protocol**: see
[`CONTRIBUTING_AGENT.md`](CONTRIBUTING_AGENT.md). Live status:
[`BUILD_STATUS.md`](BUILD_STATUS.md).

---

## Top-level architecture

```
                     OPERATOR / TEAMMATE SURFACES
   ┌─────────┬──────────┬────────┬────────┬────────┬──────────┐
   │  CLI    │ Web Inbox│  Email │ Slack  │ SMS/Push│  Voice   │
   │ (today) │  (later) │  (NEW) │ (NEW)  │  (NEW)  │  (NEW)   │
   └────┬────┴─────┬────┴────┬───┴────┬───┴────┬────┴────┬─────┘
        └──────────┴─────────┴────────┴────────┴─────────┘
                              │
                  ┌───────────▼────────────┐
                  │ argo-gateway + argo-   │
                  │ channels (NEW)         │ ← NotifyChannel registry
                  └───────────┬────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
   ┌──────────▼─┐   ┌─────────▼──────┐   ┌────▼──────────┐
   │argo-identity│   │  argo-core     │   │argo-orchestr. │
   │(NEW): Worker│   │  (loop, ledger,│   │(NEW: handoffs,│
   │/Persona/Role│   │   approval gate)│   │ multi-worker) │
   └──────────┬──┘   └─────────┬──────┘   └────┬──────────┘
              │                │                │
              │      ┌─────────▼──────┐         │
              │      │ argo-vertical  │         │
              │      │ (manifests +   │ ◄─── argo-genesis (NEW)
              │      │  actions)      │
              │      └─────────┬──────┘
              │                │
              │   ┌────────────┼─────────────────────────┐
              │   │ Executor dispatch (per-action)        │
              │   └────────────┼─────────────────────────┘
              │                │
   ┌──────────▼┐ ┌─────────────┼──────────┬──────────────┐
   │ Human      │ │  MCPExec    │ ComputerUse │ ApiExec    │
   │ Executor   │ │  (NEW: wraps│ Executor    │ (NEW)      │
   │ (today)    │ │  argo-mcp)  │ (NEW: CU+   │            │
   │            │ │             │ sandbox)    │            │
   └────────────┘ └─────┬───────┘ └─────┬─────┘ └────┬─────┘
                        │               │            │
                  argo-mcp servers   argo-sandbox  argo-plugin
                  (gmail, slack,     (E2B FC VMs)  (existing)
                   browser, calendar)

   ┌────────────────────────────────────────────────────────────┐
   │  argo-skill   (EXTEND: cross-tenant library + learning)    │
   │  argo-memory  (EXTEND: layered recent / index / summary)   │
   │  argo-store   (existing — sqlite ledger, sessions)         │
   └────────────────────────────────────────────────────────────┘
```

---

## Streams

Each stream is independently shippable. Each row in the table
maps to a brief in `streams/<brief-id>.md`. PR counts are
estimates; the real count emerges as briefs land.

| Brief | Stream | Owner | PRs | Depends on | Day starts |
|---|---|---|---|---|---|
| 0-ci-cleanup | Stream 0 (preflight) | (in landing PR) | 1 | — | day 0 |
| A-executor-trait | Stream A — executor abstraction | claude | ~3 | — | day 1 |
| B-computer-use-executor | Stream B — computer-use + sandbox | claude | ~5 | A | day 2 |
| C-mcp-wire | Stream C — MCP wired into agent dispatch | codex | ~2 | A | day 1 |
| C-gmail | Stream C — Gmail MCP connector | codex | ~1 | C-mcp-wire | day 3 |
| C-slack | Stream C — Slack MCP connector | codex | ~1 | C-mcp-wire | day 3 |
| C-gcal | Stream C — Google Calendar MCP connector | codex | ~1 | C-mcp-wire | day 3 |
| C-hubspot | Stream C — HubSpot MCP connector | codex | ~1 | C-mcp-wire | day 3 |
| C-notion | Stream C — Notion MCP connector | codex | ~1 | C-mcp-wire | day 3 |
| D-email | Stream D — Email approval channel (Postmark) | codex | ~2 | — | day 1 |
| D-slack-approvals | Stream D — Slack approval channel | codex | ~2 | D-email (trait shape) | day 3 |
| D-sms | Stream D — SMS approval channel (Twilio) | codex | ~1 | D-email (trait) | day 4 |
| D-push | Stream D — Push approval channel (Expo) | codex | ~1 | D-email (trait) | day 4 |
| D-webinbox | Stream D — Web inbox approval channel | codex | ~2 | D-email (trait) | day 5 |
| E-identity | Stream E — Worker / Persona / Role primitives | codex | ~5 | — | day 1 |
| F-memory-layering | Stream F — memory layers + embeddings | codex | ~4 | E (MemoryNs) | day 4 |
| G-genesis | Stream G — worker generator | claude | ~4 | A stable | day 6 |
| H-auto-approval | Stream H — auto-approval + orchestrator + skill library | claude | ~7 | A, D, E, F | day 9 |

**Total estimate**: ~42 PRs over 14 days = ~3 PRs/day. Peak day
when Stream C connectors and Stream D channels are both
parallelizable: ~8 PRs/day across the swarm.

---

## Critical path

```
A (executor trait + manifest schema)
    │
    ├──► B (computer-use executor end-to-end)
    │           │
    │           └──► H (auto-approval, orchestrator, integration demo)
    │
    └──► C-mcp-wire (MCP into agent dispatch)
                │
                └──► C-{gmail,slack,gcal,hubspot,notion} parallel
```

If A slips past day 2, B and the Stream C swarm cascade. **A is
the most important PR of the next 14 days.** Owner: Claude.
Target: merged by end of day 1.

---

## Parallel-friendly windows

- **Day 1 morning**: Streams A, D-email, E-identity, F-spike all
  start. Each has zero dependencies.
- **Day 3**: A merged → C-mcp-wire starts → 5 connectors fan
  out simultaneously (5 Codex tabs).
- **Day 4–5**: D channels fan out (5 Codex tabs).
- **Day 6**: G-genesis starts (Claude); D web inbox + F memory
  layering land mid-week.
- **Day 9**: H integration phase.
- **Day 12–14**: end-to-end demo + bug bash + eval pass.

---

## Stop conditions

A brief is **DONE** when:
- All acceptance criteria boxes ticked
- PR opened, review-approved, merged
- BUILD_STATUS.md updated to `MERGED`

The plan is **DONE** when every brief is `MERGED` AND the v1
success criteria in `MISSION.md` are demonstrated end-to-end in
a live demo recording.

---

## Risks (mitigations live in each brief)

1. **Computer-use reliability on real GUIs.** Mitigation: token
   budgets, screenshot diffs, automatic fallback to
   HumanExecutor on failure.
2. **E2B / sandbox cost.** Mitigation: VM pool reuse, idle TTL,
   per-tenant quota.
3. **Channel fan-out approval thrash.** Mitigation: idempotent
   `approval_id`, first-reply-wins.
4. **Memory namespace leakage across tenants.** Mitigation:
   `MemoryNs` always carries `tenant_id`; property test asserts
   no cross-tenant leak.
5. **Genesis output safety.** Mitigation: `argo-judge` enforces
   "money-touching action cannot be autonomous-safe"; first 24h
   of any new worker stays in `dry_run = true`.

---

## What replaces this plan

When this plan ships, archive it to `docs/build-plan/archive/`
and write a v1.x roadmap referencing the v1 success criteria. Do
not delete; the archive is part of the audit trail of what got
built and why.
