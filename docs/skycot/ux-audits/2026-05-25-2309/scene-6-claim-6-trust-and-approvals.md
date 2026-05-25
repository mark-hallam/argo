# Scene 6 + Approvals — Claim 6 trust / audit / cross-tenant isolation

**Auditor**: e2e-ux-journey-test-agent (read-only).
**Date**: 2026-05-25T23:09Z (Melbourne).
**Target**: dev server on http://127.0.0.1:60894 (preview-driven; mirrors web/ build), live gateway on http://127.0.0.1:7400 (schema v81).
**Scope**: `/approvals`, `/audit`, `/audit/action/<id>`, `/audit/action/<id>/replay`, plus a small probe of cross-tenant isolation against the live gateway.
**Reference**: locked designs (`project_approvals_page_design_locked`, `project_audit_page_design_locked`), function-vs-dressing rule (`feedback_no_theater_in_demo`), v1.0 demo runbook scene 6 step 7 ("Click an audit row → real provenance").

This run **confirms** the active P0/P1 briefs already authored from the prior 2026-05-25-1100 sweep AND surfaces additional issues not yet bookmarked (gateway CORS, KPI-strip dressing on `/`, audit cross-tenant scope bug).

## TL;DR

- 11 findings: 4 P0, 5 P1, 2 P2.
- The biggest blocker is **F1 (CORS)** — on the live gateway every cross-origin GET from the SPA returns with no `Access-Control-Allow-Origin` header, so the browser blocks the response. Effect on Scene 6: `/approvals` and `/audit` render empty states even though the gateway has data. This was visible in the preview server (port 60894) and reproducible from a `127.0.0.1:3000` origin via curl — meaning the v1.0 demo MP4 on the operator's machine will see the same empty states unless the CORS bug is fixed OR same-origin proxying is added to the SPA.
- **F2 (list/detail id mismatch)**, **F3 (replay theatre)**, **F4 (Scene 6 click-through unrecordable)** all already have briefs — they are STILL LIVE on main; none of the fixes have shipped.
- **F5 (cross-tenant isolation gap)** is new: passing `x-argo-tenant-id: ws-malicious` to `GET /api/v1/audit` returns the `anonymous`-tenant rows verbatim (because rows whose JSON payload omits `tenant_id` are unioned into every workspace's view via `row_in_tenant_scope.include_legacy`).
- Dressing-side fixtures on `/` (KPI strip values "42 resolved / wk", "8m avg response", "78% approval rate") are visible on the canonical Scene-6 inbox view but are summary-card decorations the camera does not drill into; flagged as P2 dressing, not POC blockers.

## Headline blockers (the demo cannot be recorded as scripted)

| # | Severity | Finding | Demo impact |
|---|---|---|---|
| F1 | P0 | Gateway returns no `Access-Control-Allow-Origin` on actual GETs (only on OPTIONS preflight). Browser blocks every SPA cross-origin call. | `/approvals` and `/audit` both render empty states on a fully-seeded tenant; Scene 6 cannot show ≥10 audit rows. |
| F2 | P0 | List/detail ID mismatch: every id `/api/v1/audit` returns is 404 from `/api/v1/audit/{id}`. | Click any row → `Action not found` shell. Scene 6 step 7 ("Click a row → real provenance") is unrecordable. Already authored as `fix-audit-list-detail-endpoint-id-mismatch`. |
| F3 | P0 | `/audit/action/{id}/replay` fabricates a forensic verdict (Drifted-misaligned + fake provenance diff + fake outputs) for ANY unknown id, **including real audit ids the list returned**. | The camera sees theatre on a function-side page. Already authored as `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`. |
| F5 | P0 | Cross-tenant scope leak in `/api/v1/audit` list: a request with `x-argo-tenant-id: ws-malicious` (or any non-existent tenant) returns the `anonymous`-bucket rows. | Claim 6 ("Multi-tenant by construction") is structurally violated by the read path. Not currently bookmarked as a brief. |

## All findings (in order)

### F1 [P0] — Gateway omits `Access-Control-Allow-Origin` on real responses; SPA cross-origin fetches all fail with `net::ERR_FAILED`

**What I saw.** Every SPA-originated fetch from `http://127.0.0.1:60894` (the preview server) to `http://127.0.0.1:7400/api/v1/*` shows `[FAILED: net::ERR_FAILED]` in the network panel, even though OPTIONS preflights all return 204. Repeated the same with the canonical SPA origin (`http://127.0.0.1:3000`) via curl — the actual GET response carries no `Access-Control-Allow-Origin` header:

```
$ curl -sv -H 'Origin: http://127.0.0.1:3000' "http://127.0.0.1:7400/api/v1/audit?range=today" 2>&1 | grep "Access-Control"
(no output)
```

The SPA's `apiGet` builds absolute URLs via `gatewayBase()` (= `NEXT_PUBLIC_GATEWAY_URL` = `http://127.0.0.1:7400`), so every read goes cross-origin and is silently dropped by the browser. The `lib/api/audit.ts` + `lib/api/approvals.ts` fallback paths treat a `TypeError` (network) as "missing or unreachable" and quietly return `emptyPage()` / `{items:[]}` — that is why `/audit` shows "Quiet day" and `/approvals` shows the empty `<EmptyState>` even though the gateway has 16 rows in `today`'s window and the audit_record table holds 1,193 anonymous-tenant rows.

**Why it's a problem.** This is the underlying reason Scene 6 looks empty. The demo MP4 records from the same SPA origin → gateway port-7400 pair, so unless the SPA is rebuilt to proxy via `/api/v1/*` same-origin or the gateway is patched to emit CORS headers, the camera will see "Quiet day" and "No approvals waiting" everywhere in Scene 6.

**Repro**
- `curl -sv -H 'Origin: http://127.0.0.1:3000' http://127.0.0.1:7400/api/v1/audit?range=today | grep -i access-control` → empty.
- In the preview, `await fetch('http://127.0.0.1:7400/api/v1/audit?range=today')` → `TypeError: Failed to fetch`.
- Same-origin `await fetch('/api/v1/audit?range=today')` → 200 + payload.

**Suspected fix site**: `crates/argo-cli/src/handlers/gateway/` — find the CORS middleware that emits 204 preflights and add the matching response header on the actual handler responses.

### F2 [P0] — `/api/v1/audit?range=…` returns IDs that `/api/v1/audit/{id}` cannot find

**What I saw.** Confirmed against the live gateway:

```
$ curl http://127.0.0.1:7400/api/v1/audit?range=today | jq -r '.items[0].id'
019e5dbaec037fa2819eab3f27f3f41e

$ curl http://127.0.0.1:7400/api/v1/audit/019e5dbaec037fa2819eab3f27f3f41e
{"error":{"code":"not_found","message":"audit row not found"}}
```

Tried all the obvious overrides — `x-argo-tenant-id: anonymous`, `x-argo-tenant-id: ws-default`, `?as_tenant=anonymous` + `x-argo-audit-cross-tenant: 1`. All 404. Root cause: list reads with an inclusive scope (legacy/anonymous union via `row_in_tenant_scope`); detail hard-filters `WHERE id = ? AND tenant_id = ?` with the request's resolved tenant (`ws-default` by default, never `anonymous` for the row's actual bucket).

**Why it's a problem.** Drill-in is the centrepiece of Scene 6. The SPA constructs `/audit/action/${id}` from list response, so every visible row clicks through to `AuditActionNotFound`.

**Repro**
- Navigate to `/audit/action/019e5dbaec037fa2819eab3f27f3f41e` in the SPA (id taken from live `/api/v1/audit` response).
- Page renders "Action not found — No audit record matches 019e5dbaec037fa2819eab3f27f3f41e."

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:74-83` (`WHERE id = ? AND tenant_id = ?`) vs `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:41-48` (`row_in_tenant_scope` permissive on rows missing payload tenant). Brief already authored: `docs/build-plan/streams/fix-audit-list-detail-endpoint-id-mismatch.md`.

### F3 [P0] — `/audit/action/{id}/replay` renders a fabricated forensic verdict for any unknown id, including real gateway ids

**What I saw.** Navigated to `/audit/action/019e5dbaec037fa2819eab3f27f3f41e/replay` (a real gateway-returned id that itself doesn't exist in any tenant's audit_record under the detail endpoint's scope). The replay screen renders:

> Policy regression detected. Replay output would have failed the current policy. Operator notified.
> Drifted-misaligned
> Output diff … "Reconstructed original output for 019e5dbaec037fa2819eab3f27f3f41e."
> Provenance diff … MODEL same / PROMPT same / SKILL same / MEMORY same / POLICY generic-policy v1.0 → generic-policy v1.1.

This is hand-crafted theatre from `web/lib/sample-replay.ts` (`mockOutcomeForActionId` deterministically picks a verdict via `actionId.charCodeAt(last) % 3`).

**Why it's a problem.** Direct violation of `feedback_no_theater_in_demo`. If a buyer (or the demo's narrator) clicks Replay on a real audit row, they get a fabricated outcome with a fabricated policy-drift narrative. Scene 6 explicitly calls out replay as a function-side moment.

**Repro**
- Hit `/audit/action/<any-id>/replay` with anything that isn't one of the 5 anchor ids in `sample-replay.ts`.
- Replay outcome card renders with verdict / diffs / provenance.

**File**: `web/lib/api/replay.ts:80-87` (the `catch → mockOutcome(actionId)` swallow) + `web/lib/sample-replay.ts:142`. Brief already authored: `docs/build-plan/streams/fix-replay-page-fabricated-forensic-report-for-unknown-action-ids.md`.

### F4 [P0] — Scene 6 drill-in click-through is doubly unrecordable

**What I saw.** Combining F2 + F3: every audit row from the live gateway lands on `Action not found` if the user clicks the row, OR on a fabricated forensic report if the user types `/replay` directly. There is no third option that shows real provenance.

**Why it's a problem.** The runbook step 7 ("Click an audit row → real provenance") is the climax of the trust-and-audit narrative. It cannot be captured against the live backend in its current state.

**Mitigation candidates**:
- (a) Fix F2 — drill-in lands on a real `<ActionSummaryHero>` populated from `audit_record`.
- (b) Until F2 lands, run the demo in `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` so the fixture rows render and the drill-in points at sample IDs that `sample-replay.ts` has hand-crafted outcomes for. Documented in `docs/skycot/demos/v1-mission-mvp-runbook.md`.

Note: the runbook already calls out "demo-mode is the recommended path"; this finding is to record that no real-data path currently exists.

### F5 [P0] — `GET /api/v1/audit` ignores `x-argo-tenant-id` for rows whose payload omits `tenant_id`

**What I saw.** Probed the live gateway:

```
$ curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d&limit=5" -H "x-argo-tenant-id: ws-malicious" | jq '.items | length'
5

$ curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d&limit=5" -H "x-argo-tenant-id: ws-tenant-b" | jq '.items[].id'
"019e5dbaec037fa2819eab3f27f3f41e"   ← same id another tenant sees
"019e5db5097b77d0becda2085091918c"
…
```

Root cause: `web/api routes_audit_feed_scope.rs:41-48` (`row_in_tenant_scope`) treats `payload.tenant_id == None` as "show this row to whoever asks". The audit table has 1,193 rows in tenant `anonymous` whose JSON payloads do not embed `tenant_id`, so every request sees them regardless of which `x-argo-tenant-id` it sends.

**Why it's a problem.** Claim 6 ("Multi-tenant by construction") relies on the audit feed being strictly tenant-scoped — this is one of the load-bearing demo claims. The detail endpoint is correctly strict (F2 is the symptom), but the list endpoint isn't, which is a confidentiality leak. Even the demo audience could discover the leak by inspecting the network panel and changing the tenant header in a curl.

**Repro**: see the curl above.

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:41-48`. Suggested fix: rows without `payload.tenant_id` should be denied unless the request resolves to the legacy `__legacy__` bucket and explicitly opts into legacy union — or migrate the legacy rows to a real tenant_id at read time. **Not currently bookmarked as a brief.**

### F6 [P1] — `/approvals` ops-dashboard (locked design) is unreachable on a real demo tenant

**What I saw.** `/approvals` renders the empty state (`<InboxIllustration>` + "No approvals waiting") because live `/api/v1/approvals` returns `{"items":[]}`. The locked E-ops-dashboard surface (5-KPI strip + 3 severity tiers + $-at-risk donut + 7-day approval-rate ring) only renders when `pendingCount > 0`.

**Why it's a problem.** Either (a) the demo workspace needs seeded pending approvals so the ops-dashboard shows; or (b) the empty-state UX is the only thing the buyer sees. If the runbook positions `/approvals` as the locked ops-dashboard view (and `project_approvals_page_design_locked` does), it's a footgun for first-time recording.

**Suggested fix**: ensure the demo seed CLI inserts ≥3 pending approvals across severity tiers, OR change the empty-state to render a brand-medium "preview of what shows here" if you want the ops-dashboard's components to always have a visual home.

**File**: `web/app/(app)/approvals/page.tsx:38-45` (the `pendingCount === 0 ? <ApprovalsEmptyState/> : <ApprovalsOpsDashboard/>` branch). Related brief: `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity.md`.

### F7 [P1] — `/approvals` ops-dashboard KPI ring + $-at-risk donut hardcode 78% / 0.78 / 0.14 / 0.08 / 0.82

**What I saw.** `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`:

- L139-141 — `rate={{ headline: "78%", slices: { approved: 0.78, refused: 0.14, expired: 0.08 } }}` passed unconditionally to `KpiStrip`.
- L163 — `<MetricDonutCard … percent={totalAtRisk > 0 ? 0.82 : 0} />` — the donut visual fill is a flat 82% regardless of the actual dollar number.
- L180 — `<ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} />` duplicate hardcoded.
- L186 — string "78% approved · 14% refused · 8% expired" literal.

**Why it's a problem.** If the operator zooms a donut and narrates "82% of cap at risk", that becomes function-side theatre. The dollar number itself is real-derived (`totalAtRisk` sums real items); the visual gauge fill is dressing pretending to be function.

**Note**: brief already authored — `docs/build-plan/streams/fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts.md`.

### F8 [P1] — Audit `hash` field absent on live rows; the Etherscan-style hash column never renders

**What I saw.** Live `/api/v1/audit` response shape (per curl, abridged):
```
{"action_type":"hipaa_compliance_check","approver":null,"channel":"web","cost":null,
 "data_classification":"internal","id":"019e5dbaec037fa2819eab3f27f3f41e","status":"succeeded",
 "timestamp":"2026-05-25T06:03:11.747736+00:00","verb":"hipaa_compliance_check",
 "worker":{"id":"","name":"unknown"}}
```
No `hash` field. `ActivityRow.tsx:82-86` conditionally renders `<HashCell />` only when `activity.hash` is truthy, so the "Etherscan parity" column is invisible on real data. The fixture path (`web/lib/sample-audit.ts:149`) computes a hash but it's not a real chain (8×FNV-1a passes; rows don't depend on each other).

**Why it's a problem.** The audit page's positioning is "forensic + compliance view" — the hash column is the load-bearing Etherscan parity affordance. With live data it's missing; with fixture data it's cosmetic. Either way, "every row is hash-chained to the previous" is unrenderable.

**Note**: brief already authored — `docs/build-plan/streams/fix-audit-hash-chain-prev-this-hover-and-live-rows.md`. Confirmed still applies as of this run.

### F9 [P1] — Audit rows' `worker.name` is "unknown" for every live row

**What I saw.** All 16 audit rows in today's window have `worker: { id: "", name: "unknown" }`. They are all `hipaa_compliance_check` verbs from the `web` channel.

**Why it's a problem.** Scene 6 narrates audit drill-in as "see Mariah's escalation". Rows attributed to "unknown" don't read as forensic evidence; they read as scaffolding the team didn't finish. Plus there is no way to filter by worker because the worker dropdown will be empty.

**Suggested fix**: either ensure `hipaa_compliance_check` rows attach a worker id at write time, or filter them out of the user-facing feed (they're security plumbing, not customer-visible activity).

**Note**: brief already authored — `docs/build-plan/streams/fix-audit-unknown-worker-and-hipaa-fixture-rows.md`.

### F10 [P1] — Home `/` Approvals inbox KPI strip hardcodes "42 resolved / wk", "8m avg response", "78% approval rate"

**What I saw.** Even on a brand-new tenant with `pendingCount = 0` and no decision history, the home inbox at `/` renders:
- "RESOLVED / WK · 42 · rolling 7-day total"
- "AVG RESPONSE · 8m · SLA target 2h"
- "APPROVAL RATE · 78% · 7-day rolling"
- the right-hand donut "78% 7-DAY APPROVAL RATE · 78% approved · 14% refused · 8% expired"

These are visible the moment a buyer lands signed-in. The dollar-at-risk tile correctly falls back to "—".

**Why it's a problem.** Per the function-vs-dressing rule, these are KPI cards the camera doesn't drill into — strictly speaking, dressing. BUT they appear on the same row as the (real) "PENDING · 0" tile. If a buyer sees "PENDING 0" but "RESOLVED 42", they will reasonably ask "wait, where did the 42 go?" mid-narration. Dressing inconsistency = trust risk.

**File**: `web/components/skycot/approvals/KpiStrip.tsx:53-80` (the `computeKpiCells` function).

**Suggested fix**: same as F7 — derive from real data, or replace these tiles with values that don't pretend to summarise a 7-day window that doesn't exist yet.

### F11 [P2] — Audit page's "PII redacted — your role shows masked data" banner renders on an empty feed

**What I saw.** Even when `/audit` shows "Quiet day" (zero rows), the redaction banner above it claims "PII redacted — your role shows masked data — Request elevated view →". There's nothing to redact.

**Why it's a problem.** Polish nit: a buyer would reasonably ask "wait, what's being hidden from me?" when the page is empty. The banner should only appear when there are redacted categories present in the rendered rows.

**File**: `web/app/(app)/audit/page.tsx:202-206` — `<RedactionBanner policy={policy} presentCategories={presentCategories} surface="audit-feed" />` is rendered unconditionally; gating on `rows.length > 0 && presentCategories.length > 0` would clean this up.

### F12 [P2] — Hash hover tooltip ("Click to copy") is the only thing the hover surfaces; no "prev → this" chain affordance even in fixture mode

**What I saw.** `web/components/skycot/ActivityRow.tsx:166-197` `HashCell` shows only the row's own hash on hover/title. There is no `prevHash` field on `AuditRow`, no chain edge rendered anywhere. The hash itself in fixture mode is 8×FNV-1a (not a chain).

**Why it's a problem.** The Scene-6 narration calls the hash column the chain. Without a "prev → this" affordance, the chain claim is words-only.

**File**: same as F8. Already covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows.md`.

## Resolved since prior audit

Comparing against `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`:

- **None of the four briefs authored from that audit have shipped.** F2 / F3 / F7 / F8 / F9 / F12 are all still live. The fixes are queued (`fix-audit-list-detail-endpoint-id-mismatch`, `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`, `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`, `fix-audit-hash-chain-prev-this-hover-and-live-rows`, `fix-audit-unknown-worker-and-hipaa-fixture-rows`).
- **New since 1100**: F1 (gateway CORS) and F5 (audit list cross-tenant scope) were not in the prior report.
- **Re-confirmed**: F2, F3, F4, F7, F8, F9, F12.
- **Recharacterised**: F10 (home KPI strip dressing on `/`) — flagged here as P2 because the camera doesn't drill into KPI tiles, but it's worth recording.

## Orientation score

**52 / 100.**

Math, roughly: the locked designs themselves (B3-Etherscan audit, E-ops-dashboard approvals) are well-implemented at the component layer — that's the +. The data plumbing underneath them is broken in at least three load-bearing places (CORS, list/detail ID mismatch, replay fabrication) and one structural place (cross-tenant scope leak). Until F1 + F2 + F3 are addressed, Scene 6 cannot be recorded against the live backend and any buyer who pokes at the network panel will notice the mismatch. Demo-mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`) is the operator's intended workaround per the runbook, which moves the score up; but the POC scope explicitly excludes theatre on function-side surfaces, and demo-mode IS function-side theatre on the audit/replay paths. The locked-design components (KPIStrip / severity tiers / Etherscan rows / HashCell) score around 80; the data path scores around 25.

## What I deliberately did not flag

- The `/landing`, `/compare`, `/customers` mismatches I noticed in passing — out of scope.
- The hydration mismatch errors in the console (Radix `aria-controls` IDs differing server vs client) — this is React 19 Radix behaviour, not Scene-6-relevant.
- Brand voice on the empty states — they're brand-on-message; not a finding.
- The "Active reviewers · 1" hardcoded value in `ApprovalsOpsDashboard.tsx:108` — already covered by the existing brief F7 links to.
