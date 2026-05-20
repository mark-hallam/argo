# Wave-6 Action-Fires-Correctly Audit — 2026-05-21

**Question:** Does the button actually DO what it claims?

**Method:** Read every interactive-element handler in `web/app/` and trace it
to a real API call, a real router navigation, a real state mutation, or
classify it as a stub/dead/silently-broken.

**Scope:** demo-path priority (Approvals, Hire, Team, Workers, Workflows,
Workflow Builder, Audit). Also touched: Skills, Settings, Billing,
Orchestration, Cohorts, Support, Help widget, Notifications.

**Verdict legend:**

- **REAL** = handler hits a real endpoint with real network call (or via mutation hook); audit-trail and side-effects fire.
- **CREDIBLE-STUB** = handler simulates the action with `setTimeout` toast or local optimistic state; demo-believable but no audit row, no persistence.
- **SILENT-BROKEN** = button is wired with handler but the handler does the wrong thing or fails silently (e.g. logs to `console.warn`, fires a vestigial "ships in E4-h" message, navigates to a 404).
- **LOUDLY-BROKEN** = button has NO `onClick` handler at all — clicking does literally nothing.

---

## Action matrix

| Page | Action | Handler-real? | Toast-truth? | API hits real endpoint? | Audit row written? | Severity |
|---|---|---|---|---|---|---|
| `/` Approvals | "Approve" / "Refuse" | REAL | YES | `POST /api/v1/approvals/<id>/decide` | YES (server-side) | OK |
| `/` Approvals | "Approve all" (batch) | REAL | YES | N parallel `POST .../decide` | YES | OK |
| `/` Approvals | "Undo" | REAL | YES | `POST /api/v1/approvals/<id>/undo` | YES | OK |
| `/` Approvals | "Hire teammate" header CTA | REAL — `router.push("/hire")` | n/a | n/a | n/a | OK |
| `/` Approvals | Notifications bell | REAL — lazy-loads `NotificationsPopover` | n/a | `markAllAsRead` mutation on open | YES | OK |
| `/` Approvals | Filter popover | REAL — debounced URL writes | n/a | re-queries audit feed | n/a | OK |
| `/team` | Search filter | REAL — debounced URL write + filter | n/a | n/a (client filter) | n/a | OK |
| `/team` | Status filter | REAL — URL write | n/a | n/a | n/a | OK |
| `/team` | "Hire" CTA | REAL — `<Link href="/hire">` | n/a | n/a | n/a | OK |
| `/team` TeammateCard | "Open profile" | REAL — `router.push("/workers/<id>")` | n/a | n/a | n/a | OK |
| `/team` TeammateCard | Long-press / right-click context menu | REAL — `useWorkerActions` hook | YES | `POST /api/workers/<id>/freeze` (and similar) | YES | OK |
| `/team` TeammateCard | Pause / Resume (context menu) | REAL — gateway call | YES | `/api/v1/workers/<id>/pause` etc. | YES | OK |
| `/workers/<id>` | Freeze / Unfreeze | REAL — proxy to gateway | YES | `POST /api/v1/workers/<id>/freeze` | YES | OK |
| `/workers/<id>/overview` | "Edit charter" (WhatSheDoesCard) | **SILENT-BROKEN** | TOAST LIES — says "Charter changes persist in E4-g" | NO — `console.warn` only | NO | **HIGH** |
| `/workers/<id>/overview` | "Add responsibility" (DutiesCard) | REAL (dialog opens) BUT misleading announce toast also fires | toast says "Add responsibility dialog ships in E4-h" while dialog is opening | YES via `AddResponsibilityDialog → /api/v1/workers/<id>/duties` | YES | **MEDIUM** (works, but vestigial toast embarrassing on camera) |
| `/workers/<id>/overview` | "Edit duty" row click | **SILENT-BROKEN** | toast says "Edit duty ships in E4-h" | NO | NO | **MEDIUM** |
| `/workers/<id>/overview` | Duty toggle on/off switch | **SILENT-BROKEN** | no toast; UI flips locally | NO — `console.warn` only | NO | **HIGH** (toggle appears to work but server-side nothing changes) |
| `/workers/<id>/memory` | "Add fact" | REAL — `useAddFact` optimistic mutation | YES | `POST /api/v1/workers/<id>/memory/facts` | YES | OK |
| `/workers/<id>/memory` | "Edit fact" / "Delete fact" | REAL | YES | `PATCH/DELETE .../facts/<id>` | YES | OK |
| `/workers/<id>/memory` | "Adjust voice" | REAL — opens RetrainDialog inside VoiceProfileSummary | n/a | dialog calls retrain endpoint | YES | OK |
| `/workers/<id>/settings` | Approval policy edit | REAL — `onSubmit` wires to mutation | YES | `PATCH .../policy` | YES | OK |
| `/workers/<id>/responsibilities/new` | ProposeWorkflowChat → "Approve" | REAL | YES | streams to `/api/workflows/propose` then `POST /api/v1/verticals/<v>/workflows/<n>` | YES | OK |
| `/workflows` | Search by name/vertical/desc | REAL — useMemo filter | n/a | n/a (client filter) | n/a | OK |
| `/workflows` | "Browse 50+ templates" | REAL — `<Link href="/workflows/templates">` | n/a | n/a | n/a | OK |
| `/workflows` | "Propose new" button | **LOUDLY-BROKEN** | n/a | n/a — **no `onClick` handler** | NO | **HIGH** |
| `/workflows` | "TRY" chips (approval flows, etc.) | **LOUDLY-BROKEN** | n/a | n/a — **no `onClick`** | NO | **MEDIUM** (suggested searches do nothing) |
| `/workflows` workflow card | Card click | REAL — `<Link href="/workflows/<v>/<n>">` | n/a | n/a | n/a | OK |
| `/workflows/templates` | Search | REAL — useMemo filter | n/a | n/a | n/a | OK |
| `/workflows/templates` | Vertical + risk chips | REAL — setState | n/a | n/a | n/a | OK |
| `/workflows/templates/<id>` | "Use template" CTA | REAL — `<Link href="/workflows/<v>/<n>/build?from_template=<id>">` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>` (timeline) | "Open builder" | REAL — `<Link>` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>` (timeline) | "Version history" | REAL — `<Link>` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>` (timeline) | "Test run" button | **LOUDLY-BROKEN** | n/a | n/a — **no `onClick`** | NO | **HIGH** (demo-path button does nothing) |
| `/workflows/<v>/<n>` (timeline) | Per-step "Edit" pencil | **LOUDLY-BROKEN** | n/a | n/a — **no `onClick`** | NO | **HIGH** |
| `/workflows/<v>/<n>/build` | "View timeline" link | REAL — `<Link>` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>/build` | "Test run" header button | **CREDIBLE-STUB** | toast lies — says "Test run started" then "Test run complete. All 5 steps passed. No errors." after 1.6s | NO API call | NO audit row | **HIGH** (lies to operator on camera; would lie to a buyer) |
| `/workflows/<v>/<n>/build` | "Save + publish" header button | **CREDIBLE-STUB** | toast lies — says "Saving + publishing v3..." then "Published v3. Workflow is live for the tenant." after 0.9s | NO API call | NO audit row | **HIGH** (a buyer who clicks this thinks workflow shipped — it didn't) |
| `/workflows/<v>/<n>/build` | "Propose" AI prompt send | **CREDIBLE-STUB** | toast says "Proposing: '<prompt>'" then "Proposal ready for review. 1 step change proposed." after 1.4s | NO — `/api/workflows/propose` exists but is NOT called from here (it IS called by `ProposeWorkflowChat` at `/workers/<id>/responsibilities/new`) | NO audit row | **HIGH** (the SKYCOT class-leading differentiator is a fake) |
| `/workflows/<v>/<n>/build` | Prompt suggest chips ("Add a step" / "Remove gate") | REAL — `setPromptText(...)` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>/build` | Canvas node click → side panel | REAL — `setSelectedNode` | n/a | n/a | n/a | OK |
| `/workflows/<v>/<n>/build` | Side panel "Edit step" button | **LOUDLY-BROKEN** | n/a | **no `onClick` handler** | NO | **HIGH** |
| `/workflows/<v>/<n>/build` | "Add step" floating button | **LOUDLY-BROKEN** | n/a | **no `onClick` handler** | NO | **HIGH** |
| `/audit` | Search input | REAL — debounced URL write + re-query | n/a | re-fetches audit feed with `q` | n/a | OK |
| `/audit` | Range picker (24h / 7d / 30d) | REAL — URL write | n/a | re-fetches | n/a | OK |
| `/audit` | "Custom range" picker option | **CREDIBLE-STUB** | toast says "Custom range picker ships in E5-b" | NO | NO | **LOW** (admits stub honestly) |
| `/audit` | Filter chips (worker / channel / action type / etc.) | REAL — URL write + client filter | n/a | n/a | n/a | OK |
| `/audit` | Row click → drill-in | REAL — `<Link>` to `/audit/action/<id>` | n/a | n/a | n/a | OK |
| `/audit` | Hash copy (action id) | REAL — `navigator.clipboard.writeText` | YES (icon flips to ✓) | n/a | n/a | OK |
| `/audit` | "Load more" pagination | REAL — `feed.fetchNextPage()` | n/a | re-fetches with cursor | n/a | OK |
| `/audit/action/<id>` | "Replay" button | REAL — `router.push(".../replay")` | n/a | n/a | n/a | OK |
| `/audit/action/<id>` | "Export" button | REAL — opens EvidenceBundleExporter | YES | mutation hook calls real export endpoint | YES | OK |
| `/audit/action/<id>` | "Back" button | REAL — `router.back()` | n/a | n/a | n/a | OK |
| `/audit/action/<id>` | "View related actions" link | REAL — `<Link>` with worker + time range | n/a | n/a | n/a | OK |
| `/audit/action/<id>` | Action-id copy button | REAL — clipboard | n/a | n/a | n/a | OK |
| `/audit/action/<id>/replay` | "Export replay artifact" | **CREDIBLE-STUB** | toast says "Replay artifact export ships in E5-e" | NO | NO | **LOW** (admits stub honestly) |
| `/skills` | Search | REAL — local filter | n/a | n/a | n/a | OK |
| `/skills` | "TRY" chips | REAL — `setQuery(chip)` | n/a | n/a | n/a | OK |
| `/skills` skill card | Card click → `/skills/<name>` | REAL navigation — BUT destination is "coming soon" stub page | n/a | n/a | n/a | **HIGH** (any skill card click reveals a `<div className="stub-stage">` reading "Skill detail coming soon") |
| `/cohorts` | Card click → drawer | REAL — `setOpenCohortId` | n/a | n/a | n/a | OK |
| `/cohorts` | "Create cohort" empty-state CTA | **SILENT-BROKEN** | n/a | navigates to `/cohorts/new` which doesn't exist (404) | n/a | **LOW** (only fires when tenant has 0 cohorts) |
| `/billing` | "Manage subscription" | REAL — Stripe portal mutation | n/a | `POST /api/billing/portal` | YES (Stripe-side) | OK |
| `/billing` | Plan card "Upgrade" CTAs | REAL — Stripe checkout mutation | n/a | `POST /api/billing/checkout` | YES (Stripe-side) | OK |
| `/settings` | Tile click | REAL — `<Link>` for non-`coming:true` tiles | n/a | n/a | n/a | OK (5 of 9 tiles render disabled `<div>` w/ "v1.x" badge — not broken, just gated) |
| `/settings/members` | Invite member | REAL (per file header — stubbed PATCH, but real seeded data) | n/a | gateway PATCH | partial | **LOW** |
| `/settings/api-keys` | Token rotation | REAL | n/a | gateway endpoint | YES | OK |
| `/settings/data-export` | Export bundle | REAL — opens EvidenceBundleExporter | YES | mutation | YES | OK |
| `/orchestration` | "Cancel run" | REAL | YES | `POST /api/v1/orchestration/runs/<id>/cancel` | YES | OK |
| `/orchestration` | "Reset retry" | REAL | YES | `POST .../reset-retry` | YES | OK |
| `/orchestration` | "Force escalate" | REAL | YES | `POST .../force-escalate` | YES | OK |
| Sidebar | Workspace footer ("Sam Harlow · Acmeco · Owner") | **LOUDLY-BROKEN** | n/a | n/a — **no `onClick`** | NO | **MEDIUM** (every page has it; clicking does nothing — operator looks confused on demo) |
| Help widget | Submit ticket | REAL — `useCreateTicket` mutation | YES | `POST /api/support/ticket` | YES | OK |
| Help widget | SLA link | REAL navigation — BUT target `/docs/skycot/support/sla.md` is a repo file, NOT served by Next.js (404 in production) | n/a | n/a | n/a | **LOW** |

---

## Tally

- **Total interactive actions audited:** 71
- **REAL:** 47 (66%)
- **CREDIBLE-STUB:** 5 (7%)
- **SILENT-BROKEN:** 4 (6%)
- **LOUDLY-BROKEN:** 7 (10%)
- **REAL-but-dest-broken:** 3 (4%) — navigates somewhere real, but the destination is a stub page or 404
- Other (read-only / nav / clipboard): the rest

## Top 5 silent-broken demo-path actions (will burn on camera)

1. **Workflow builder "Save + publish"** — Lies. Toast says "Published v3. Workflow is live for the tenant." Nothing was saved. Anyone who calls the API will see no version v3.
2. **Workflow builder "Test run"** — Lies. Toast says "Test run complete. All 5 steps passed. No errors." Nothing ran. A buyer asking "show me the logs" gets nothing.
3. **Workflow builder "Propose" (AI prompt)** — Lies. Toast says "1 step change proposed. Approve to apply to the canvas." Canvas does not change; no proposal exists. The `/api/workflows/propose` endpoint *does* exist (and is called correctly from `/workers/<id>/responsibilities/new`) — the builder just doesn't use it.
4. **Worker overview "Edit charter"** — Lies. Toast says "Charter changes persist in E4-g" (which is an internal brief ID — leaks build process). `console.warn` only; nothing saved.
5. **Duty toggle on/off switch on `/workers/<id>/overview`** — UI flips visually; nothing persists. `console.warn` only. The most dangerous silent-broken because the operator believes she has just paused a duty.

## Top 7 loudly-broken (button does literally nothing)

1. **`/workflows` "Propose new"** — Header CTA does nothing. Operator clicks it on camera, watches it not respond, has to recover ("uh, you click the workflow card to edit one").
2. **`/workflows/<v>/<n>` (timeline) "Test run"** — Big "Play" button, no handler.
3. **`/workflows/<v>/<n>` (timeline) per-step pencil "Edit step"** — Pencil icon button, no handler. Every step has one.
4. **`/workflows/<v>/<n>/build` "Add step" floating button** — Gradient pill in bottom-right of canvas, "Add step" label. Click → nothing.
5. **`/workflows/<v>/<n>/build` side panel "Edit step" button** — Inside the panel that opens when you click a node. Full-width gradient button. No handler.
6. **`/workflows` TRY chips** — "approval flows", "data extraction", "outbound qualification", "escalation". Visible call-to-action; no handler.
7. **Sidebar workspace footer** — "Sam Harlow · Acmeco · Owner" with a chevron-down. Looks like a workspace switcher / sign-out menu trigger. No handler. Present on every (app) page.

## Single biggest fix-before-record

**`/workflows/<v>/<n>/build` "Propose" button.** This is the SKYCOT class-leading differentiator — the natural-language workflow editor — and it's a `setTimeout` toast. The real endpoint exists (`web/app/api/workflows/propose/route.ts`) and is even called correctly from `/workers/<id>/responsibilities/new` via `<ProposeWorkflowChat>`. Wiring the builder's `handleProposeChange` to the same component (or the same fetch) would convert the demo from "this is a polished pitch deck" to "this is real software." Cost: maybe 80 lines of code (replicate `ProposeWorkflowChat`'s streaming/diff logic into the builder's prompt-bar, or reuse the component inline).

## Are the workflow-builder Propose / Test run / Save+publish toasts believable to a buyer?

**Propose:** Mostly believable — the 1.4s "loading" + then "Proposal ready for review. 1 step change proposed. Approve to apply to the canvas." sounds like an AI thinking. **BUT** a buyer who then asks "show me the diff" / "where do I approve?" will see nothing change on the canvas. The toast promises something the UI doesn't deliver.

**Test run:** **Not believable to a buyer who watches closely.** Real test runs stream events (worker output, tool calls, decision branches). A 1.6s loading-then-success toast with "All 5 steps passed. No errors." is what a *mock* says, not what a real engine says. A technical buyer will immediately ask "where's the trace?" and there isn't one.

**Save + publish:** **Most damaging because it's the most consequential.** The toast says "Workflow is live for the tenant." A buyer will reasonably ask "OK, now route a real ticket through it" — and nothing has actually changed in the gateway. The version number "v3" in the toast is hardcoded, not real.

The pattern across all three is: **the toast lies about state.** A safer credible-stub would say things like "Saved as draft (not yet published in this demo build)" or just stay silent. The current copy actively claims completion of work that didn't happen.

---

## Suggested action ladder before record

1. **Remove the toast lies** on Propose/Test run/Save+publish — change copy to admit the demo-stub state OR wire the endpoint. (Cheap copy edit if wiring is out of scope.)
2. **Add `onClick` handlers** to the 7 loudly-broken buttons. Minimum: open the existing AddStep modal (build it inline; can be a "coming in v1.1" panel that admits the gap). For workspace footer: open a profile/sign-out menu.
3. **Wire `Edit duty` toggle and `Edit charter`** to real mutations, OR hide the controls behind a `permissions.canEditCharter === false` short-circuit so they don't render at all.
4. **Strip internal brief IDs** ("E4-g", "E4-h", "E5-b") from any toast copy that ships to operators. "Saves persist in E4-g" is a leaked build-process artifact.
5. **Fix the `/skills/<name>` stub page** — at minimum render the skill name, description, and a "this skill is used by X workers" stat, even if no version-history yet.
6. **Add the sidebar profile menu** (sign out / switch workspace) — the footer button is on every page; demo cameras will land on it.
