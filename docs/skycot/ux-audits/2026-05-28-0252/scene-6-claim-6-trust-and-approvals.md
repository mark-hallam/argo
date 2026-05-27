# Scene 6 + Claim 6 — Trust & Approvals UX audit

**Date**: 2026-05-28 02:52 AEDT (UTC 2026-05-27 ~17:00Z)
**Scope**: `/approvals` (E-ops-dashboard), `/audit` (B3 Etherscan-style), audit drill-in (`/audit/action/<id>`), audit replay (`/audit/action/<id>/replay`), cross-tenant isolation negative test, multi-channel approval evidence (Claim 6).
**Mode**: read-only, no cargo, no edits. Gateway 7400 schema v83 + SPA preview (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`).
**Reframe applied**: POC/investor demo function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

---

## TL;DR

The two locked designs render (E-ops-dashboard + Etherscan-style audit table) — visually they match the brand-loud / B3-Etherscan design specs. **But the drill-in path the camera walks in Scene 6 is broken in two compounding ways**:

1. The /audit feed today renders **fixture rows** (`act_018f67d0-…` IDs from `lib/sample-audit.ts`) instead of the real gateway feed, because the live-gateway feed only carries `worker.kpi_rolled` / `team.list.queried` housekeeping rows under a tenant the SPA can't reach (see F2). Even with `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` set, the audit table the camera sees is mock.
2. Clicking a fixture row → gateway returns 404 → SPA falls back to a **fabricated mock provenance card** (`mem_synthetic`, `generic-skill`, `generic-policy`, "Trigger reconstructed from feed row"). This is the textbook function-side P0 the function-vs-dressing rule prohibits — the camera will read the placeholder strings.

Counts: 1 P0, 6 P1, 5 P2, 3 P3 (15 findings).

---

## Headline blockers

- **F1 [P0]** Audit drill-in renders fake provenance card (`mem_synthetic`, `generic-skill 1.0.0`, `generic-policy 1.0`, "Trigger reconstructed from feed row · ticket_resolved") when the row ID isn't in the gateway. Scene 6 plays this exact beat.
- **F2 [P0]** List↔detail tenant-id resolution drift: `GET /api/v1/audit` (list) and `GET /api/v1/audit/<id>` (detail) read different tenant headers (`x-tenant-id` + `?tenant=` vs `x-argo-tenant-id`) with different fallback defaults. A row returned by the list is **404 on the detail** when no tenant header is sent. Confirmed in-browser from the SPA: list returns real rows, detail on the same ID returns `{"error":{"code":"not_found"}}`.
- **F3 [P0]** No multi-channel evidence in the Approvals UI: the data model has `channel: ChannelKind` (slack/sms/imessage/whatsapp/voice/push/web) but the dashboard renders no channel chip on any approval card. Scene 6 narrates "multi-channel approvals" with nothing on screen to back it.

---

## All findings

### F1 [P0] — Drill-in renders fake provenance card with literal placeholder strings

When the camera clicks an /audit row (currently any of the fixture `act_018f67d0-…` IDs), the drill-in fetches `/api/v1/audit/<id>`, gets 404 from the gateway, and **`web/lib/api/audit-detail.ts:110-114`** falls through to `mockDetail(actionId)`. The rendered card then shows: `MEMORY mem_synthetic · mem_s…`, `SKILL VERSION generic-skill · 1.0.0`, `POLICY VERSION generic-policy · 1.0 …`, `PROMPT TEMPLATE generic.action · feed…`, and `Trigger reconstructed from feed row · ticket_resolved.` These are placeholder strings the camera will read at high zoom in Scene 6's hash-chain beat.

Reproduction:
- Open `/audit/action/act_018f67d0-0000-7000-8000-000000000003_025` against the running SPA (preview port).
- Observe `Provenance & Chain` panel renders `mem_synthetic`, `generic-skill`, `generic-policy`.
- Network shows `GET http://localhost:3000/api/v1/audit/act_018f67d0-0000-7000-8000-000000000003_025 → 404 Not Found`.

Files: `web/lib/api/audit-detail.ts:88-115` (404 → mockDetail fallback), `web/lib/sample-audit-detail.ts` (the synthetic strings).

Suggested fix (do NOT implement here): for the demo recording, either (a) seed real audit rows whose IDs are stable + return them from the gateway, or (b) suppress the mockDetail fallback when `isDemoLive() === true` so a 404 surfaces the empty-state instead of the synthetic card.

---

### F2 [P0] — Gateway list↔detail tenant-id resolution drift

Re-verified in-browser from the running SPA:
- `fetch('/api/v1/audit?limit=2')` (no tenant header) → 200, returns real rows like `019e6a5facdd79f2a9a37ae87e22a6e4`.
- `fetch('/api/v1/audit/019e6a5facdd79f2a9a37ae87e22a6e4')` (same SPA, same session) → `404 {"error":{"code":"not_found"}}`.

Root cause: the list uses `rate_limit::extract_tenant` (reads `?tenant=` query param, then `X-Tenant-Id` header, then `argo_config.tenant.default_id`, finally `"anonymous"`). The detail uses `header_value(req, "x-argo-tenant-id")` defaulting to `DEFAULT_AUDIT_TENANT_ID` = `"ws-default"`. Two different headers, two different fallbacks. The SPA `lib/api/audit.ts` + `lib/api/audit-detail.ts` send NO tenant header at all — so list resolves to `"anonymous"`, detail resolves to `"ws-default"`, and the stored rows are under the demo-tenant UUID (`019e6785-654d-7302-9a29-ff6035517c53`).

This isn't a cosmetic bug — it means even when the audit feed is wired live (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`), every drill-in click 404s and triggers the F1 mock-fallback.

Files: `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:443-446` (list reads `extract_tenant` → query/header/`anonymous`), `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:171-194` (detail reads `x-argo-tenant-id` → `ws-default`). SPA: `web/lib/api/client.ts:85-88` sends no tenant header.

Suggested fix: detail endpoint should use the same `extract_tenant` resolver as the feed, OR both should read the tenant from the verified session JWT (which is the architecturally correct answer once auth-bypass mode is off).

---

### F3 [P0] — Approvals UI never shows the channel — Claim 6 visual signal absent

Claim 6 is "multi-channel + multi-tenant trust." The approvals data model carries channel (`ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp"`) on every row, but `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` and sibling card components (`ApprovalRowActions.tsx`, KPI strip, severity tiers) render no channel chip or icon anywhere. Only fitness reference to channel in the dashboard file is the comment `// Undo back-channel.` (line 192) — nothing visible.

If Scene 6 mentions multi-channel approvals, the audience sees no Slack / SMS / Discord / voice indicator on the page. The "multi-channel" claim is invisible.

Reproduction:
- Load `/approvals` (E-ops-dashboard renders).
- Scan KPI strip + tier cards + each approval row for any channel iconography.
- None present; the `channel` field of each `ApprovalItem` is unread by the UI.

Files: `web/lib/sample-approvals.ts:17` (channel kind defined), `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (no channel render).

Suggested fix: add a small channel chip per approval row (Slack/SMS/voice/etc) — the data is already there, no backend work required.

---

### F4 [P1] — Approval card layout collapses to single-character column

In the screenshot of `/approvals` the first P0 approval card renders with the body text squashed to roughly 4-character width: title "W…" then "Peta", then body text wrapping vertically letter-by-letter ("Thr / cust / repo / the / sam / API …"). The right rail (`$ at risk · $48k`) is intact but the card content is unreadable.

Repro:
- Load `/approvals` against running SPA on port 53028 (preview).
- First HIGH-tier card under the KPI strip is the affected one.

Suggested fix: check the flex / min-width on the card content column in `ApprovalsOpsDashboard.tsx`; the right rail likely has `flex-shrink: 0` while the left content lacks `min-width: 0`. Test viewport: 1600×900 (default capture).

---

### F5 [P1] — Audit feed renders fixture rows in demo-live mode (defeats DEMO_LIVE=1)

The launch config sets `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`, which per `web/lib/api/audit.ts:349` is supposed to skip the mock-bypass short-circuit and hit the real gateway. But the audit page still renders fixture rows ("resolved SUP-29382 with KB article SK-413", "drafted standup digest for the engineering pod", `act_018f67d0-…`) — these come from `web/lib/sample-audit.ts:134` (the `act_${worker.id}_${i}` synth pattern).

This means one of:
1. `isDemoLive()` is returning false at module-eval time (env var not actually inlined into the bundle), or
2. The gateway feed call IS firing but returning the auto-emitted `worker.kpi_rolled` / `team.list.queried` housekeeping rows under tenant `019e6785-654d-7302-9a29-ff6035517c53` — which the SPA's "anonymous" tenant-id call doesn't see — so the SPA falls through to the demo-mode mockPage (`audit.ts:362-364`).

I observed the SPA list call returning real-looking rows (`019e6a…` IDs) for the "anonymous" tenant scope in the gateway DB, but the rendered page shows `act_018f67d0-…` — so the bundle is short-circuiting demo-live before the network call. Worth diagnosing before recording day.

Suggested fix: log `isDemoLive()` + `isAuthBypassed()` at app boot in the browser console so the operator can confirm which path is live.

---

### F6 [P1] — Approvals queue empty against real gateway; KPI strip silently invents 5/$48k/43m

`GET /api/v1/approvals` against the live gateway returns `{"items":[]}` (empty). The /approvals page shows KPI strip with `PENDING 5 active human decisions`, `$ AT RISK $48k`, `MEDIAN TIME 43m waiting for approval`, `APPROVAL RATE 67%`, `ACTIVE REVIEWERS — reviewer roster pending` — and at least one HIGH-tier and other approval cards (e.g., "Peta · Three customer reports..."). The numbers and cards are the bundled fixture (`SAMPLE_APPROVALS` from `lib/sample-approvals.ts`).

Demo classification: per the function-vs-dressing rule, dressing-side KPI fixtures are fine. But the moment the camera clicks "Approve" or "Refuse" on one of these fixture cards, that's a function-side beat. The current path: bypass mode short-circuits `submitDecision` to `{ ok: true, source: "mock-auth-bypass" }` with no gateway round-trip (`web/lib/api/approvals.ts:212-214`). The toast says "Approved" but nothing was approved. For the v1.0 POC demo this is acceptable IF the runbook narrates around it; if the runbook says "watch the approval round-trip", it's a P0.

Files: `web/lib/api/approvals.ts:184-205` (fetch short-circuit), `web/lib/api/approvals.ts:207-251` (decide short-circuit).

Suggested fix: same as F5 — make sure DEMO_LIVE actually wires the gateway path so the Approve click really mutates state.

---

### F7 [P1] — Real audit rows show UUIDs as worker names, not persona names

In the gateway's live `/api/v1/audit?limit=20` response, every row's `worker` field reads `{"id": "019e6785-928f-7281-bf8d-e062bc277478", "name": "019e6785-928f-7281-bf8d-e062bc277478"}` — the UUID is duplicated as the display name. If Scene 6 ever runs against the real gateway feed (which F5 says it currently doesn't, but DEMO_LIVE is supposed to flip it), the audit table would render rows labeled with raw UUIDs instead of "Mariah" / "Peta" / "Priya" / "Jamal" / "Nori".

Files: gateway audit-feed serializer (worker-name lookup is not being joined against the worker registry).

Suggested fix: have the audit feed serializer join against the workers table on `worker_id` and use the worker's `name` for the `name` field. Falls back to UUID only when the worker has been deleted.

---

### F8 [P1] — Replay endpoint works on real IDs, but SPA replay UI never reaches it

`POST /api/v1/audit/<real-32-hex-id>/replay` returns a clean verdict envelope: `{verdict: "same", expected_self_hash: ..., actual_self_hash: ..., seq: 23033, replayed_at: ...}`. The hash chain validates. **However** the SPA's `/audit/action/<id>/replay` page only ever sees the `act_018f67d0-…` synth IDs from the fixture feed (F5), which gateway returns 400 `audit id must be 32-char hex (16 bytes)`. SPA falls back to `findSampleReplay` (`web/lib/api/replay.ts:263-264`). So the Replay button shows a hand-built fixture verdict, never the real one — even though the real one is one curl away.

Suggested fix: same upstream as F5/F2 — get the real audit IDs in front of the SPA and the replay surface will hit the live endpoint.

---

### F9 [P1] — Hash chips on audit feed are decorative slug, not the real Merkle hash

The /audit table renders `0x0000…001a`, `0x0000…000b`, `0x0000…0001` chips on each fixture row. These come from the fixture row index (`i.toString().padStart(4, "0")`) — not from the row's `self_hash`. The real gateway feed includes `self_hash` (64-char hex) + `parent_hash` (64-char hex) per row — those are the actual hash-chain anchors. If the camera hovers the chip expecting to see a real Merkle anchor, it will see `0x0000…0001`.

Note: when I navigated back to /audit later (briefly), the chips were `0x45ec…cbd6`, `0x8025…8c30`, etc — those look like real hash prefixes. So the chip might actually be reading real hash data on some renders. Worth a confirmation pass before recording.

Files: `web/lib/sample-audit.ts:134` (synthetic ID slug for hash chip), audit page table column for hash chip render.

Suggested fix: ensure the chip is always reading `row.self_hash.slice(0, 4) + "…" + row.self_hash.slice(-4)`, not a synthetic counter.

---

### F10 [P2] — Pre-existing dev-bypass widens audit feed to any caller

In local-dev with `ARGO_GATEWAY_AUTH_BYPASS=1`, `/api/v1/audit` returns rows with **no Authorization header at all** AND with arbitrary bogus bearer tokens. This is by design (the dev-bypass principal is `system:dev-bypass`), but it means the operator's machine has a fully-readable audit log on `127.0.0.1:7400` while bypass mode is on. If the recording laptop ever exposes 7400 on a LAN — e.g. by accidentally binding `0.0.0.0` instead of `127.0.0.1` — the audit feed leaks. Not a POC demo blocker; flag for the day-of-recording op-sec checklist.

Files: `crates/argo-cli/src/handlers/gateway/startup_check.rs:150-162` (the bypass gate).

---

### F11 [P2] — Approve / Refuse never hits gateway in current bypass mode

Repeating the function-side note from F6 for explicit recording-day clarity: in the current preview config (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`), `submitDecision` returns `mock-auth-bypass` source without firing `POST /api/v1/approvals/<id>/decide` — see `web/lib/api/approvals.ts:212-214`: `if (authBypassEnabled() && !isDemoLive()) return { source: "mock-auth-bypass" }`. The condition reads "bypass AND NOT demo-live" — so demo-live SHOULD make the POST fire. If the audit feed is showing fixtures (F5), demo-live isn't taking effect on this surface either; verify before recording.

Test plan: on the live preview, open dev-tools Network, click Approve on a card, check whether a `POST /api/v1/approvals/<approval-id>/decide` shows up.

---

### F12 [P2] — "MOCK-BYPASS" badge appears on /team but no equivalent chip on /approvals or /audit

The /team page shows a `MOCK-BYPASS` chip next to the title (visible in the first preview screenshot of `/approvals` — the navigation accidentally landed on /team). /approvals and /audit have no such chip. The chip is the operator's "this is fixture, not live" cue; if the camera ever pauses on /approvals or /audit in fixture mode, the absence of the chip looks more "real" than it should.

Suggested fix: render the same `DataSourceChip` on /approvals + /audit when source is `mock-auth-bypass` or `mock-offline`. The `useApprovals` hook already exposes `dataSource` for exactly this purpose.

---

### F13 [P2] — Audit feed page has no hash-chain hover tooltip per the locked B3 design

The B3-Etherscan design (per memory `project_audit_page_design_locked`) calls for a hash-chain hover tooltip on each row that reveals the full `parent_hash → self_hash` link. In the current implementation, the chip is static — no hover, no tooltip. The Etherscan analogy promises that interaction; without it, the audit table is a static list, not a chain.

Files: audit row component (locate via `web/components/skycot/audit/`).

---

### F14 [P3] — Audit row count, KPI numbers, and confidence donuts are visually correct but provably fictional

The 5-KPI strip (PENDING 5, $48k, MEDIAN 43m, APPROVAL RATE 67%, ACTIVE REVIEWERS —) and the per-tier donuts are fixture aggregates of `SAMPLE_APPROVALS`. Per the function-vs-dressing rule these are dressing-side and acceptable for the POC demo. Mentioning only because the audit reframe doc explicitly classifies these as P2/P3 not P0.

---

### F15 [P3] — Cross-tenant negative test inconclusive in dev-bypass mode

I minted a second principal (`rival@othertenant.com`) via `/api/auth/demo-mint` and called the gateway with that token. Result: both principals see the same data because both resolve to `system:dev-bypass` under `ARGO_GATEWAY_AUTH_BYPASS=1`. **This isn't a production leak** (the gateway tests `routes_audit_detail_tests.rs:164 detail_returns_404_for_cross_tenant_action_id` cover the real path), but the v1.0 demo cannot record a "watch the rival workspace get a 404" beat against the local preview as-configured. Boot the gateway WITHOUT bypass for that beat, or skip it.

---

## Resolved since last audit

`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md` (prior pass) flagged the approvals empty-state short-circuit; that's now removed (per `web/app/(app)/approvals/page.tsx:22-28` comment — fix-approvals-inbox-waiting-on-you-fixture-drift). The locked E-ops-dashboard now renders regardless of queue depth. ✅

Worker IDs no longer crash the audit feed (prior reports cited this); they now show as UUIDs (F7 above) which is a separate, milder bug.

---

## Orientation score

**52 / 100.**

The visual design is locked and rendered correctly (E-ops-dashboard with 5-KPI strip + severity tiers + $ at risk + donuts + Approval Rate ring; B3 Etherscan audit table with hash chips). The data plumbing is the weak link: list↔detail tenant-id drift (F2) means real audit data never makes it to the drill-in; the SPA quietly falls back to a synthetic provenance card (F1) using literal "synthetic" / "generic" placeholder strings. Multi-channel approvals (Claim 6's headline) have no UI representation despite the data model carrying the field (F3).

For the POC demo: F1 + F2 + F3 are recordable-as-scripted blockers — the camera in Scene 6 will read placeholder text on the drill-in card unless one of (a) seed real audit rows the SPA can reach, or (b) suppress the mock-fallback in demo-live mode, lands first. F5/F6/F8/F11 all stem from the same `isDemoLive()` short-circuit not actually flipping; one diagnostic console-log at boot would confirm. F4 (squashed card column) is a 20-line CSS fix.

Function-side: the gateway primitives ARE real and the hash chain validates (`/api/v1/audit/<id>/replay` returns verdict "same" on a real 32-hex ID). The bridge from SPA to those primitives is what's broken.
