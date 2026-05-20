# Workflow execution observability audit — wave 14

**Date:** 2026-05-21
**Auditor:** Wave-14 audit agent #5
**Scope:** the web surfaces where an operator can see a workflow execution after it has run — `/orchestration`, `/audit`, `/audit/action/<id>`, `/audit/action/<id>/replay`, `/workers/<id>/activity`, `/workflows`, `/workflows/<v>/<n>`, `/workflows/<v>/<n>/build`, `/verticals/<v>/workflows/<n>` (version viewer), `/workers/<id>/evals`.
**Method:** read-only static analysis of `web/app/(app)/` pages, the API client modules under `web/lib/api/`, the locked type contracts under `web/lib/types/audit.ts` + `audit-detail.ts`, and the seed datasets under `web/lib/sample-*`.

---

## TL;DR

**Procurement-grade execution observability is missing.** The platform has rich per-action observability (audit drill-in, decision-chain, provenance, replay) but no per-workflow execution history surface at all. There is no way for an operator to ask "show me the last 50 runs of `support/triage_ticket` with their statuses, durations, costs, and a click into the offending step that failed." The audit data model carries no `workflow_id` / `workflow_version` linkage, so even if such a screen existed, the rows could not be joined back to the workflow they ran.

**Observability score: 2.5 / 5.** Cross-link verdict: **one-way only** (workflow → audit-filtered-by-worker exists; audit → workflow does NOT). Re-run support: **partial — single-action sandboxed replay exists; "re-run a past workflow execution" does not.**

The audit drill-in (`/audit/action/<id>`) is the strongest surface in the entire app and a real procurement asset. The orchestration page covers the in-flight queue well. The gap is the bridge between them: **no per-workflow run table, no per-run drill-in scoped to a workflow execution, no link from an audit row up to "the workflow this came from."**

---

## Dimension 1 — Per-workflow execution history (runs count + status grid)

**Score: 1 / 5. Cosmetic only.**

The workflow detail viewer `/workflows/<v>/<n>` (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:835-942`) ends with a `RUN HISTORY · LAST 7 DAYS` strip showing **four hardcoded numbers**: `82 Runs`, `94% Success rate`, `6.2s Median latency`, `$0.012 Cost / run`. These are static literals embedded in JSX — not bound to any API, not parameterized by workflow, identical for every workflow the operator opens. Lines 859, 882, 905, 928.

There is no:
- list of recent runs (no table, no rows, no timestamps)
- per-run status grid (the 24×7 heat-map procurement decks expect)
- filter by status / time-range / version
- pagination / load-more

The `/workflows` catalog page (`web/app/(app)/workflows/page.tsx`) shows `lastEdited` (workflow definition edit time) and `versions` count per workflow card, but **no `lastRun` field, no recent-runs count, no success-rate badge**. The `SEED_WORKFLOWS` const has no per-workflow run metadata at all.

`/orchestration` (`web/app/(app)/orchestration/page.tsx`) shows the live queue. Each `OrchestrationRun` row (`web/lib/api/orchestration.ts:7-21`) carries `worker`, `skill`, `status`, `slaState`, `latencyMs`, `retryAttempt` — **but no `workflowId`, `workflowName`, or `workflowVersion` field**. The QueueTable (`web/components/skycot/orch/QueueTable.tsx`) renders Worker + Skill columns and exposes Cancel / Reset retry / Force escalate actions — there is no Workflow column and no link from a queue row to a workflow definition.

The verticals workflow viewer `/verticals/<v>/workflows/<n>` (`web/app/(app)/verticals/[verticalId]/workflows/[name]/page.tsx`) shows **definition version history** (commit-style `is_head`, `created_at`, "Compare with head") — confusingly named, this is not run history; it is workflow-source version history.

## Dimension 2 — Per-run drill-in (steps fired, durations, costs, outputs)

**Score: 0 / 5 for workflow runs. 5 / 5 for individual actions inside a run.**

There is no `/workflows/<v>/<n>/runs/<runId>` route. There is no `/orchestration/runs/<runId>` drill-in. The QueueTable in `/orchestration` has no row-click handler that opens a per-run inspector — the only row-level affordances are Cancel / Reset retry / Force escalate (`web/components/skycot/orch/QueueTable.tsx:50-57`).

What exists at `/audit/action/<actionId>` (`web/app/(app)/audit/action/[actionId]/page.tsx`) is excellent for a **single action**:
- `ActionSummaryHero` — worker + verb + status + action id with copy + Replay + Export
- `ActionInputsCard` — trigger kind + summary + predecessor link
- `DecisionChainCard` — the locked 8-step trace (memory loaded → skill considered → plan generated → plan selected → policy check → approval → execution → outcome)
- `ProvenanceBlock` — model + prompt template + skill + memory snapshot + policy version, all with hashes
- `ActionOutputCard` — JSON / text / computer-use screenshots + delivery confirmation + follow-up link

This is procurement-grade for the **action** level. But there is no analogue at the **workflow run** level — no surface that says "Run #1247 of `extract_invoice` started at 14:02, fired step 1 (trigger) in 12ms, step 2 (OCR + line extraction) in 4.2s costing $0.011, step 3 (PO reconcile) failed at retry 2 with this error." Each step of a workflow run, if it left an audit trail, would be a separate action — but the rows do not carry `workflow_run_id`, so there is no way to reassemble them into a run timeline.

## Dimension 3 — Cross-link (audit ↔ workflows)

**Score: 1 / 5. One-way and indirect.**

**Audit → Workflows: BROKEN.** No audit row, no audit detail, no decision step carries a `workflowId` field (confirmed by reading `web/lib/types/audit.ts:103-120` and `web/lib/types/audit-detail.ts:75-83`). The drill-in surface has a "View related actions →" link in `ActionSummaryHero` (line 161), but it links to `/audit?worker=<id>&from=<hour-floor>&to=<hour-ceil>` — i.e. same worker, same hour. NOT same workflow. There is no "View this workflow's other runs" or "Open the workflow definition that produced this action" anywhere.

The 7 audit filter dimensions are: worker, channel, action type, status, approver, cost band, classification (`web/lib/filters/audit.ts:51-58`). **"Workflow" is not a dimension** — the operator cannot filter the audit timeline to a single workflow.

**Workflows → Audit: ONE-WAY ONLY.** `/workers/<id>/activity` has an "Open in Audit timeline →" link (`web/app/(app)/workers/[workerId]/activity/page.tsx:121-129`) that pre-fills `?worker=<id>&range=<range>` — useful for worker-scoped forensics, but again not workflow-scoped. The workflow detail page (`/workflows/<v>/<n>`) has no "see audit rows from this workflow" link at all.

**Orchestration → Workflows: BROKEN.** `/orchestration` does not link to `/workflows/<v>/<n>`. The Skill column shows skill ids (e.g. `invoice.extract`), but skills are not workflows — and there is no link even to `/skills/<id>`.

## Dimension 4 — Re-run a past execution

**Score: 2 / 5. Single-action replay exists; workflow re-run does NOT.**

The action drill-in has a `Replay` button (`web/components/skycot/audit/ActionSummaryHero.tsx:86-90`) which routes to `/audit/action/<id>/replay`. That surface (`web/app/(app)/audit/action/[actionId]/replay/page.tsx`) runs a **sandboxed re-execution of one action** and returns a discriminated verdict: `reproducible` / `drifted-aligned` / `drifted-misaligned` / `unavailable` / `failed` / `leak-detected`. The brief is explicit that side effects do NOT happen — replay is for verification, not for retry-after-failure (`web/lib/api/replay.ts:14-16`).

There is no:
- "Re-run this workflow from the start" button on `/workflows/<v>/<n>` (the "Test run" button on the viewer/builder is a stub — `handleTestRun` in `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:698-706` is a toast that says "Real run streams from the gateway in an upcoming release")
- "Retry this failed run with the same inputs" button on any orchestration row (Reset retry resets the retry counter on a CURRENT queued run; it does not re-launch a completed-failed run)
- "Re-run this run as a draft I can edit before submitting" pattern

## Dimension 5 — Failed-run alerting

**Score: 1 / 5. No workflow-failure notification kind exists.**

The notification system (`web/lib/sample-notifications.ts`) defines 8 `NotificationKind` values: `budget-approaching`, `budget-exceeded`, `skill-promoted`, `skill-drift-detected`, `oauth-needs-reauth`, `duty-output-ready`, `autonomy-promotion-suggested`, `incident`. **None of them is `workflow-run-failed` or `workflow-error-rate-spiked`.**

The `incident` kind exists but the sample row uses it for gateway 502s ("Gateway returned 502 for 4 calls in a 60s window") — i.e. infrastructure, not workflow-step failures. There is no demo notification along the lines of "Workflow `triage_ticket` failed 3 of last 10 runs — investigate."

Per-run failures DO emit audit rows with `status: "failed"` (e.g. `parse_failure` in `web/lib/sample-audit.ts:60`), but those are passive — the operator only sees them if they go look at `/audit` and filter to `status=failed`. No push, no badge, no email, no Slack DM.

The `budget-exceeded` and `skill-drift-detected` notifications are the closest analogues — they prove the surface CAN carry workflow-relevant alerts; the kinds simply aren't authored.

---

## Top 3 gaps for procurement

1. **No per-workflow execution history surface.** A procurement diligence team will ask "show me last 100 runs of your top workflow with status, duration, cost, and a click into the failing step." The only thing the demo can show today is four hardcoded statistics on the workflow viewer (`82 / 94% / 6.2s / $0.012`) that are identical for every workflow the operator opens. Fixing this requires (a) the gateway endpoint `GET /api/v1/workflows/<id>/runs?from=&to=&status=` returning `WorkflowRun[]`, (b) a new route `/workflows/<v>/<n>/runs` rendering the table, (c) a per-run drill-in route at `/workflows/<v>/<n>/runs/<runId>` showing the step-by-step timeline with durations + costs + outputs + status per step.

2. **`AuditRow` has no `workflow_id` / `workflow_run_id` / `workflow_version` linkage.** This is a data-model gap that blocks every cross-link feature: filter audit by workflow, "view this workflow's audit rows" link, group decision-chain steps by run, compute success-rate per workflow. The type lock at `web/lib/types/audit.ts:103-120` is the canonical contract and would need to be extended (additive — three optional fields) before any UI can show workflow-scoped views. The sibling backend brief `mission-gateway-audit-feed-endpoint` referenced in the type-lock comments should be reopened to extend the shape.

3. **No failed-run alerting.** A failed workflow run today is silent unless the operator manually checks `/audit?status=failed`. Procurement teams expect either a push notification, a Slack DM, an email, or at minimum a dashboard badge. The `NotificationKind` union (`web/lib/sample-notifications.ts:11-19`) needs `workflow-run-failed` and `workflow-error-rate-spiked` kinds, and the gateway needs a workflow-runs poller that fires the notification when consecutive failures or burst-failure thresholds trip. Without this, the operator finds out about failures from the customer, which is enterprise-disqualifying.

---

## Honourable mentions (what's strong)

- `/audit/action/<id>` drill-in is the strongest surface in the entire app — 5-card layout with hashes, decision chain, provenance, output, follow-up. This is the foundation; the workflow-level surface should re-use the same pattern.
- `/orchestration` queue + SLA cards + Cancel/Reset/Escalate row actions cover in-flight runs well. The gap is when the run completes — there's no "where did it go" link.
- `/workers/<id>/activity` → `/audit` cross-link is correctly built and proves the linking pattern works; just needs to be replicated workflow-side.
- `EvalRunHistory` component (`web/components/skycot/eval/EvalRunHistory.tsx`) is exactly the visual pattern that should be re-used for `WorkflowRunHistory`. Copy the layout, change the data source.
- The 8-step locked decision-chain trace shows the platform has the underlying execution telemetry — it just isn't surfaced at the workflow-run aggregation level.

---

## Suggested briefs (not authored — operator/conductor call)

1. `web-workflow-run-history-surface` — gateway endpoint + `/workflows/<v>/<n>/runs` table + per-run drill-in. Highest priority; procurement table stakes.
2. `audit-row-add-workflow-linkage` — additive 3-field extension to `AuditRow` (`workflowId?`, `workflowRunId?`, `workflowVersion?`) + gateway populate + audit-filter-by-workflow dimension. Unblocks every cross-link.
3. `notifications-workflow-failure-kinds` — add `workflow-run-failed` + `workflow-error-rate-spiked` to `NotificationKind` + the gateway poller that produces them. Defuses the "we find out from the customer" failure mode.
4. `workflow-builder-real-test-run` — wire the existing "Test run" button to a real sandboxed run with a returned run id that drill-ins to (1). Removes the demo-stub honesty toast.
