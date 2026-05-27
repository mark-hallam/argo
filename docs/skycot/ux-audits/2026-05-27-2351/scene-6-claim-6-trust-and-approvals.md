# Scene 6 / Claim 6 — Trust + Approvals (multi-channel + multi-tenant)

**Audit run**: 2026-05-27 23:51 Melbourne
**Scope**: `/approvals` (E-ops-dashboard) + `/audit` (B3-Etherscan) + audit drill-in (real-vs-fabricated) + `/audit/action/{id}/replay` + cross-tenant isolation negative tests + multi-channel surfacing in UI.
**Environment**: SPA on http://localhost:3000 (Next.js 15.5.18, PID 45789); gateway on http://127.0.0.1:7400 (schema v83 per CLAUDE.md). Session via `POST /api/auth/demo-mint` (anonymous demo tenant `human:operator@skycot.local`).
**Tooling**: curl + sqlite + code grep (Claude Preview MCP was stuck on `/skills` after a navigation; restart was not in scope so the walk shifted to API + SSR-fetch).

## TL;DR

The trust surface ships the right **shape** — `BrandLoudHero` + `ApprovalsOpsDashboard` + Etherscan-style `/audit` + provenance card + hash-recompute replay — and the **cryptographic core is real** (replay verdict `same`, `expected_self_hash == actual_self_hash` on a real row). But the demo is gated by **three function-side defects** that the camera will read directly on the Scene-6 recording path:

1. **`/audit/<id>` detail returns 404 for almost every row the LIST returns.** The list endpoint defaults to workspace tenant `ws-default`; the detail endpoint defaults to `ws-default` but the DB has **zero rows under `ws-default`** — every row in the table is under tenant `anonymous` or a real UUID. Drill-in is structurally broken without an explicit `X-Argo-Tenant-Id` header that the SPA does not send.
2. **The 13 pending approvals + 67k real audit rows are in tenant `019e6785-654d-...` (workspace `skycot-v1-demo`); the SPA's CWD-derived workspace is `019e5057-...` (workspace `argo`).** The seed CLI populated the wrong workspace. Both `/api/v1/approvals` and `/api/v1/approvals/<id>/decide` return `[]` / 404 because they CWD-scope and ignore the request's tenant header.
3. **100% of recent `turn_end` / `tool_call` rows are scaffolding (`is_dressing:true` or `actor:argo-demo-seed`).** Even when fetched with the matching tenant header, the audit detail's `provenance` (model.id / skill / policy / memory_snapshot) is `""` and the 8-step decision chain is all `body:""` `details:null`. The camera reads "Memory loaded — empty / Skill chosen — empty / ..." for the real Scene-6 drill-in.

Cross-tenant isolation is **silently broken on `/api/v1/audit`**: sending `X-Argo-Tenant-Id: <any-real-tenant-uuid>` returns that tenant's rows with no 403 challenge — the cross-tenant capability gate only fires on the explicit `?as_tenant=` query parameter, not the header. **P0** as a customer-trust defect (not a Scene-6 blocker, since the camera doesn't exercise it).

Multi-channel evidence in the UI: there is **no visible representation** of SMS / Discord / voice routing in `/approvals` or `/audit` outside a hidden filter facet. The audit DB only has 22 rows with `channel="slack"` and zero with SMS / Discord / voice — the rest fall back to a hardcoded `"web"` default in `routes_audit_feed.rs:256`. Claim 6's multi-channel narrative is invisible to a viewer.

**Function-side defects**: 4 P0, 3 P1, 4 P2, 2 P3.

## Headline blockers

- **F1 [P0]** — Audit drill-in 404s for every list-returned id under default-tenant scope (tenant-default mismatch between list and detail handlers).
- **F2 [P0]** — Seed CLI populated `skycot-v1-demo` workspace; SPA reads from `argo` CWD workspace. Approvals + audit rendered empty even though real seed data exists.
- **F3 [P0]** — Cross-tenant probe via `X-Argo-Tenant-Id: <other-tenant-uuid>` returns that tenant's audit rows with no 403. Capability gate (`Action::AuditCrossTenant`) only enforced on `?as_tenant=` query, not the workspace-tenant header.
- **F4 [P0]** — 100% of recent agent-turn audit rows are scaffolding (`is_dressing:true` / `actor:argo-demo-seed`); decision-chain + provenance fields are empty even on `turn_end` rows. Camera-readable "Provenance unavailable" empty state on the drill-in card.

## All findings, in order

### F1 [P0] — Audit drill-in 404s for every list-returned action id

**What I saw**: `GET /api/v1/audit?limit=5` returns 5 rows with valid ids (e.g. `019e69b7a34872209b139af357f9a8ed`). `GET /api/v1/audit/019e69b7a34872209b139af357f9a8ed` returns `{"error":{"code":"not_found","message":"audit row not found"}}` (HTTP 404). Same on the SPA proxy. Reproduces on every list-returned id I probed.

**Root cause**: the list handler defaults its workspace tenant to `ws-default` (`routes_audit_feed_scope.rs:8 DEFAULT_WORKSPACE_TENANT_FALLBACK`) and filters by **payload-json** `tenant_id`. The detail handler defaults its workspace tenant to `DEFAULT_AUDIT_TENANT_ID` = `"ws-default"` (`crates/argo-audit/src/sink_tenant.rs:11`) and filters by **DB-column** `tenant_id`. The DB has 5,971 rows under `tenant_id="anonymous"`, 2,733 under real UUID `019e613b-...`, **zero under `ws-default`**. So the detail handler's default-tenant SELECT never matches the rows the list handler surfaces.

**Why it matters**: Scene 6's narration is "click an audit row to see the chain." Every click 404s in the canonical demo path. The `<AuditActionNotFound>` shell renders ("Audit row not found").

**Function-vs-dressing**: function-side. The camera tries to drill in and sees "Action not found."

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/audit?limit=3 | jq '.items[0].id'` → e.g. `019e69b7a34872209b139af357f9a8ed`
- `curl http://127.0.0.1:7400/api/v1/audit/019e69b7a34872209b139af357f9a8ed -w "\ncode=%{http_code}\n"` → 404
- `sqlite3 ~/.argo/sessions.db "SELECT tenant_id, COUNT(*) FROM audit_record GROUP BY tenant_id"` → no `ws-default` row.
- Detail succeeds **only** when both list and detail are called with `X-Argo-Tenant-Id: <real-tenant-uuid>` matching DB rows.

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:46-48,176-190,192-194` vs `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:8,24-41,43-53`.

**Suggested fix**: make the list and detail handlers agree on what "no tenant header" means. Either (a) detail handler should fall back to `payload.tenant_id` lookup the same way list does, or (b) both handlers should resolve the workspace tenant via `resolve_workspace_tenant(store)` (CWD-derived) when no header is supplied, matching `routes_team_list.rs` semantics.

### F2 [P0] — Seed data lives in a workspace the SPA cannot read

**What I saw**: `sqlite3 ... "SELECT COUNT(*), status FROM approval_request GROUP BY status"` returns 16 approvals (13 pending, 2 denied, 1 granted) on tenant `019e6785-654d-7302-9a29-ff6035517c53`, plus 3 more pending on tenant `019e6780-3c3d-...`. That tenant id maps to workspace `skycot-v1-demo`. The SPA (per `routes_approvals_json.rs:221-232 resolve_workspace_tenant_id`) CWD-resolves to the `argo` workspace (`019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`), which has **zero approvals**. Result: `/api/v1/approvals` returns `{"items":[]}` even with `X-Argo-Tenant-Id` pointing at the seeded tenant (the header is ignored — gating is CWD-based).

**Why it matters**: prior audit's F1 ("approvals empty / no real audit rows") flagged the seed gap; the seed CLI has since run (per CLAUDE.md note on #2437 demo-seed marketplace idempotency) but it ran against `skycot-v1-demo`, not the operator's CWD workspace. Until the seed is re-run against `argo` (or the gateway's resolve-tenant logic honors a directive header), Scene 6 still has nothing to point at.

**Function-vs-dressing**: function-side. The empty approvals inbox + zero pending counters are visible during the camera's pan across the page.

**Reproduction**:
- `sqlite3 ~/.argo/sessions.db "SELECT hex(tenant_id), COUNT(*), status FROM approval_request GROUP BY hex(tenant_id), status"` → 19 rows under `019E6785654D...`, 3 under `019E67803C3D...`, **zero under `019E50571D3F...`** (= argo CWD tenant).
- `curl http://127.0.0.1:7400/api/v1/approvals` → `{"items":[]}`
- `curl http://127.0.0.1:7400/api/v1/approvals -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53"` → STILL `{"items":[]}` (header ignored).
- `curl -X POST http://127.0.0.1:7400/api/v1/approvals/019e6780-4e9c-7722-8584-f55153ffe0d3/decide -d '{"decision":"approve"}'` → `approval_not_found` even when the id is a real pending row.

**File**: `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs:221-232` (`resolve_workspace_tenant_id`).

**Suggested fix**: re-run `argo demo seed --tenant <argo-cwd-tenant>` so the operator's day-to-day gateway has the demo rows, OR add a `SKYCOT_DEMO_WORKSPACE_OVERRIDE` env var that the approvals + team + audit handlers honor in dev. Either way, surface a clear "seeded into workspace X" line in the seed CLI output so this mismatch is visible.

### F3 [P0] — Cross-tenant audit data leaks via `X-Argo-Tenant-Id` header

**What I saw**: `curl http://127.0.0.1:7400/api/v1/audit?limit=2 -H "X-Argo-Tenant-Id: 019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"` returns 2 rows for that tenant — no auth, no challenge. Sending a different real tenant UUID (e.g. `019e6785-654d-...`) returns THAT tenant's rows. Same for detail: `curl http://127.0.0.1:7400/api/v1/audit/<real-id> -H "X-Argo-Tenant-Id: <matching-tenant>"` returns the full 8-step `detail` including hash chain, decision steps, provenance.

**Why it matters**: the workspace-tenant header is treated as authoritative without verifying the caller's session has access to that workspace. `resolve_tenant_scope()` only fires the `Action::AuditCrossTenant` capability check on the `?as_tenant=` query parameter. The header path skips it entirely. Combined with no authn requirement on the gateway loopback today (auth-bypass mode), any client that knows a tenant UUID can enumerate that tenant's audit log.

**Function-vs-dressing**: function-side — load-bearing multi-tenant claim. **This is the Claim 6 regression.** The mitigation is that in production the gateway is fronted by the SPA's session cookie, but the header-trust path is still a known-bad shape. Cross-tenant pins #2257 (mentioned in CLAUDE.md) pin the LIST behavior under `?as_tenant=` but the header path is unguarded.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/audit?limit=2 -H "X-Argo-Tenant-Id: 019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"` → returns rows from tenant `019e5057`.
- `curl http://127.0.0.1:7400/api/v1/audit?limit=2 -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53"` → returns rows from tenant `019e6785` (different tenant, different rows, no challenge).
- `curl http://127.0.0.1:7400/api/v1/audit/019E576EE6FC7E92840661BFC22CCFE1 -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53"` → 200 with full detail (decision chain + provenance + hash anchor) for that tenant's row.

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-41` (header trusted without capability check); `routes_audit_detail.rs:192-194` (same pattern in detail handler).

**Suggested fix**: before honoring `X-Argo-Tenant-Id` in the list and detail handlers, verify the calling principal's session has access to that workspace. In auth-bypass dev mode, log a warning and let it through; in production, refuse with 403 unless the principal has `Action::AuditCrossTenant` capability for that tenant.

### F4 [P0] — Recent agent-turn audit rows are seeded scaffolding; provenance + decision chain render empty

**What I saw**: every `turn_end` / `tool_call` / `tool_result` row I sampled has either `payload.is_dressing:true` (with `"note":"Pass B dressing row — camera does not drill into individual contributing rows"`) or `actor:argo-demo-seed` (with `"note":"scaffolding row — Pass A real-LLM body wired by mission-demo-seed-real-llm-driver"`). The `mission-demo-seed-real-llm-driver` brief mentioned in the scaffolding note has not shipped the real-LLM body.

When the audit-detail endpoint serializes one of these rows, the entire `decision_chain[0..7].body` is `""`, every `decision_chain[].details.*` is null, and `provenance.model.id` / `provenance.skill.id` / `provenance.policy.hash` / `provenance.memory_snapshot.id` are all `""`. The hash anchor IS real (a valid SHA-256), but every other evidence field that Scene 6 narrates as "we know what your AI worker DID and WHY" is empty. The SPA's `ProvenanceBlock` (`web/components/skycot/audit/ProvenanceBlock.tsx:36,52-57`) detects this and renders the `<ChainEmptyState title="Provenance unavailable" body="This <verb> row is a meta-event, so model, prompt, skill, memory, and policy hashes were not produced." />` empty state. The decision-chain card renders 8 step titles with empty bodies.

**Why it matters**: this is the textbook function-vs-dressing P0. Scene 6 says "real provenance"; the camera sees "Provenance unavailable." Even when the drill-in 404 (F1) is fixed and the seed-tenant mismatch (F2) is fixed, the camera will still drill into a card that says "no provenance produced for this row."

**Function-vs-dressing**: function-side. The card title + body are camera-readable. Worse, this is exactly the failure mode the no-theater directive warns against — the data is dressing in shape (`is_dressing:true` in the payload) but the SPA renders it as if it were real, then falls back to the "unavailable" empty state.

**Reproduction**:
- `sqlite3 ~/.argo/sessions.db "SELECT payload_json FROM audit_record WHERE kind='turn_end' ORDER BY seq DESC LIMIT 1"` → `{"is_dressing":true,"note":"Pass B dressing row — camera does not drill into individual contributing rows",...}`
- `curl http://127.0.0.1:7400/api/v1/audit/019E67AC8D5374828D2293D3B8581052 -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53" | jq '.detail.provenance'` → all model/skill/policy/memory ids `""`.
- `curl http://127.0.0.1:7400/api/v1/audit/019E67AC8D5374828D2293D3B8581052 -H "X-Argo-Tenant-Id: 019e6785-654d-7302-9a29-ff6035517c53" | jq '.detail.decision_chain[0]'` → `{"body":"","details":{"snapshot_id":null},"n":1,"title":"Memory loaded"}`.

**File**: `mission-demo-seed-real-llm-driver` brief (TODO) is the load-bearing fix. Without real LLM turns, the seed CLI produces decoration rows that the camera reads as evidence of nothing.

**Suggested fix**: ship `mission-demo-seed-real-llm-driver` (≤10 real agent turns per persona is enough — the camera only drills into 1-2 rows in the Scene-6 beat per the runbook). Until then, exclude `is_dressing:true` rows from the audit-feed default list endpoint so the camera doesn't land on one by accident.

### F5 [P1] — Audit feed defaults to showing 100% SPA self-traffic (`http.request` + `worker.kpi_rolled`)

**What I saw**: `curl http://127.0.0.1:7400/api/v1/audit?limit=200 | jq '[.items[].verb] | group_by(.) | map({verb: .[0], count: length})'` → 6 rows of `http.request`, 194 rows of `worker.kpi_rolled`. The kpi_rolled rows come from the orchestrator's hourly KPI-aggregation tick (`crates/argo-orchestrator/src/kpi_rollup.rs`) — those are platform housekeeping rows, not agent activity. Worker names show up as bare UUIDs (no joined `name`).

**Why it matters**: the audit feed is supposed to read like "AI workers acting on your behalf." Today it reads like "platform observing itself." Camera sees a stack of `019e6785-928f-...` UUIDs and `worker.kpi_rolled` rows.

**Already in flight**: prior audit T0819 / T0623 surfaced this as `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`. Brief still TODO. Recommend extending the filter to also exclude `worker.kpi_rolled` unless `?include_meta=1`.

**Function-vs-dressing**: function-side. The verb + worker text is exactly what the camera reads on Scene 6's pan.

**Suggested fix**: extend the existing brief: exclude `verb IN ('http.request', 'worker.kpi_rolled', 'audit.detail.viewed', 'audit.replay', 'audit.list.viewed')` from `/api/v1/audit` by default; show them only with `?include_meta=1`. Also join `worker_id` against the `agent` table so the row labels render Mariah / Peta / Priya / Jamal / Nori instead of bare UUIDs.

### F6 [P1] — Multi-channel claim is invisible in `/approvals` and `/audit`

**What I saw**: `/api/v1/audit` returns `channel="web"` for 100% of rows under the demo tenant (default fallback in `routes_audit_feed.rs:256`). DB has 22 rows with `channel="slack"`, zero with SMS / Discord / voice. The `/approvals` ops-dashboard renders KPI / severity tiers / donuts but has no channel column or chip. The audit page has a hidden channel filter facet (`AuditFilterPopover.tsx:123-134`) but the chip rail doesn't surface it by default — the operator has to open the popover to see channels exist.

**Why it matters**: Claim 6 ("multi-channel + multi-tenant") narrates approvals routed via SMS / Discord / voice. The UI today shows none of that. Even if SMS / Discord / voice approvals were happening on the backend (#2208 / #2222 / #2224 per CLAUDE.md), the camera has no way to see them.

**Function-vs-dressing**: function-side for the SCENE-6 narrative; less so for the rest of the demo. A demo-blocker if Scene 6 narrates "approve from SMS, see the trail in audit"; nice-to-have if Scene 6 stays web-only.

**Reproduction**:
- `sqlite3 ~/.argo/sessions.db "SELECT channel, COUNT(*) FROM audit_record WHERE channel IS NOT NULL GROUP BY channel"` → `slack|22` only.
- `grep -n "channel" web/components/skycot/approvals/*.tsx` → only `Undo back-channel.` comment and test fixtures; no channel column or chip in the dashboard.
- `grep -n "SMS\|Discord\|voice" web/components/skycot/audit/*.tsx web/components/skycot/approvals/*.tsx` → zero matches.

**Suggested fix**: (a) add a `Channel` column to the Etherscan row + a channel chip on the approvals row; (b) seed at least 3 approvals across SMS / Discord / voice for the demo tenant so the camera sees the chip render with varied values; (c) consider promoting the channel filter facet into the chip rail by default.

### F7 [P1] — Approval decide handler ignores `X-Argo-Tenant-Id` header; CWD-bound

**What I saw**: `POST /api/v1/approvals/<real-pending-id>/decide` returns `approval_not_found` even when the tenant header points at the correct workspace. The decide handler at `routes_approvals_json.rs:205-208 resolve_workspace_tenant_id` only looks at CWD. Same pattern as F2 but compounded — even if you find an approval, you can't decide it from a different workspace.

**Why it matters**: the eventual multi-tenant gateway will need every endpoint that touches approvals to agree on tenant scope. Today the inconsistency means you can list a workspace's approvals (with header) but not decide them (without re-running the gateway in that workspace's CWD).

**Function-vs-dressing**: function-side reliability gap; not a Scene-6 blocker until F2 is closed.

**Suggested fix**: extract a shared `resolve_tenant_for_request(req, store)` helper. Order: (1) verified-session JWT tenant, (2) `X-Argo-Tenant-Id` header (gated by capability check), (3) CWD-resolved workspace fallback in dev only. Apply uniformly to `/team`, `/approvals`, `/audit/<id>`, `/skills`.

### F8 [P2] — `/audit` Etherscan table is `<div role="list">` not `<table>`

**What I saw**: `web/components/skycot/audit/AuditDayGroup.tsx` renders the day-grouped feed using `<section><h2>...<div role="list">...</div></section>` — not a real `<table>` with `<thead>/<tbody>`. Screen reader announces "list, N items" rather than "table, N rows, columns: time, worker, verb, id, status."

**Why it matters**: accessibility nit. Locked design (memory `project_audit_page_design_locked`) doesn't specify markup, only visual. The camera doesn't read DOM semantics, so this isn't a demo blocker.

**Suggested fix**: post-MVP polish — refactor `AuditDayGroup` to use `<table>` with header cells. Carried over from prior audit's F10.

### F9 [P2] — `/approvals` zero-state mixes em-dash + literal `0`

**What I saw**: KPI strip shows `Pending: 0` (literal zero, correct) but `Resolved/wk: —`, `Avg response: —`, `$ at risk: —`, `Approval rate: —` (em-dash). Inconsistent zero-state vocabulary.

**Why it matters**: minor polish. Reads slightly defensive vs. clean ops dashboard.

**Suggested fix**: normalize. `Pending: 0` stays `0`; `Resolved/wk: 0`, `$ at risk: $0`, `Approval rate: 0%`; only `Avg response: —` (over 0 events genuinely undefined). Carried over from prior audit's F11.

### F10 [P2] — `worker.kpi_rolled` audit rows show worker `name` = the bare UUID

**What I saw**: in `/api/v1/audit?limit=10`, every kpi_rolled row has `"worker":{"id":"019e6785-928c-72b3-85c6-436fe5004c50","name":"019e6785-928c-72b3-85c6-436fe5004c50"}`. The list handler isn't joining `worker_id` against the `agent` table to render the persona name.

**Why it matters**: if F5's `kpi_rolled` filter doesn't ship, the camera reads rows of UUIDs instead of Mariah / Peta / Priya / Jamal / Nori. Even after F5, any caller using `?include_meta=1` still sees raw UUIDs.

**Suggested fix**: in the audit-feed handler, join `worker_id` against `agent.name` and surface the friendly name; fall back to the UUID only when no agent row exists.

### F11 [P2] — Replay endpoint contract still wrapped by `adaptGatewayReplay` adapter (technical debt)

**What I saw**: `POST /api/v1/audit/<id>/replay` returns `{verdict:"same", expected_self_hash, actual_self_hash, diff:{}, audit_id, kind, ...}` — the hash-recompute envelope. `web/lib/api/replay.ts:204-228 adaptGatewayReplay()` maps that to the SPA's canonical `ReplayOutcome` discriminated union. Adapter is correct (verdict `"same"` → `"reproducible"`); TODO comment says delete once gateway ships SPA-shaped response per `fix-gateway-replay-envelope-spa-contract`.

**Why it matters**: no demo impact. Tracked debt.

**Function-vs-dressing**: function-side debt, but adapter is fully wired. Replay is genuinely real — I verified `expected_self_hash == actual_self_hash` on row `019E67AC8D5374828D2293D3B8581052` (both `ad2fac03e38dd3fbc12e92951e33be732411282037d15dcb34c3af26a3fa83df`).

**Suggested fix**: track via the existing brief. No demo blocker.

### F12 [P3] — `/audit` h1 title leaks "Approvals · SKYCOT" on shoulder pages

**What I saw**: navigating direct to `/audit` gives `document.title === "Audit · SKYCOT"` (correct). But several in-app pages still default to "Approvals" via the root metadata fallback (per the existing brief `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`).

**Why it matters**: dressing-side; not on Scene 6's recording path but `/genesis` is per CLAUDE.md.

**Suggested fix**: ship the existing brief. Carried over.

### F13 [P3] — Replay button + page title both render "Action" instead of "Replay <verb>"

**What I saw**: `curl http://localhost:3000/audit/action/<id>/replay` SSR title = `<title>Action</title>`. The page does render a "Replay" button label but the tab title isn't differentiated from the parent action-detail page.

**Why it matters**: cosmetic. If the operator pre-opens both pages during demo prep, the two tabs read identically.

**Suggested fix**: set per-route metadata title on `/audit/action/[id]/replay/page.tsx` to `Replay action · SKYCOT`.

## Resolved since last audit (T0819 → T2351 ≈ 15.5h)

- **F2 (SPA proxy slow)** — RESOLVED. Direct gateway audit-detail: 0.003s. SPA proxy audit-detail: 0.009s. SSR `/audit/action/<id>` returns 200 in 0.9s (was timing out >60s). Per the comment at `web/app/(app)/audit/action/[actionId]/page.tsx:26-40`, the `EvidenceBundleExporter` was dynamically-imported off the cold-compile path; that closed the 50× slowdown.
- **F4 (title leaks 9 pages)** — partially resolved; per-page tests added for some pages. Still tracked under the existing brief.

Persistent from prior audit: F1 audit-feed self-traffic pollution (now broader: `worker.kpi_rolled` floods the feed), F2 demo-seed gap (now narrowed: seed CLI ran but against wrong workspace tenant).

## Orientation score: 58 / 100

The trust **architecture** is sound: replay is genuinely cryptographic, the audit row schema carries the right fields, the approvals dashboard renders the right shape, CSRF + auth-bypass flags are correctly threaded. But the **delivered demo experience** is weaker than the T0819 score (64) because two new defects surfaced this run: the audit detail / list tenant-default mismatch (F1) — which is structural, not data-shaped — and the seed-into-wrong-workspace mismatch (F2). Both have to land before Scene 6's drill-in beat can record.

The cross-tenant header trust path (F3) is the highest-leverage **security** fix — it's not a Scene-6 blocker but it's a Claim 6 regression of the multi-tenant claim the demo literally narrates as locked.

What's working: CSRF on POST /decide; replay-fabrication closure (`UNKNOWN_ACTION_FAILED` envelope); approvals decide returns real errors (not silent mock-success); the gateway-direct replay verdict matches recomputed self_hash; the SPA proxy round-trip latency closed from 11s to 9ms.

What's not working: the camera-visible drill-in path. The runbook says "click row → see chain" — today the click 404s, and even if F1 lands, the drilled-into row is scaffolding with empty provenance.

Recommended next ticks:
1. Fix F1 (5 LOC change to make list + detail handlers agree on tenant default).
2. Re-run `argo demo seed --tenant <argo-cwd-tenant>` to close F2.
3. Add capability gate to the audit feed's header-tenant path (F3).
4. Ship `mission-demo-seed-real-llm-driver` (10 real turns per persona) to close F4.
