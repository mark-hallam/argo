# Scene 6 + Approvals + Audit — Claim 6 (Multi-channel + multi-tenant trust surface)

**Run**: 2026-05-26 21:52 AEST
**Scope**: `/approvals`, `/audit`, `/audit/action/<id>`, `/audit/action/<id>/replay`, gateway cross-tenant negative test, settings → tenant surfacing
**Mode**: dev server on `:3000` (preview-launch from `web/.claude/launch.json`) + gateway on `:7400` schema v83 FRESH; flags `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` set in launch.json
**Approach**: read-only — driver was preview-mcp where stable, otherwise curl + source-trace

---

## TL;DR

**16 findings — 4 P0, 5 P1, 5 P2, 2 P3.**

The Scene-6 demo recording, as scripted, is **not recordable today**. The headline issue is an env-var name mismatch (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` set in `launch.json` vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` read by the code) that silently disables the mock-auth-bypass path on the SPA. With bypass disabled, the SPA queries the real gateway, which returns an empty approvals queue and an audit feed populated entirely with the gateway's OWN HTTP-request self-traces (verb=`http.request`, worker=`unknown`). The result on `/approvals` is a 0-pending 0-at-risk dashboard; the result on `/audit` is a wall of `http.request · UN` rows with no agent vocabulary visible. If the camera so much as pans across either page in Scene 6 the demo is not credible.

Secondary load-bearing issues: the audit-feed gateway envelope omits `self_hash` / `parent_hash` columns (so the Etherscan-style chain cell never renders on row), there is no hash-chain pop-out modal at all despite the locked design memory promising one, and the `/settings/workspace` page is entirely hardcoded copy with no tenant identity surfaced (so the operator cannot demonstrate "you are scoped to one tenant").

Function-side good news: replay endpoint does return a real cassette envelope with matching `expected_self_hash` / `actual_self_hash` and a usable verdict; the action-detail endpoint does carry `provenance.hash_chain_anchor`; and the action detail UI gracefully falls back to a synthetic-or-placeholder shape under demo mode so the drill-in never 500s.

---

## Headline blockers

1. **F1 [P0] — `/approvals` is empty AND `/audit` shows only HTTP self-traces because the auth-bypass env var name is wrong.** Launch.json sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`; every relevant `is*Bypassed()` helper reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. With bypass disabled, the SPA queries the real gateway, gets `{"items":[]}` for approvals and 27 `http.request` rows for audit, and renders nothing real. The Scene-6 talk track refers to a `publish_post` approval, a Slack interactive card, and a hash-chain audit ring — none of those exist on the page.
2. **F2 [P0] — Audit drill-in lands on `verb: http.request` / `worker: unknown` rows.** Every clickable row in the demo-recording state belongs to the gateway's own self-traced HTTP requests. If the camera clicks any row to demonstrate "real provenance", the destination shows a worker named "unknown", an action_type `audit.detail.viewed`, and an 8-step decision chain with empty bodies — i.e. a skeleton of reasoning with no reasoning actually inside. Theatre-the-camera-sees.
3. **F3 [P0] — `/approvals` has no approve / reject affordance the operator can demo end-to-end.** Even if the env-var were fixed, the gateway has zero seeded approvals and the dashboard's only action is to click a row to drill into audit. There is no inline approve / refuse button on the dashboard rows themselves — the only UI is the row → audit/action drill. Scene 6 requires "Operator clicks Approve" as the central beat. The current `/approvals` page cannot serve that beat.
4. **F4 [P0] — Etherscan-style hash chain is not visible on the audit feed.** The gateway feed envelope (`GET /api/v1/audit`) does not include `self_hash` / `parent_hash` / `sample_hash` columns; `normalizeAuditRow` reads them as undefined; `ActivityRow` then omits the `<HashCell>` entirely. So the row that the runbook calls out for a "≥2-second hash-chain hover" doesn't actually have a hash cell to hover. The locked-design memory `project_audit_page_design_locked` (B3 Etherscan + pop-out modal) is not satisfied on the feed.

---

## All findings

### F1 [P0] — Auth-bypass env-var name mismatch silently disables mock data

**Where**: `web/.claude/launch.json:8` (sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`) vs `web/lib/api/audit.ts:64` + `web/lib/api/audit-detail.ts:63` + `web/lib/api/approvals.ts:75` (read `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`).

**Symptom**: the SPA queries the real gateway for approvals + audit-feed + audit-detail, gets the bare-bones / empty responses that the dev gateway happens to expose today, and renders an inbox with 0 pending across all 3 severity tiers (P0 / P1 / P2). The walk-up demo state is "Approvals · 0 active human decisions" — exactly the opposite of the Scene 6 narration.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/approvals` → `{"items":[]}`
- Open `/approvals` — 5-KPI strip shows 0 / — / 0m / — / — and all severity sections say "No <X> severity approvals waiting."
- Demo banner at top reads "Demo mode — showing sample data" (so the user believes the empty state IS the sample data).

**Suggested fix**: pick a single canonical env-var name. The CLAUDE.md cluster references both names freely; pick the `ARGO_` prefix (code is authoritative) and fix launch.json. Document the rename in CLAUDE.md and any sister demo scripts (`scripts/restart-live-*`).

### F2 [P0] — Audit drill-in shows skeleton-but-empty decision chain on gateway HTTP self-traces

**Where**: gateway returns rows where `summary.worker.name == "unknown"`, `summary.verb == "http.request"`, `summary.action_type == "audit.detail.viewed"`. The `detail.decision_chain` carries the canonical 8-step shape ("Memory loaded" / "Skill chosen" / "Plan candidates" / "Plan selected" / "Policy evaluated" / "Approval" / "Execution" / "Outcome") with all `body: ""` and all `details: { ... : null }`. `web/lib/api/audit-detail.ts:126 normalizeAuditDetail` passes the empty chain through unchanged; `web/components/skycot/audit/DecisionChainCard.tsx` renders 8 rows with the title + an empty body.

**Symptom**: the camera clicks any audit row → lands on a detail page that says "audit.detail.viewed · unknown · 2h ago" with an 8-step plan that looks like a teammate reasoned about an action but said nothing on any of the 8 steps. This is the most dangerous shape of theatre: the structure-of-reasoning is preserved while the substance is null.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/audit/019e63ccb6d27ae187658baf3fdc9e16` and inspect `detail.decision_chain[i].body` — all empty strings.
- Open `/audit/action/019e63ccb6d27ae187658baf3fdc9e16` — DecisionChainCard renders 8 titled-but-empty steps.

**Suggested fix**: either (a) the gateway's self-trace audit kind should NOT use the agent-reasoning decision-chain schema — emit a tighter "HTTP request observed" shape; or (b) the SPA's detail page should detect a fully-empty chain and either suppress DecisionChainCard or render a "This row is a system audit, not a teammate action" badge. Option (b) is the cheaper demo-day fix.

### F3 [P0] — No inline approve / reject affordance in the `/approvals` ops dashboard

**Where**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:297-358 ApprovalRow`. Each row is a `<button>` that navigates to `/audit/action/<actionId>` on click. There is no `Approve` / `Refuse` / `Undo` button on the dashboard itself. The full approve/refuse mutations exist in `web/lib/hooks/use-approvals.ts` (`useSubmitDecision`, `useBatchSubmitDecisions`, `useUndoDecision`) but they're consumed by `/` (the canonical inbox at root) — not by `/approvals`.

**Symptom**: Scene 6 narration is "Operator clicks Approve. Slack POSTs the interactivity callback…" but the demo can't show that click happening anywhere on `/approvals`. The operator would have to navigate to `/` (the root inbox card surface) to find approve buttons — which then renders a different design language (the card-style inbox, not the locked E-ops-dashboard).

**Reproduction**: open `/approvals` in a state with pending approvals (will require fixing F1 first); scroll the 3 severity sections; the only affordance per row is "click → navigate to audit drill-in". No approve / refuse / undo on this page.

**Suggested fix**: either (a) add approve / refuse buttons to each `ApprovalRow` on the ops dashboard (closest to the locked E design memory); or (b) accept that `/approvals` is the "stats view" and Scene 6 should narrate the inbox at `/` for the approve beat. Pick one and update the runbook.

### F4 [P0] — Audit feed gateway envelope omits hash-chain columns; Etherscan cell never renders

**Where**: `crates/argo-gateway` (worktree-only — not in main `crates/`) emits `GET /api/v1/audit` rows with keys `[action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker]` — no `self_hash`, no `parent_hash`, no `sample_hash`. `web/lib/api/audit.ts:194 normalizeAuditRow` reads `gatewayRow.selfHash ?? gatewayRow.self_hash` and falls through to undefined. `web/components/skycot/audit/AuditDayGroup.tsx:80 toActivity` maps undefined to `hash`; `web/components/skycot/ActivityRow.tsx:98` conditionally renders `<HashCell>` only if `activity.hash` is truthy → never renders.

**Symptom**: the audit row visible at `/audit` shows worker initials, verb, timestamp, status pill, status arrow — but NO hash column. The runbook narrative "the audit hash-chain hover in Scene 6 lingers for ≥2 seconds so viewers can read the link" has nothing to hover. The Etherscan-parity promise in the design lock is broken on the feed.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/audit?range=7d | python3 -c "import sys,json;d=json.load(sys.stdin);print(list(d['items'][0].keys()))"` → `['action_type', 'approver', 'channel', 'cost', 'data_classification', 'id', 'status', 'timestamp', 'verb', 'worker']`
- Open `/audit`, inspect any row → no `[data-testid^="activity-row-hash-"]` element present.

**Suggested fix**: the gateway feed envelope should carry the per-row `self_hash` (the value IS computed — `provenance.hash_chain_anchor` is populated on the detail endpoint with `b150c9870ec797dde639783c68dba8d32b9460dbb7994bde79427d0b1770f201` for the test row). Either expose it on the list endpoint too, or back-fill via the detail endpoint client-side. The first option is the right one; the second is a hot-fix.

### F5 [P1] — No hash-chain pop-out modal anywhere; locked design promised one

**Where**: design memory `project_audit_page_design_locked` says B3-enterprise includes a "pop-out modal" for the chain. `web/components/skycot/audit/` has 28 components — none named `ChainModal` / `HashModal` / `HashTooltip` / `ChainPopover`. The hash cell only does click-to-copy via `web/components/skycot/ActivityRow.tsx:225 HashCell`.

**Symptom**: even after F4 is fixed and the row carries a hash, the hover behaviour is just a `title` attribute (browser-native tooltip), not the prev-hash → curr-hash → verification-status modal the design promised. Scene 6 will not have the "look at the cryptographic provenance chain" visual.

**Suggested fix**: build a `<ChainModal>` that shows prev-hash, self-hash, signature, hash_chain_anchor, and a green "verified" / red "tampered" badge derived from a client-side hash recompute. The detail endpoint already exposes `provenance.hash_chain_anchor`. ~80 LOC.

### F6 [P1] — DemoBanner spans full viewport; sidebar overlaps the first 2 words on desktop

**Where**: `web/components/skycot/AppShell.tsx:38` places `<DemoBanner />` at the root of the shell, OUTSIDE the `<main className="md:pl-64">` content column. The banner's max-w-5xl inner container is centered, but the leading edge of the banner runs from x=0 — under the sidebar.

**Symptom**: on the recorded `/approvals` screenshot the banner reads "...mode — showing sample data. Sign in to see your own workspace." with "Demo " hidden behind the SKYCOT logo block. Camera-visible polish bug. Visible on every (app)-group page when the banner is undismissed.

**Reproduction**: open any (app) route in a fresh browser (no `skycot:demo-banner-dismissed` in localStorage) at desktop width ≥768px; observe the banner text starts under the sidebar.

**Suggested fix**: either move `<DemoBanner />` INSIDE `<main>` (after the WideRailStyles, before children), or add `md:pl-64` to the banner's outer div.

### F7 [P1] — `/settings/workspace` is entirely hardcoded; no tenant identity surfaced anywhere

**Where**: `web/app/(app)/settings/workspace/page.tsx` is a static React component. Display name = "SKYCOT Demo", vertical = "Marketing operations", timezone = "Australia/Melbourne", billing contact = "owner@skycot.example" — all baked in. `useSkycotSession()` does expose `tenantId` (e.g. `tenant_acmeco` in demo mode) but it's not consumed anywhere on this page or anywhere else customer-facing.

**Symptom**: the operator cannot demonstrate "I am scoped to one tenant". There is no `/settings/tenants`, no tenant switcher, no tenant id visible in the user pill or workspace settings. Claim-6's multi-tenant ergonomics are invisible to the end user.

**Reproduction**: open `/settings/workspace`; observe nothing changes if you swap tenants via cookie / session; no tenant id displayed.

**Suggested fix**: surface `session.tenantId` in the workspace settings page as a read-only field ("Workspace ID · tenant_acmeco · copy"), AND in the `<UserPill>` popover. ~30 LOC.

### F8 [P1] — Gateway `as_tenant=<other>` does not require capability check in current dev mode

**Where**: capability `argo-capability` defines a capability for "cross-tenant audit feed override" (`crates/argo-capability/src/lib.rs:39-40`), but the dev gateway honors `?as_tenant=<x>` without checking any header — returns 200 OK with the other tenant's data shape (empty 2-row list for `as_tenant=other_tenant`).

**Symptom**: this is fine for dev convenience but is a P1 demo-day risk if someone tries to demonstrate "see, cross-tenant is refused" — the gateway will instead silently scope to the other tenant's data, leaving the operator looking at someone else's audit feed (currently empty but architecturally not denied). The Claim-6 cross-tenant negative test (the actual claim_6_live_cross_tenant rust test) is what's load-bearing; this is the gateway's HTTP surface diverging from the substrate.

**Reproduction**:
- `curl "http://127.0.0.1:7400/api/v1/audit?range=7d"` → 27 rows for default tenant
- `curl "http://127.0.0.1:7400/api/v1/audit?range=7d&as_tenant=other_tenant"` → 200 OK with 2 rows (the other tenant's self-traces)
- Expected: 403 Forbidden unless the calling principal has the cross-tenant override capability.

**Suggested fix**: turn on the capability gate for the `?as_tenant=` override path on the live gateway. Without auth context in dev, this would have to default to "ALLOW with a clearly-visible audit row written", or "DENY with a flag to enable" — either is fine, just don't silently let it through.

### F9 [P2] — `/audit` feed empty-state ("Quiet day") doesn't differentiate signed-out vs no-activity

**Where**: `web/app/(app)/audit/page.tsx:397 AuditEmpty` shows "Quiet day · Worker actions and approvals will appear here as they happen" when there are 0 rows + no active filters. This is the same copy whether the operator is signed out, the gateway is offline, the tenant is fresh, or it's literally a quiet hour.

**Symptom**: if the env-var fix (F1) doesn't seed agent activity but does deliver an empty queue, Scene 6 will narrate "the audit timeline" against a "Quiet day" screen — flat and uninformative.

**Suggested fix**: differentiate "0 rows because of empty filter set" from "0 rows because of fresh workspace" from "0 rows because gateway down". Use the network response shape + dataSource hint to pick copy.

### F10 [P2] — Audit row arrows + "Succeeded" pills on http.request self-traces are misleading

**Where**: every self-trace row in the gateway feed has `status: "succeeded"` (because the HTTP request itself returned 200). `web/components/skycot/audit/AuditDayGroup.tsx:STATUS_MAP` maps `succeeded` to the green pill.

**Symptom**: combined with F2, the operator (or buyer) reading `/audit` sees a wall of green "Succeeded · http.request · 2h ago" rows. If the camera dwells, the visual narrative is "everything is green and nothing is happening" which is the worst-case version of "your trust dashboard".

**Suggested fix**: tag self-trace `audit.detail.viewed` / `audit.feed.queried` / `audit.replay` action_types as `system` rows and hide them from the default feed (with a filter chip to surface them). The runbook narrative implies the audit is the agent's actions, not the gateway's HTTP introspection on itself.

### F11 [P2] — Approvals dashboard sample action-id map is non-deterministic against live data

**Where**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:26-35 SAMPLE_APPROVAL_ACTION_IDS` hard-codes a map from `ap_01H9Z7` → `act_018f67d0-0000-7000-8000-000000000001_018`. Under live gateway state these action ids do not exist in the audit DB (verified: `curl /api/v1/audit/act_018f67d0...` → 404).

**Symptom**: even if the env-var were swapped to the bypass-on branch and the SAMPLE_APPROVALS rendered, clicking through to the action drill-in would 404 in strict-prod and fall through to `placeholderAuditDetail` in demo (which renders "Action detail unavailable — drill into another row for full provenance"). Theatre: the camera sees a fabricated "detail unavailable" page where it expected real provenance.

**Suggested fix**: the SAMPLE_APPROVAL rows should reference action ids that DO exist in SAMPLE_AUDIT_ROWS (the 5 hand-shaped Priya/Peta/Mariah/Jamal/Nori details). Audit the mapping; align ids. ~10 LOC.

### F12 [P2] — `/audit` "PII redacted — your role shows masked data" banner shows on every demo session

**Where**: `web/components/skycot/audit/RedactionBanner.tsx` (rendered conditionally in `web/app/(app)/audit/page.tsx:205-211`). In demo mode the viewer policy lacks the categories `confidential` etc, so the banner shows ALL the time.

**Symptom**: the recorded `/audit` screenshot shows a permanent amber "PII redacted — your role shows masked data · Request elevated view →" banner. For an investor demo, this is confusing: "wait, what's masked? are we missing the demo?". It's also a footgun: the camera will dwell on it and the viewer will wonder what they're not seeing.

**Suggested fix**: in demo mode, default the viewer policy to "all permissions granted" (the demo operator is the workspace owner) so the redaction banner suppresses. Or: drop the banner on the first 60s of a demo session via localStorage flag.

### F13 [P3] — Hash-chain rows say "Self-hash · click to copy" — operator-facing copy reads engineer-y

**Where**: `web/components/skycot/ActivityRow.tsx:232` builds the tooltip title as `${label} · click to copy\n${hash}` where label is "Self-hash" or "Sample row — not part of the production audit chain".

**Symptom**: in a buyer-facing demo, "Self-hash" is jargon. The Etherscan analogue calls it "transaction hash" or "block hash". For a non-blockchain enterprise audit surface, "Action hash" or "Audit row ID" reads cleaner. Low-leverage cosmetic.

### F14 [P3] — Approvals KPI strip leaves "Active reviewers" as a permanent dash

**Where**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:147-150 — Active reviewers: "—" / "reviewer roster pending"`.

**Symptom**: the 5th KPI tile is hardcoded to display "—" forever ("reviewer roster pending"). On a 5-KPI strip ops dashboard, one perma-dash tile looks broken. Buyer asks "is this loading?".

**Suggested fix**: either ship the underlying reviewer-roster aggregator now, or drop the tile to a 4-KPI strip until it can be wired. The visible "pending" creates the impression the dashboard is half-built.

### F15 [P2] — Auth-bypass DOC drift: every API client comment says "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1"; launch.json sets the other name

**Where**: same files as F1. Each one has documentation comments at the top explicitly referencing `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` while the dev-config sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`. Doc-vs-config drift is one of the patterns flagged in `docs/skycot/DRIFT_PREVENTION.md`. This is F1 looked at as a doc-hygiene issue separately.

**Suggested fix**: once F1 is resolved by picking a canonical name, grep the codebase for the loser and fix all references in one pass.

### F16 [P1] — `/audit/action/<id>/replay` endpoint works AND returns matching hashes, but the SPA replay page doesn't render the verdict

**Where**: `curl -X POST /api/v1/audit/<id>/replay` returns `{"actual_self_hash":"...","expected_self_hash":"...","verdict":"same"}` with matching hashes — function-side this WORKS, hash-chain replay is real not theatre. But `web/lib/api/replay.ts:fetchReplay` parses the gateway envelope into a `ReplayOutcome` and the SPA `/replay` page renders one of the four outcome views. The replay endpoint shape returned by the gateway is `{actual_self_hash, audit_id, created_at_micros, diff, expected_self_hash, kind, kind_label_known, replayed_at, seq, session_id, verdict}` — which does NOT match the SPA-side `ReplayResult` interface shape (`{originalOutput, replayedOutput, provenanceDiff}`). I didn't test the rendering because the preview server kept dying during navigation, but the schema mismatch will either fall back to a "failed" view or to "drifted" — neither matches the actual `verdict: "same"` returned by the gateway.

**Suggested fix**: align the gateway replay envelope to the SPA contract OR add a normalizer in `replay.ts` that maps the existing envelope (verdict "same" → verdict "reproducible", recompute provenanceDiff = all-unchanged) to the SPA shape. Verify by opening `/audit/action/019e63ccb6d27ae187658baf3fdc9e16/replay` and confirming the "Reproducible" verdict banner renders, not the failure view.

---

## Resolved since last audit

Previous audit at `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md` flagged 7 P0s. Cross-checked at this run:

- "Approvals page empty-state crashes" — RESOLVED. `fix-approvals-inbox-waiting-on-you-fixture-drift` removed the empty-state short-circuit and the ops dashboard renders 0-state KPIs cleanly. Banner header in the page file documents this.
- "Audit drill-in 500s on unknown action id" — RESOLVED. `placeholderAuditDetail` fallback shipped. Detail page now degrades gracefully under demo mode.
- "Replay page hard-fabricates 'Policy regression detected' for unknown ids" — RESOLVED per `web/lib/api/replay.ts:8` comment: unknown ids now return `{ kind: "failed" }` and render the failed view, not a synthesised forensic report.

---

## Orientation score: 56 / 100

The trust surface architecture is sound: the data shapes are real, replay returns a real cassette envelope, hashes ARE chained server-side, the redaction system is wired, the multi-tenant scoping happens at the SQL layer per the Claim-6 substrate tests. The PRESENTATION layer has six demo-blocking defects, four of which are camera-visible inside Scene 6's natural shot path. With the env-var rename (F1) + the audit feed hash-column expansion (F4) + the http.request row hiding (F10) + a 30-line tenant identity surface (F7), this becomes a recordable scene. As shipped today it is not.

**Function-vs-dressing application**: F1, F2, F3, F4, F11 are FUNCTION-side issues (the camera drills into them and sees nulls). F6, F7, F12, F13, F14 are DRESSING-side (camera dwells on them but the underlying claim is intact). Severity-laddered accordingly: function-side issues over the demo path are P0, dressing-side issues are P1/P2 unless the dressing is the headline (F6 is P1 only because the banner truncation IS camera-visible on every demo session).
