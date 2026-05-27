# Scene 6 / Claim 6 — Trust + Approvals (multi-channel + multi-tenant)

**Audit run**: 2026-05-27 08:19 Melbourne
**Scope**: `/approvals` (E-ops-dashboard) + `/audit` (B3-Etherscan) + hash-chain detail + audit-row drill-in (real-vs-fabricated test) + cross-tenant isolation negative tests (direct gateway calls) + `/audit/action/{id}/replay` fabrication check.
**Environment**: web dev server on http://localhost:3000, gateway alive on http://127.0.0.1:7400 (schema v83), demo mode (anonymous tenant lane).
**Tooling**: Claude Preview MCP for browser drive (with intermittent stalls — fell back to curl + sqlite + code grep where preview was unresponsive).

## TL;DR

Trust story is mostly **real** at the function side — the audit hash chain genuinely recomputes (replay verdict `same` with matching `self_hash`), tenant scoping enforces 403 on `/api/v1/team?tenant_id=<other>`, and the approve / decide endpoints route through real gateway handlers (no client-side fabrication). Replay-fabrication is closed (the `findSampleReplay` table + `failed` envelope for unknown ids ships per the E5-d brief).

**However**, two camera-readable theatre issues remain on Scene 6's recording path:
1. **The audit feed shows 100% SPA self-traffic** (`verb=http.request` from worker "UN" / "unknown"). Zero rows are real agent turns. Already covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` (TODO).
2. **`/approvals` shows "All clear" empty state** in demo mode because the DB has 0 `approval_request` rows. Scene 6's narration ("an AI worker requests approval, you approve, see the audit trail") has no data to point at. This is the demo-seed gap that `mission-demo-seed-data-bootstrap` was rescoped for (option B real backend seed CLI per the no-theater directive).

**Function-side defects**: 1 P0 (audit-detail SSR hangs >60s), 1 P0 (approvals demo has nothing to demonstrate), 2 P1, 4 P2, 2 P3. **Dressing-side issues**: metadata title leaks (already in flight via existing brief).

## Headline blockers

- **F1 [P0]** — Scene 6 cannot be recorded as scripted: `/approvals` shows "All clear" (DB has 0 approval_requests) and `/audit` shows only SPA self-traffic. The camera's drill-in lands on a `http.request` row, not an agent turn.
- **F2 [P0]** — SPA proxy to `/api/v1/audit/<id>` takes 11s for a request the gateway serves in 220ms; SSR of `/audit/action/<id>` times out >60s. Click-through is broken-feeling at recording time.
- **F3 [P1]** — Audit feed renders 31 rows of `http.request` from worker "UN" — telegraphs "this product is observing itself" rather than "AI workers acting."
- **F4 [P1]** — `/audit` h1 title leak: `<title>` reads "Approvals · SKYCOT" on a number of in-app pages because the app-shell metadata default is `"Approvals"`. Already in-flight via the `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages` brief.

## All findings, in order

### F1 [P0] — Scene 6 demo-data void: 0 approvals, 0 real audit rows

**What I saw**: `/approvals` renders the locked E-ops-dashboard (KPI strip, severity tiers, donuts) but the data layer is empty — `pending=0`, `resolved/wk=—`, `avg response=—`, `$ at risk=—`, `approval rate=—`, plus a centered "All clear / Your teammates have nothing waiting on you" empty state. The DB confirms `SELECT COUNT(*) FROM approval_request → 0`. `/audit` renders 31 rows but every row is `verb=http.request, worker=unknown, action_type=http.request`.

**Why it matters**: Scene 6's runbook beat is "AI worker hits guardrail → asks for approval → you approve → audit trail receives a real provenance row." Today there is nothing on the inbox AND nothing in the audit log that the camera can drill into.

**Function-vs-dressing**: this is function-side. The audit-row click-through goes to a real detail page that loads a real audit detail, but the detail itself is for a SPA-internal HTTP request — there's no skill, model, policy, memory_snapshot, decision_chain content. Camera-readable theatre.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/approvals` → `{"items":[]}`
- `curl 'http://127.0.0.1:7400/api/v1/audit?limit=50' | python3 -c 'import json,sys; d=json.load(sys.stdin); print(set(i["verb"] for i in d["items"]))'` → `{'http.request'}`
- `sqlite3 ~/.argo/sessions.db "SELECT COUNT(*) FROM approval_request"` → `0`

**Suggested fix**: ship `mission-demo-seed-data-bootstrap` (the rewritten option-B real-backend-seed CLI per the operator's no-theater directive). One seeded approval per persona + one seeded multi-step agent turn per Mariah/Peta is enough to demonstrate the trust loop.

### F2 [P0] — SPA proxy for `/api/v1/audit/<id>` is 50× slower than gateway-direct; SSR of `/audit/action/<id>` times out

**What I saw**: gateway-direct `GET /api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e` returns 200 in 0.22s. Same request through SPA proxy at `http://localhost:3000/api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e` takes 11.22s. SSR of the page itself (`/audit/action/<id>`) hangs >60s. This isn't only turbopack cold-compile — `/audit` returns 200 in 3.8s, the detail route does not return inside a minute.

**Why it matters**: in demo recording, the operator clicks an audit row and waits >10s for the detail to populate. That dead-air is camera-visible.

**Function-vs-dressing**: function-side reliability gap.

**Reproduction**:
- `curl -m 10 -o /dev/null -w "%{time_total}\n" 'http://127.0.0.1:7400/api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e'` → ~0.22s
- `curl -m 20 -o /dev/null -w "%{time_total}\n" 'http://localhost:3000/api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e'` → 11.22s
- `curl -m 60 -o /dev/null -w "%{time_total}\n" 'http://localhost:3000/audit/action/019e6666b1dd7ef3971b47289dc1a76e'` → times out at 60s

**File**: `web/next.config.ts:30-32` (rewrite is one-line `/api/v1/:path*` → gateway). Bottleneck is likely turbopack dev or the SSR page doing an extra fetch path. Worth investigating whether `web/app/(app)/audit/action/[actionId]/page.tsx` has a server-side data hook even though the file is `"use client"`.

**Suggested fix**: investigate dev rewrite latency under turbopack — there may be a synchronous .next chunk regeneration loop on each `/api/v1/audit/<id>` rewrite. Verify the audit-detail page is genuinely client-only (no parent server component triggering a wait).

### F3 [P1] — Audit feed is 100% SPA self-traffic (worker="unknown", verb="http.request")

**What I saw**: 31/31 rows in `/api/v1/audit?limit=50` have `verb=http.request`, `worker.name="unknown"`, `worker.id=""`. Drilling into one returns a real hash-chain but for the SPA's own poll of `/api/v1/audit`. The Etherscan-style table renders these honestly, but the camera reads "http.request · UN · 1m ago · http.request · UN · 1m ago" — every row reads as the SPA observing itself.

**Why it matters**: Scene 6 narrates "audit shows what your AI teammates DID." The feed shows what the SPA's own polling did.

**Already in flight**: this is covered by `docs/build-plan/streams/fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request.md` (TODO). Recommend prioritising before recording.

**Function-vs-dressing**: function-side — the verb / worker text is data the camera reads.

**Suggested fix**: ship the existing brief. Per its recipe, exclude `verb IN ('http.request', 'worker.kpi_rolled', 'audit.detail.viewed', 'audit.replay', 'audit.list.viewed')` at the list endpoint unless `?include_meta=1`.

### F4 [P1] — `/audit` and `/approvals` h1 titles correct, but `<title>` element shows "Approvals · SKYCOT" on `/audit`

**What I saw**: navigating to `/audit` gives `document.title === "Audit · SKYCOT"` (this page has its own metadata). But on first navigation from `/` → `/approvals` the title sometimes lags as "Approvals · SKYCOT" on pages that don't set their own metadata. Operator memory + existing brief confirm 9 pages leak the "Approvals" default: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`.

**Already in flight**: `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages.md` (no owner assigned at audit time).

**Function-vs-dressing**: dressing-side; demo recording stays on canonical pages with their own metadata, but `/genesis` (investor moment per `project_pitch_agent_v1_demo`) IS on the recording path and currently reads "Approvals · SKYCOT" in the tab.

**Suggested fix**: ship the brief. 1-LOC default change + 9 per-page metadata exports.

### F5 [P2] — Demo-mode flat default tenant causes audit cross-tenant test to look broken until you supply a real tenant UUID

**What I saw**: hitting `/api/v1/audit?limit=2` with `X-Argo-Tenant-Id: evilco`, `NOSUCH`, `anonymous`, or no header at all returns 2 rows of recent self-traffic. The audit feed scope code at `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-66` IS doing the right thing — it filters by `payload.tenant_id` — but every gateway-request-audit row gets tagged `tenant_id="anonymous"` (5527/8344 rows in the DB are anonymous), so any header that happens to match anonymous lane returns data. When you supply a real tenant UUID (e.g. `019e613b-9ca2-70b2-a2a9-b54373df98b4`), the response correctly returns ITS rows only — verified the IDs against the DB.

**Why it matters**: the security control is sound; the test surface is just noisy. A security reviewer hitting the public API without a real tenant header might wrongly conclude cross-tenant isolation is broken (I did, until I queried the DB).

**Function-vs-dressing**: function-side, but not a demo blocker. POC will only show one tenant.

**Suggested fix**: documentation only — add a one-paragraph note to the gateway README explaining that the demo workspace uses `tenant_id="anonymous"` for gateway-request-audit rows and that real-tenant isolation must be probed with a real UUID. Code stays as-is.

### F6 [P2] — `/api/v1/team` and `/api/v1/approvals` ignore the `X-Argo-Tenant-Id` header; cross-tenant gating goes through `?tenant_id=` query and CWD-derived workspace

**What I saw**: `/api/v1/team` resolves the tenant via `resolve_workspace_tenant(store)` which does `ensure_workspace(name=cwd_file_name, &root)` — the tenant is bound to the gateway's CWD, not the request header. Cross-tenant test: `?tenant_id=00000000-0000-0000-0000-000000000000` returns 403, `?tenant_id=evilco` returns 400 "invalid tenant_id". So gating is functionally correct, but the X-Argo-Tenant-Id header is silently ignored on `/team` and `/approvals` (it IS honored on `/audit`).

**Why it matters**: inconsistent header semantics across endpoints. If a multi-tenant gateway ever ships, every endpoint will need to honor the same scoping signal. Single-tenant-per-workspace today is documented in the route source comment but not in any user-facing surface.

**File**: `crates/argo-cli/src/handlers/gateway/routes_team_list.rs:234-243` (CWD-derived tenant) and `routes_approvals_json.rs:221-232` (same pattern).

**Suggested fix**: long-term, when v1.x adds multi-tenant per gateway, refactor `resolve_workspace_tenant` to honor the verified-JWT-session tenant first, then `X-Argo-Tenant-Id`, then CWD fallback. Out of scope for v1.0 demo.

### F7 [P2] — Replay endpoint returns hash-recompute envelope, SPA adapts via `adaptGatewayReplay`; works correctly but the contract drift is a TODO

**What I saw**: `POST /api/v1/audit/<id>/replay` returns `{verdict: "same", expected_self_hash, actual_self_hash, diff: {}, ...}` — the hash-recompute envelope. `web/lib/api/replay.ts:204-228` has an `adaptGatewayReplay()` adapter that maps that envelope to the SPA's canonical `ReplayOutcome` discriminated union. The adapter is correct (verdict "same" → "reproducible"; verdict "drifted" + policy diff → "drifted-misaligned"; otherwise "drifted-aligned"). The TODO comment in the adapter says "Once gateway ships the SPA-shaped response per `fix-gateway-replay-envelope-spa-contract`, delete this adapter."

**Why it matters**: no demo impact — the adapter does the right thing. But it's a documented contract debt. Replay is genuinely real: I verified `expected_self_hash == actual_self_hash` for a real audit row (`5ff07b7f53ee3d916b128dd8b0eb8c0edc08b4dc55d7c43546ad8404cebfabc7`), so the hash chain checks out cryptographically.

**Suggested fix**: track via the existing brief `fix-gateway-replay-envelope-spa-contract`. No demo blocker.

### F8 [P2] — Unknown / 32-char-hex actionId for replay returns `failed` envelope only when demo+auth-bypass is on; production replay surface lacks a unit test

**What I saw**: `web/lib/api/replay.ts:230-244` has `UNKNOWN_ACTION_FAILED` envelope for ids not in `findSampleReplay` table. The fabrication-from-URL bug that the E5-d brief closed is now safe — `replayAction` returns `{kind: "failed", reason: "Action not found — the replay sandbox only covers the seeded demo personas."}` for any id outside the canonical sample table when `isDemoMode() || isAuthBypassed()` AND not `isDemoLive()`. When `isDemoLive()` is on, the request goes to the real gateway, which 400s for invalid hex ids (`audit id must be 32-char hex (16 bytes)`) — that's handled by the catch block returning `sampleOrFailed`.

**Why it matters**: covered. The fabrication bug is closed. This is a P3 nit on test coverage.

**Suggested fix**: add a vitest fixture for `replayAction("not-in-table-id")` returning `failed` envelope. Probably exists already in `web/lib/api/replay.test.ts` — quick verify.

### F9 [P2] — Demo banner ("Demo mode — showing sample data") covers the top of every in-app page; "Sign in to see your own workspace" CTA is the only path out of demo

**What I saw**: the violet-gradient demo banner persists across all in-app pages. It's dismissable per session, but the dismiss state isn't persisted (refresh → banner returns). For Scene 6's recording, the banner sits over the "Approvals" h1 and adds 32px to every above-fold capture.

**Why it matters**: function-side — does not break the demo, but it's chrome the camera reads. The banner copy is also slightly off-tone for an investor demo ("showing sample data") since by the no-theater directive the data SHOULD be real backend seed data.

**Suggested fix**: rescope the banner copy to "Demo workspace — exploring real Skycot capability" or similar once seed data is real. The dismiss-persist is a smaller polish brief.

### F10 [P3] — `/audit` Etherscan-style table renders rows as `<div role="list">` not `<table>`; screen-reader announces ambiguous structure

**What I saw**: `web/components/skycot/audit/AuditDayGroup.tsx` renders the day-grouped feed using `<section><h2><div role="list"><div data-status … /></div>…` — not a real `<table>` with `<thead>/<tbody>`. The locked Etherscan-style design is preserved visually but the markup isn't semantically tabular. Screen readers will hear "list, 31 items" rather than "table, 31 rows, columns: time, worker, verb, id, status."

**Why it matters**: accessibility nit. Etherscan itself uses a real `<table>`. The locked design memory (`project_audit_page_design_locked`) doesn't specify markup, only visual.

**Suggested fix**: post-MVP polish — refactor `AuditDayGroup` to use `<table>` with header cells. Not a demo blocker; the camera doesn't read DOM semantics.

### F11 [P3] — Approvals page KPI strip shows em-dash ("—") for empty metrics; reads slightly defensive vs. clean zero state

**What I saw**: `Resolved/wk: —`, `Avg response: —`, `$ at risk: —`, `Approval rate: —`. The `—` placeholder reads "we have no data" but in a clean ops-dashboard the zero state should arguably be `0` with explanatory tagline. `Pending: 0` IS rendered as `0` (correct), so the page is inconsistent in its empty-state vocabulary.

**Why it matters**: minor polish. Locked design (memory `project_approvals_page_design_locked`) doesn't specify which numeric metrics show `—` vs `0`.

**Suggested fix**: normalize. Suggest: `Pending: 0` stays as `0`; `Resolved/wk: 0`, `Approval rate: 0%`, `$ at risk: $0`, `Avg response: —` (since avg over 0 events is genuinely undefined).

## Resolved since last audit

The `2026-05-27-0623` scene-6 audit (`scene-6-claim-6-trust-and-approvals.md`) had similar findings. Two appear resolved:

- **Replay fabrication closed**: `web/lib/api/replay.ts:230-276` now uses the `findSampleReplay` lookup + `UNKNOWN_ACTION_FAILED` envelope. Past brief `fix-audit-replay-page-rendering-fabricated-mock-report` shipped.
- **Approvals decide 403 is properly toasted**: `web/lib/api/approvals.ts:102-119` now surfaces `toastForbidden` instead of silently faking `ok: true`. Past brief `mission-rbac-approvals-decide-role-gate` (#1956) shipped.

Still open from prior audit: audit-self-traffic filter, demo-seed gap.

## Orientation score: 64 / 100

Function-side is real and trustworthy — the hash chain genuinely recomputes, tenant isolation is correctly gated where it matters (the /team 403, the audit feed payload-tenant filter), replay no longer fabricates. The trust story COULD record well.

What pulls the score down: the demo workspace has nothing for the camera to drill into. The 31-row audit feed of `http.request · UN · http.request · UN` is the visible artefact of an empty backend. Until `mission-demo-seed-data-bootstrap` ships real seed approvals + real seed agent turns, Scene 6 can't be recorded without theatre the camera sees.

Two structural wins: (a) the replay endpoint is genuinely hash-chained (not a stub) and (b) cross-tenant probes return 403 / 400 on the right surfaces. The trust + multi-tenant claim is **architecturally LIVE**; the demo just lacks real data to put on top.
