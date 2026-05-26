# Scene 6 — Claim 6 — Trust + Approvals + Audit + Hash-Chain UX audit

**Date**: 2026-05-26 ~23:10 AEST
**Scope**: `/approvals`, `/audit`, `/audit/action/<id>`, `/audit/action/<id>/replay`, hash-chain hover, cross-tenant negative test, approve/deny persistence.
**Mode**: Read-only, against running local stack (gateway `127.0.0.1:7400` schema v83, Next dev `localhost:3000`).
**Branch**: `main` @ HEAD `b9deef872`.

## TL;DR

12 findings. **4 P0 demo-blockers**, 5 P1, 3 P2. The page chrome (E-ops-dashboard on `/approvals`, Etherscan-style list on `/audit`) renders cleanly and matches the locked designs. **But the live gateway audit feed is full of self-referential `audit.detail.viewed` / `http.request` meta-rows produced by the SPA itself (worker = "unknown"), and clicking any of those rows lands on a "Action detail unavailable — drill into another row for full provenance" placeholder page with synthetic hashes (`generic-placeholder`, `mem_placeholder`, `generic-policy`)** — exactly the theater the function-vs-dressing rule says is a P0 for Scene 6. The gateway IS producing real chain anchors (verified via curl: `provenance.hash_chain_anchor: "03c2fc6d..."`), but the SPA's `normalizeAuditDetail` silently DROPS the chain anchor field, so the "Hashes pinpoint the exact code, prompt, skill, memory, policy, and chain anchor" promise on the detail page never shows the anchor. Approvals inbox is empty (`{items:[]}` from gateway), so the E-ops-dashboard renders 0s / "—" / "All clear" — not blocking the design but **the camera cannot record an approve/deny without seeded approvals**.

Replay (POST `/api/v1/audit/<id>/replay`) is the one thing that actually works end-to-end: real hash verify, real `verdict: "same"`, real diff envelope. That's a load-bearing positive.

## Headline blockers

1. **F1 [P0]** — Audit-row drill-in renders synthetic placeholder content (`generic-placeholder`, `generic-skill`, `mem_placeholder`) for every gateway-real row. Scene 6's drill-in moment is theater.
2. **F2 [P0]** — Live gateway audit list returns ONLY self-referential `audit.detail.viewed` / `http.request` rows authored by the SPA viewing the SPA. No real agent actions in the feed. Nothing for the camera to drill into.
3. **F3 [P0]** — Gateway's real `hash_chain_anchor` is dropped on the SPA side. The "Provenance & Chain" card promises a chain anchor but never displays it for real rows.
4. **F4 [P0]** — `/approvals` shows an empty inbox (gateway returns `{items:[]}`). The 5-KPI strip is "0 / — / — / — / —" and the severity tiers all say "No high/medium/low severity approvals waiting." The camera cannot demonstrate Approve / Deny against real data.

## All findings

### F1 [P0] — Audit-row drill-in shows placeholder provenance for every real gateway row

When the operator clicks any row in `/audit`, the drill-in route `/audit/action/<id>` calls `fetchAuditDetail`. The gateway DOES respond with a 200 + a `{detail: {...}}` envelope (verified via curl), and the SPA's `normalizeAuditDetail` correctly maps the snake_case payload into the canonical `AuditDetail`. But the gateway's data for every "real" current row is itself empty — `decision_chain` has 8 steps all with `body: ""` and `details: { ... : null }`; `provenance.model.id: ""`, `provenance.skill.id: ""`, `provenance.policy.id: ""`, etc. So the SPA's empty-provenance branch fires and renders `<ChainEmptyState testId="audit-provenance-empty" title="Provenance unavailable" body="This http.request row is a meta-event, so model, prompt, skill, memory, and policy hashes were not produced." />`.

Then for rows that the SPA can't match to a sample hand-shape (i.e., every gateway row), the fallback path in `web/lib/api/audit-detail.ts:117` calls `demoModeDetail(actionId)` → `placeholderAuditDetail(actionId)` (from `web/lib/sample-audit-detail.ts:285`), which fabricates: worker = "Unknown worker", verb = "Action detail unavailable — drill into another row for full provenance.", and the visible provenance card showing **`MODEL: claude-opus-4-7 2026...`**, **`PROMPT TEMPLATE: generic-placeholder`**, **`SKILL VERSION: generic-skill`**, **`MEMORY SNAPSHOT: mem_placeholder`**, **`POLICY VERSION: generic-policy`** — *real-looking but entirely synthesized*.

This was verified visually via screenshot: navigating to `/audit/action/019e6469e52b70e1bd05c25e98819da6` (a real id from the gateway feed) renders exactly that placeholder content with PII-redacted banner and a Replay button.

**Per the function-vs-dressing rule, this is the textbook P0 demo-blocker**: theater the camera sees on a drill-in. Operator cannot truthfully narrate "this is what actually happened" while the displayed model/prompt/skill/memory rows are literal strings `generic-placeholder`.

Repro:
- Start gateway + web. Sign in via demo-mint (`POST /api/auth/demo-mint`).
- Navigate to `/audit?range=30d` — see a list of `http.request` rows.
- Click any row — lands on `/audit/action/<id>`.
- Observe: title "Action detail unavailable — drill into another row for full provenance."; PROVENANCE block shows `generic-placeholder` / `mem_placeholder` / `generic-policy`.

Files:
- `web/lib/api/audit-detail.ts:114-122` (the strict-prod fallthrough to `demoModeDetail`)
- `web/lib/sample-audit-detail.ts:285-310` (the `placeholderAuditDetail` content)

Suggested fix: either seed the gateway with the canonical demo rows (so real action ids in the feed map to real backend rows with non-empty provenance), OR change the fallback to surface an honest "This is a meta event, no agent provenance" empty state without inventing placeholder hashes. Do NOT keep `generic-placeholder` visible on screen during recording.

### F2 [P0] — Live gateway audit feed contains only self-referential SPA meta-rows

`curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d"` returns 26 rows; every single one has `action_type ∈ {"audit.detail.viewed", "audit.replay"}`, `verb: "http.request"`, `worker: {"id":"", "name":"unknown"}`, `approver: null`, `cost: null`. These are the audit rows produced when the SPA itself viewed `/audit/action/<id>` or POSTed `/api/v1/audit/<id>/replay`. There is no `vendor.refund.approved`, `candidate.screened`, `email.sent`, no MCP tool call, no computer-use action — none of the agent activity that Scene 6's narrative depends on.

The result is that even if F1 were fixed by seeding the gateway, the seed step has not been run on this stack — there are no real agent turns to inspect. The camera will see "http.request from unknown worker" for every row.

Repro:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items | group_by(.action_type) | map({type: .[0].action_type, count: length})'`
- Returns only `audit.detail.viewed` (23) + `audit.replay` (3).

Suggested fix: run the `argo demo seed --tenant <demo>` CLI (or equivalent) before recording. If that CLI does not exist or does not seed the canonical 5-worker × N-action set documented in `v1-mission-mvp-runbook.md`, that's a separate ship-gate.

### F3 [P0] — Gateway's real `hash_chain_anchor` is silently dropped by the SPA normalizer

`curl -s "http://127.0.0.1:7400/api/v1/audit/<id>"` returns `detail.provenance.hash_chain_anchor: "03c2fc6d2e721713c3cfb87c0d457f472ef2aa031a4394f640c1638053a86373"` — a real 64-char SHA-256. The SPA's `normalizeProvenance` at `web/lib/api/audit-detail.ts:206-215` reads only `model / promptTemplate / skill / memorySnapshot / policy` and discards the `hash_chain_anchor` field. The detail page's `<ProvenanceBlock>` component at `web/components/skycot/audit/ProvenanceBlock.tsx:84-96` is fully wired to render a chain-anchor row when `provenance.hashChainAnchor` or `provenance.hash_chain_anchor` is present — but the normalizer never threads it through.

The header copy in the card promises "Hashes pinpoint the exact code, prompt, skill, memory, policy, **and chain anchor** that produced this action." — that anchor cannot appear, even when the gateway IS sending it.

Repro:
- `curl -s "http://127.0.0.1:7400/api/v1/audit/<id>" | jq '.detail.provenance.hash_chain_anchor'` → returns a hash.
- Open `/audit/action/<id>` — no anchor row in the Provenance card.

Files:
- `web/lib/api/audit-detail.ts:206-215` — `normalizeProvenance` returns 5 fields; needs to add `hashChainAnchor: stringValue(p.hashChainAnchor ?? p.hash_chain_anchor)`.

Suggested fix: pass the anchor field through normalization. The downstream UI is already plumbed.

### F4 [P0] — `/approvals` shows an empty inbox; no approvals to approve/deny on camera

`curl -s "http://127.0.0.1:7400/api/v1/approvals"` returns `{"items":[]}`. The `/approvals` landing page (web/app/(app)/approvals/page.tsx) renders the locked E-ops-dashboard correctly — BrandLoudHero with "Approvals." Audiowide gradient title, 5-KPI strip (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS), 3 severity tiers (HIGH / MEDIUM / LOW) and the right rail (donuts + ring). All KPIs read `0` or `—`. All three severity sections say "No high/medium/low severity approvals waiting." The donuts show `0%`. The ring says "No resolved approvals yet."

Per the locked design memo this is the intended 0-state, and per the 2026-05-26 commit message it is intentional that the dashboard renders even when items are empty. But for Scene 6 specifically, the operator cannot demonstrate "press Approve, it persists, it lands in audit" — there's nothing to approve.

Repro:
- Sign in (demo-mint). Navigate to `/approvals`.
- Observe all KPI cells = `0` or `—`; severity tiers empty; no rows.

Files:
- `web/app/(app)/approvals/page.tsx:31` — `const approvals = approvalsQuery.data ?? []` honestly reflects backend state.
- Gateway-side: no seed running.

Suggested fix: seed N pending approvals as part of the demo bootstrap (same `argo demo seed` step as F2). Until then, Scene 6's narrative of "Mark approves Mariah's refund decision on camera" is unrecordable.

### F5 [P1] — Hash-chain hover affordance on the audit list is invisible for real-gateway rows

`web/components/skycot/audit/AuditDayGroup.tsx:79-93` maps each `AuditRow` to an `<ActivityRow>` with `hash: row.selfHash ?? row.sampleHash` and a `kind: "self" | "sample"` label. The `<HashCell>` only renders when `activity.hash` is truthy (`web/components/skycot/ActivityRow.tsx:98`). The gateway list envelope (`/api/v1/audit?range=...`) returns rows WITHOUT `self_hash`/`hash`/`sample_hash` fields — only id/verb/worker/status/timestamp/action_type/channel/cost/data_classification. So the live audit list shows NO hash cells at all — the "Etherscan-style truncated cell" promised in the locked design is suppressed.

Repro:
- View `/audit` with live gateway. Inspect any row — no `data-hash` attribute, no clickable hash badge.
- Switch to demo-mode mock (NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1) — `sampleHash` strings appear.

Suggested fix: either teach the gateway's list endpoint to include `self_hash` in each row envelope, OR derive a displayed-only hash client-side from the row id (and label it accordingly). Without a hash on the list rows, the table cannot communicate the "every action is hash-chained" message Scene 6 needs.

### F6 [P1] — All gateway audit endpoints accept requests without authentication and ignore X-Tenant-Id header

The gateway is currently running with `ARGO_GATEWAY_AUTH_BYPASS=1` (verified in `ps`). Anonymous unauthenticated `curl http://127.0.0.1:7400/api/v1/audit?range=7d` returns all audit rows. Sending `X-Tenant-Id: tenant_evil` (or any other tenant header) returns the same rows. Sending `Cookie: session=fake-other-tenant-session` also returns the same rows.

This is the dev/auth-bypass mode of the gateway and is by design for the recording session. **But** the cross-tenant negative test asked for cannot be meaningfully performed against this binary. To actually validate cross-tenant isolation we need an auth-on build + a multi-tenant seed + two different cookies.

The good news: gateway code DOES gate at the SQL layer per memory `feedback_no_theater_in_demo` + the 2026-05-25 merge wave that landed tenant-binding via #2167 + #2218 + #2245 + #2257. The bad news: I cannot empirically verify it from this stack. Status from prior round-54 evaluator + #2257 cross-tenant-pin lands shows it as LIVE.

Suggested fix: run a small cross-tenant `cargo test -p argo-gateway tenant_isolation_audit_drill_in` in CI ahead of the demo. Don't try to demo cross-tenant in the camera shot — the theater of "here's me failing to fetch tenant_evil's row" is much worse than skipping it.

### F7 [P1] — Workers/[id] page crashes with `Objects are not valid as a React child` infinite loop, contaminates Scene 6 nav

Browser console floods with `Maximum update depth exceeded` errors plus a hard React render error: `Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version}).` originating from `[skycot:workers/[workerId]] worker detail`. The render failure is caught by the per-route error boundary, but the recovery cycle (boundary catches → state reset → re-render → re-throw) interacts with router prefetching and produces sporadic redirects from any in-app route to `/workflows` / `/skills` / `/compare` / `/pricing` while the React loop is active.

During the audit I lost 3 navigation attempts to /audit because the page bounced to /workflows during the React fault. Once a worker detail page has been visited in the session, subsequent `window.location.href = '/audit'` calls sometimes settle on whatever route the broken hook lands on. Hard reload of about:blank → /audit eventually works.

This is **technically Scene 1's bug** (worker detail), not Scene 6's, but it pollutes Scene 6 nav because the runbook scripts say "go from worker detail back to audit". On a clean session (no worker drill-in first), `/audit` and `/approvals` load fine.

Repro:
- Sign in. Navigate to `/workers/<any worker id>`.
- Open dev tools console — see `Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version})` and `Maximum update depth exceeded` spam.
- Now try `window.location.href = '/audit'` — frequently bounces.

Files (best guess based on error key shape): a worker memory/fact field in `web/app/(app)/workers/[workerId]/memory/page.tsx` or `web/components/skycot/worker/` is rendering a full memory fact object instead of its `text` field.

Suggested fix: separate brief. Find the `<p>{fact}</p>` (where `fact` is an object) and change to `<p>{fact.text}</p>`. Should not need a worker drill-in to be live for Scene 6 — the runbook says Scene 6 is `/approvals` + `/audit` after Scene 5.

### F8 [P1] — `(app)/page.tsx` and `/approvals` page show inconsistent KPI labels

Root canonical inbox at `/` (which is the "Approvals" nav target) renders a KPI strip with `PENDING / RESOLVED / WK / AVG RESPONSE / $ AT RISK / APPROVAL RATE`. The `/approvals` standalone landing renders `PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS`. Different column count (5 vs 5 but different labels), different positions for `$ AT RISK`. The locked E-ops-dashboard memo says 5-KPI strip but doesn't name all 5 — both pages comply with the spec but disagree with each other.

Repro:
- View `/` after sign-in — KPI labels are one set.
- View `/approvals` — different set, different order.

Files:
- `web/app/(app)/page.tsx:24` (uses `computeKpiCells` from `KpiStrip`)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (the standalone E-ops dashboard)

Suggested fix: pick one set + one order. The `/approvals` set (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS) reads more enterprise/SLA-shaped and matches the locked memo better.

### F9 [P1] — Audit page header reads "Forensic + compliance view", not the Etherscan headline expected by locked design

`/audit` page header reads "Audit / Forensic + compliance view" + a Search box. The locked design memo (`project_audit_page_design_locked`) says "Etherscan-style + Inter + pop-out modal." The screenshot shows the page IS Etherscan-shaped (sticky `TODAY` day header, monospace timestamps, hash-chain promise, single-column list, no modal — drill-in is a route). The hash-chain truncated cell promised in the locked design is missing (see F5). And the drill-in is a full route push (`/audit/action/<id>`), not the "pop-out modal" the memo describes.

This is a memo-vs-implementation drift, not an implementation bug per se. Operator should decide if the locked-design's modal is still wanted or if the full-route drill-in supersedes.

Repro: View `/audit` — observe full-route drill-in vs the memo's modal claim.

### F10 [P2] — Demo banner says "Sign in to see your own workspace" but user is already signed in

The persistent `<DemoBanner />` at the top reads "Demo mode — showing sample data. Sign in to see your own workspace." while showing the signed-in account pill "Sam Harlow · Acmeco · Owner" in the bottom-left. The banner copy is unconditionally present regardless of session state. Not a P0 — the camera doesn't narrate the banner — but if the viewer reads it, the contradiction is jarring.

Files:
- `web/components/skycot/DemoBanner.tsx`

Suggested fix: gate the "Sign in to see your own workspace" CTA on `session === null`, OR rewrite to "Demo mode — showing seeded sample data."

### F11 [P2] — Replay button on the detail page is the one thing that actually works end-to-end

Positive note. `POST /api/v1/audit/<id>/replay` returns a real envelope:
```
{"actual_self_hash":"03c2fc6d...","audit_id":"019e643a...","diff":{},"expected_self_hash":"03c2fc6d...","kind":"http.request","verdict":"same","seq":6614,...}
```
The SPA's `web/lib/api/replay.ts` correctly accepts this gateway shape, computes `provenanceDiff` from `diff`, and routes to the `ReplayDiff` + `ProvenanceDiffCard` components. `verdict: "same"` is the real hash-verify result. **This is the one function-side claim-6 surface that does NOT need seeding to demo truthfully** — if the operator clicks Replay on any row, they get a real hash-equality result.

The only catch: the detail page that the Replay button lives on is the placeholder page (F1), so the operator has to know to focus the camera on the Replay outcome panel and not on the surrounding `generic-placeholder` chrome.

### F12 [P3] — `/approvals` deep-link works but `Approvals` nav link routes to `/` (the canonical inbox), so they're two different pages with similar names

The sidebar `Approvals` nav item targets `/`, which is the queue inbox. Typing `/approvals` in the URL bar takes you to the standalone E-ops-dashboard landing page (different layout, different KPI labels — see F8). For a buyer doing a 30-second site tour this is two different pages they might encounter sequentially without realising they're not the same page.

Suggested fix: pick one. If `/approvals` is meant to be a SEO/bookmark landing, redirect it to `/` on auth. If `/` is meant to be the rich landing and the typed URL `/approvals` is supposed to be a niche bookmark, surface a small breadcrumb on `/approvals` clarifying the relationship.

## Function-vs-dressing summary

| Element | Side | Status |
|---|---|---|
| Approvals inbox empty state visual chrome | Dressing | OK |
| Approve / Deny click path | Function | **BLOCKED** — empty queue, nothing to approve (F4) |
| Audit list page header + Etherscan layout | Dressing | OK |
| Audit list hash-cell column | Function | **BROKEN** — real rows missing `self_hash` (F5) |
| Audit row drill-in | Function | **BROKEN** — placeholder content (F1) |
| Audit drill-in chain-anchor row | Function | **BROKEN** — silently dropped by SPA (F3) |
| Replay POST → verdict + hash diff | Function | **WORKS** — real verify (F11) |
| Cross-tenant negative test from camera | Function | Untestable in current auth-bypass stack (F6); per SQL gate work in #2167+ should be wired |

## Orientation score

**42 / 100.** The locked designs render. The replay endpoint genuinely hash-verifies. But three of the four pillars the camera lights up in Scene 6 — approvals queue, audit drill-in, chain anchor — show emptiness, placeholder strings, or silent field-dropping. **The Scene 6 demo as-currently-deployed is not recordable without theater.** Highest-leverage fix is F1+F2+F3+F4 as a single chunk: seed the gateway with the canonical 5-worker × N-action set, then F3 (10-line normalizer fix) and F5 (gateway list-row hash field) become small follow-ups instead of P0s.
