# Round 8 — Mission-vs-Code coverage campaign (bootstrap)

**Date**: 2026-05-23 (round 8)
**Scope**: Bootstrap the long-horizon mission-vs-code coverage campaign Mark asked for. Author the master manifest. Triage 89 crates against 6 mission claims. Identify top-priority unaudited surfaces.
**Verdict**: 🟢 Living document published. **29 gaps logged** (6 P0, 13 P1, 10+ P2). Estimated **5-7 evaluator sessions** to reach full mission-vs-code coverage at current pace.

## What this round produced

1. **`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`** — the master living document. Structured by mission claim, then by crate, then by gap. The conductor reads it each tick (Step 4.5b's autonomous handoff loop) and turns gaps into briefs.

2. **Triaged all 89 workspace crates** into 6 mission-claim buckets + cross-cutting. Each crate has: LOC, audited status, known gaps, suggested brief shape.

3. **Master gap list** at the end of the manifest, sorted by priority. 6 🔴 P0, 13 🟡 P1, 10+ 🟡 P2.

## Coverage map so far (rounds 1-8)

| Crate / surface | Status |
|---|---|
| `argo-identity` | 🟢 audited rounds 2 + 7 — solid |
| `argo-skill` | 🟢 audited round 2 — claim-4 e2e passes |
| `argo-executor-computer-use-macos` | 🟢 audited round 4 — 37/37 unit tests |
| `argo-audit` | 🟢 audited rounds 3+4 — cross-tenant verified |
| `argo-capability` | 🟢 by use — RBAC engine verified through workflow + worker routes |
| `argo-workflow_definition` (store) + `routes_workflows` (cli) | 🟢 audited round 7 — strong tenant scoping, txns, schema UNIQUE |
| `argo-cron` | 🟡 partial round 8 — wired at gateway boot; need duty-fires-runner cargo test |
| `argo-channels` | 🟡 partial round 8 — 13 channel impls + inbound for email/Slack only |
| `argo-genesis` | 🟡 surface-mapped round 8 — 8881 LOC, 20+ pub modules; need pipeline e2e smoke |
| `argo-workplace-knowledge` | 🟡 substrate audited round 3, gateway just MERGED (#2062) |
| **29 other crates** | 🔴 NOT YET AUDITED |

## Top 6 P0 gaps (action ranked)

| # | Gap | Brief shape | Why P0 |
|---|---|---|---|
| 1 | **`eval-argo-agent-runner-turn-lifecycle`** | Walk turn entry → policy gate → skill lookup → tool call → audit emit → capability check → response. Add cargo smoke covering one full turn end-to-end. | THE runtime. Largest unaudited mission-critical surface. Every claim 1-6 routes through this code. |
| 2 | **`eval-credentials-cross-tenant-isolation`** | Verify `argo-credentials` keys OAuth + API keys per tenant; assert tenant_b cannot reuse tenant_a's stored Slack/Gmail/etc. token. | All MCP connectors + all channels use this crate. A leak here breaks claim 6 and exposes every customer's third-party credentials. |
| 3 | `web-workflow-build-layout-ts-rename` | (round 5, IN_REVIEW as #2066) | Workflow build canvas blank — demo-blocker. |
| 4 | `web-workflow-build-fetch-from-gateway` | (round 7) — wire canvas to gateway, not static SEED. | Save-loop works but reload shows seed not saved version. Mission claim 3 edit-loop is half-broken. |
| 5 | `mission-workflow-audit-write-transactional` | (round 7) — wrap workflow + audit in one tx. | Mission claim 6 says "audit chain on every action"; current code silently drops audit on transient sqlite hiccup. |
| 6 | `web-audit-feed-platform-events` | (round 1) — UNION platform events into the operator's /audit view. | Operator can't see WorkerHired / WorkflowRolledBack et al. Visibility gap. |

## Top 13 P1 gaps

7. `eval-cron-duty-fires-agent-runner-end-to-end` — does the duty scheduler ACTUALLY drive the worker?
8. `eval-genesis-pipeline-end-to-end-smoke` — does Genesis produce a valid vertical from a plain-English role?
9. `eval-workflow-execution-runs-real-step` — argo-workflow's executor + checkpoint.
10. `eval-mcp-connector-real-vs-stub-inventory` — 27 connectors; which are real vs deferred stubs?
11. `eval-in-core-tool-inventory-vs-mission` — MISSION.md claims 15 in-core tools; verify.
12. `mission-wire-api-action-executor` — per MISSION.md known-blocked status.
13. `eval-channels-v1-config-vs-code-inventory` — 13 channels in code, 3 in Pro pricing card; matrix the gap.
14. `eval-channels-inbound-coverage` — only email + Slack have inbound paths; the rest are outbound-only.
15. `eval-argo-store-tenant-bind-sweep` — sweep the other 30+ tables for tenant_id binds.
16. `eval-argo-cli-gateway-module-sweep` — complete handler-by-handler RBAC + tenant audit.
17. `eval-plugin-abi-stable` — MISSION.md invariant #1 says polyglot core; verify JSON-RPC ABI is stable.
18. `eval-memory-per-worker-scope-honored` — per MISSION.md status, schema extension queued.
19. `mission-kpi-rollup-from-audit-trail` — Mariah-card KPIs aren't auto-derived from audit events.

## How this campaign runs going forward

Per Mark's directive:

> *"run this for as long as it takes to identify every single gap and get it documented. the conductor can pick those up and have workers always going to complete the work."*

The shape:

1. **Each evaluator session** picks 3-6 unaudited crates from the manifest's "Crates NOT YET TOUCHED" list, reads them, and adds rows.
2. **Each conductor tick** (every 20 min) reads SHARED.md, opens this manifest, picks the top un-actioned 🔴/🟡 row, authors a brief.
3. **Each worker** claims a brief and ships.
4. **Each verifier eval session** re-reads closed rows, flips them 🟢, and adds new gaps that the closing PR may have introduced.

The manifest is the single source of truth for "what's left." When the master gap list at the end of the manifest has zero un-claimed P0/P1 rows, the campaign is done.

## Estimated cadence

- 4-6 crates audited per eval session
- 30 remaining unaudited crates (after subtracting MERGED-confirmed + DEFERRED-v1.x)
- **~5-7 evaluator sessions to reach full mission-vs-code coverage**

Plus, ongoing rounds re-verifying closed gaps and adding new ones the PRs surface.

## What's already in flight from prior rounds

PRs in IN_REVIEW or recently MERGED that close earlier rows:

| PR | Closes | Cumulative impact |
|---|---|---|
| [#2066](https://github.com/mark-hallam/Argo-Agent/pull/2066) IN_REVIEW | round-5 `web-workflow-build-layout-ts-rename` | Build canvas unblanks |
| [#2067](https://github.com/mark-hallam/Argo-Agent/pull/2067) IN_REVIEW | round-1 `mission-hire-audit-payload-shape-validation` | Hire audit shape pinned |
| [#2065](https://github.com/mark-hallam/Argo-Agent/pull/2065) IN_REVIEW | round-3 `claude-conductor-fix-by-enforcement` | FINDINGS-ONLY deadline enforcement |
| [#2064](https://github.com/mark-hallam/Argo-Agent/pull/2064) MERGED | round-5 `claude-dev-safe-check-deps-present` | pnpm-missing-deps fail-loud |
| [#2063](https://github.com/mark-hallam/Argo-Agent/pull/2063) MERGED | round-4 `mission-cu-macos-executor-tenant-preflight-enforce` | Computer-use cross-tenant guard |
| [#2062](https://github.com/mark-hallam/Argo-Agent/pull/2062) MERGED | round-3 `mission-workplace-knowledge-gateway` | Workplace-knowledge zero-caller drift CLOSED |
| CLAIMED `web-dark-mode-class-collision-fix` | round-6 | Light/dark class collision |
| CLAIMED `web-onboarding-f1-route-restore` | round-6 | /onboarding 404 fix |
| CLAIMED `web-workers-empty-state-converge-with-team` | round-6 | Empty-state UX |

## Next session start-here

Open `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`. Read the "Crates NOT YET TOUCHED" list. Pick the top 4-6 by priority. Read their `lib.rs` + their tests. Add rows to the manifest.

If P0 #1 (`eval-argo-agent-runner-turn-lifecycle`) is still open, START THERE — it's the biggest unaudited surface.

Specific crates with the highest-leverage next reads:

1. `argo-agent-runner` — THE runtime
2. `argo-credentials` — cross-tenant target
3. `argo-mcp` (core) — connector substrate
4. `argo-tool` — verify the 15-in-core-tool claim from MISSION.md
5. `argo-workflow` (the executor, not the gateway routes — those are audited)
6. `argo-genesis::pipeline` + `safety_judge` — verify the safety gate is real
