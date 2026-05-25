# Scene 6 (Claim 6 — Multi-tenant + multi-channel) — trust + approvals UX audit

**Date**: 2026-05-25 11:00 AEST
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 + /approvals)
**Method**: source-code walk + live curl against gateway on `127.0.0.1:7400` (schema v81, 5h uptime). The Next dev server is up on `:3000`; no demo-mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE`) is set in `.env.local`, so the SPA is hitting real gateway data with `NEXT_PUBLIC_ENV=development`.
**Prior audit referenced**: `docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` (operator-rejected). F67 has been independently re-verified below.

---

## TL;DR

Scene 6 has **3 P0 demo-blockers** and **6 P1 visible issues**. The headline blocker is real and unchanged: `/audit/action/{id}/replay` still fabricates a forensic mock report for any unknown action ID (the prior F67). A second P0 just surfaced: with the current gateway data, **clicking any real audit row 404s** — the audit-list endpoint returns rows whose IDs the audit-detail endpoint does not recognise, breaking the entire Scene 6 drill-in. A third P0: with live gateway state, `/approvals` shows the empty-state instead of the locked ops-dashboard at all — the brief's KPI strip / severity tiers / donuts are unreachable. The hash-chain "prev → this" hover affordance the brief calls out as Scene 6's centerpiece does not exist; only a single-row hash tooltip ships today, and the field is dropped entirely on live data.

---

## Headline blockers

### P0 — F1: `/audit/action/{id}/replay` fabricates forensic content for fake IDs (F67 unchanged)

`web/lib/api/replay.ts:69–88` catches 404 from `POST /api/v1/audit/{id}/replay` and falls through to `mockOutcomeForActionId()` (`web/lib/sample-replay.ts:142–201`) **whenever NODE_ENV !== production AND NEXT_PUBLIC_ENV !== production**. Unlike `audit.ts` and `approvals.ts`, the replay client never consults `isDemoMode()` — the fallback is unconditional in any non-strict-prod build.

`mockVerdict()` hashes the last char-code of the action ID mod 3 → one of `reproducible` / `drifted-aligned` / `drifted-misaligned`. For verdict 2, `ReplayHeader.tsx:23–38` renders the literal red banner:

> **Policy regression detected.** Replay output would have failed the current policy. Operator notified.

Plus an "ORIGINAL output" / "REPLAYED output" diff whose strings are concatenated from the URL parameter ("Reconstructed original output for {actionId}. Policy tightening would block this output today."). All fabricated.

I confirmed via curl: a 32-char-hex unknown ID `01234567890abcdef0123456789abcde` returns gateway 404, which `isMissingOrUnreachable` matches → the client returns the mock. The user-supplied test case `act_test_obviously_fake_999` instead hits gateway 400 ("audit id must be 32-char hex") which the client does NOT match → the error propagates → `FailedView` renders with the gateway's error message. **So the F67 reproduction depends on the fake ID being well-formed; any reasonable URL-typing user lands on a fabricated report.** The prior audit's narrower repro doesn't undermine the finding; it strengthens it.

Per the function-vs-dressing rule, a fabricated forensic report IS function-side (the page claims to do the cryptographic replay). This is theatre the camera sees.

**Suggested fix** (do not implement): gate the mock fallback in `replay.ts:83` on `isDemoMode()` AND only for IDs present in `findSampleReplay()`'s `TABLE`. For any ID outside the canonical 5-persona table, render `<FailedView>` with a real error ("Action not found in replay sandbox") instead of synthesising content.

### P0 — F2: clicking any real audit row 404s — audit-list and audit-detail endpoints disagree

I curled `GET /api/v1/audit?range=7d` against the live gateway and got real rows like `id: "019e5dbaec037fa2819eab3f27f3f41e"`. Curling `GET /api/v1/audit/019e5dbaec037fa2819eab3f27f3f41e` returns:

```json
{"error":{"code":"not_found","message":"audit row not found"}}
```

`ActivityRow.tsx:30` constructs the drill-in href as `/audit/action/${id}` — so the demo's Scene 6 click-through ("Click an audit row → real provenance") immediately lands on the `<AuditActionNotFound>` shell (`audit/action/[actionId]/page.tsx:96–106`). The list endpoint and detail endpoint appear to disagree about which rows exist — likely a tenant-binding mismatch (list isn't scoping; detail is) or the rows in the list table aren't being persisted into the detail table. This is a function-side blocker for Scene 6 step 7 (the brief's "real provenance" promise).

**Suggested fix**: align tenant scoping between `/api/v1/audit` (list) and `/api/v1/audit/{id}` (detail) in the gateway, or have the list endpoint filter to only IDs the detail endpoint can resolve.

### P0 — F3: `/approvals` renders the empty state instead of the locked ops-dashboard on live data

`web/app/(app)/approvals/page.tsx:25–48` only renders `<ApprovalsOpsDashboard>` when `pendingCount > 0`. Live gateway returns `{"items":[]}` for `GET /api/v1/approvals`, so the operator's locked E-ops-dashboard design (KPI strip + 3 severity tiers + $-at-risk donut + confidence donut + 7-day approval rate ring) does not render at all on current backend state — the "Open the inbox" CTA empty state shows instead. The Scene 6 brief's promise of "/approvals — E ops-dashboard per memory `project_approvals_page_design_locked`" is currently unreachable.

The page docstring acknowledges: "The canonical approval queue lives at `/` (the AppNav's 'Approvals' row routes there)." This means a buyer who types `/approvals` and a buyer who clicks the sidebar see two different surfaces. The recorded demo route is ambiguous.

**Suggested fix**: either (a) seed a demo-mode approval payload via `argo demo seed` before recording, and confirm `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set when recording the MP4, or (b) merge `/approvals` and `/` into a single canonical surface that uses the ops-dashboard for both empty and populated states (so the empty-state shows KPI labels with `—` instead of an entirely different shell).

---

## All findings, in order

### F4 [P1] — `/approvals` ops-dashboard hardcodes the entire "7-day approval rate" ring

`ApprovalsOpsDashboard.tsx:135–142` passes the KPI strip the literal `{ headline: "78%", slices: { approved: 0.78, refused: 0.14, expired: 0.08 } }`. `ApprovalsOpsDashboard.tsx:180` likewise hardcodes the `ApprovalRateRing` props to `approved={0.78} refused={0.14} expired={0.08}`. The "7-day approval rate" sub-copy "78% approved · 14% refused · 8% expired" is also a string literal at line 187. None of these are derived from any real 7-day rollup.

Per function-vs-dressing: the 7-day rate is a NUMBER, not a drill-in target. If the camera doesn't click it, this is dressing-side (acceptable for the POC demo). If a buyer asks "how is that calculated?" mid-narration, this becomes a problem. **Recommend: derive from the audit feed's last 7 days of decided rows (still mock data in demo mode, but at least the math is real-shaped).**

### F5 [P1] — "$ at risk" donut visual fill is hardcoded 82%

`ApprovalsOpsDashboard.tsx:163` passes the `MetricDonutCard` `percent={totalAtRisk > 0 ? 0.82 : 0}` — a fixed 82% conic-gradient fill regardless of the actual dollar amount. The dollar number itself is summed from real `item.dollarAtRisk` values (function-side, real) but the visual gauge is purely cosmetic. If the camera zooms on the donut narrating "82% of cap at risk", that becomes function-side theatre.

**Suggested fix**: either remove the visual fill (replace with a flat circle), or derive the percent from some real ratio (e.g. $ at risk / known tenant approval-budget).

### F6 [P1] — Approval rows are unclickable; no detail surface exists

`ApprovalRow` (`ApprovalsOpsDashboard.tsx:242–296`) is rendered as a `<button>` with `aria-label`, hover style, focus ring — every visual affordance of a clickable item — but there is no `onClick` and no `href`. The Scene 6 brief asks to "click into an approval row" and verify real worker_id + audit lineage. There is nowhere to click into. Either the rows need to navigate to `/audit/action/{approval.actionId}` (linking approvals to their underlying audit row), or the visual affordance needs to be removed so the buttons don't lie.

### F7 [P1] — Hash-chain "prev → this" hover affordance does not exist

The Scene 6 brief calls the hash-chain hover "Scene 6's centerpiece" — the demo claim that hovering a row shows the prev-hash → this-row-hash linkage. The current implementation (`ActivityRow.tsx:166–195` `HashCell`) shows only the row's own hash on hover (`title="${hash} — Click to copy"`) with click-to-copy. There is no `prevHash` field on `AuditRow` (`lib/types/audit.ts`) and no rendering of a chain edge anywhere.

Worse, the seeded hash itself is not a chain: `sample-audit.ts:149` computes each row's hash as `rowHash(worker.id|verb|timestamp)` — eight FNV-1a passes on per-row content. Rows do not depend on previous rows. Per the comment at `:159`: "matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade". This is the canonical example of dressing pretending to be function — a visual that looks like a chain hash but provides zero chain integrity.

**Per the function-vs-dressing rule**: if the camera lingers on the hash column AND narrates "every row is hash-chained to the previous", this is function-side theatre and a P0. If the camera does not narrate the chain claim, it's dressing. The prior audit's F67 + this finding together create a "the audit chain is theatre" perception risk for any SOC-2-conscious viewer.

### F8 [P1] — Live audit rows from the gateway have no `hash` field at all

`HashCell` only renders when `activity.hash` is truthy. Live `/api/v1/audit` rows include NO `hash` field (confirmed via curl) — so on a fresh signed-in user without demo-mode, the entire hash column disappears. The Etherscan-style aesthetic the audit page promises is fixture-only.

### F9 [P1] — Live audit rows have empty `worker.name` ("unknown") and identical `verb` ("hipaa_compliance_check")

Curl against `/api/v1/audit?range=7d` returns rows like:

```
worker: {id: "", name: "unknown"}, verb: "hipaa_compliance_check", action_type: "hipaa_compliance_check"
```

Repeated identically across 7+ rows. The demo MP4 cannot show real audit drill-in narrative — it's all `unknown / hipaa_compliance_check`. Either the gateway needs richer seed data, or the demo MUST run with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` to fall back to `SAMPLE_AUDIT_ROWS` (which IS narratively rich).

### F10 [P1] — Approval-rate fallback in KPI cells is 0.78 with sub-label "7-day baseline"

`ApprovalsOpsDashboard.tsx:77`: `const approvalRate = decided.length > 0 ? approved / decided.length : 0.78;`. The sub-label at line 102 reads "7-day baseline" when no decided rows exist — implying a system that tracks a real 7-day rate. There is no such tracking; it's a literal 78. Same drug, smaller dose, as F4.

### F11 [P2] — "Active reviewers" KPI is hardcoded 1 when pending > 0

`ApprovalsOpsDashboard.tsx:108`: `value: pending.length > 0 ? "1" : "0"`. There is no derivation from a real reviewer roster. Camera-side, this isn't load-bearing for Scene 6 narration (reviewers aren't part of the script) so it's a P2 polish item. Worth removing or wiring to a real roster query.

### F12 [P2] — Cross-tenant isolation: live `/api/v1/approvals` ignores tenant header on the read path

I curled `GET /api/v1/approvals` with no header and again with `X-Tenant-Id: tenant_b_other` and both returned `{"items":[]}`. With no rows on either side I can't conclude isolation is enforced — I can only confirm the read path doesn't crash or leak. For the v1.0 demo evaluator scenario `claim_6_live_cross_tenant`, the gateway needs at least one tenant with seeded approvals + a negative test against tenant B. **Until that's curl-able, claim 6's "LIKELY-LIVE" rating on /approvals specifically is unverified by this audit.** (The audit-list endpoint similarly returned the same rows for both default and `X-Tenant-Id: tenant_b_other` — but that may just mean the gateway is treating the header as advisory in dev-bypass mode; I did NOT confirm whether the SQL-level filter is wired.)

### F13 [P2] — Approvals empty-state CTA "Open the inbox" routes to `/` — same surface AppNav already has

The `ApprovalsEmptyState` (`approvals/page.tsx:59–72`) hands the empty viewer to `/`. But the AppShell sidebar's "Approvals" tab also routes to `/` (`nav-items.tsx:23`). So a buyer who types `/approvals`, sees the empty state, clicks "Open the inbox" → ends up on the canonical inbox at `/`. Either both surfaces should converge or the empty-state CTA should keep the user on the locked ops-dashboard layout (showing `—` placeholders instead of jumping to a different aesthetic).

### F14 [P3] — `RedactionBanner` likely renders only when permissions are constrained

`audit/page.tsx:202–206` renders `<RedactionBanner>` if `presentCategories` is non-empty. In dev-bypass / no-RBAC the viewer policy is `{ permissions: <full set> }` so `categoriesLackedBy` returns []. The redaction banner is therefore invisible during the demo recording — which is correct, but means the camera doesn't get to demonstrate the "we redact PII for under-privileged viewers" trust signal that's part of Scene 6's vocabulary. Not a blocker; flagging for the narration script.

---

## Independent verification of prior-audit F67

The brief asked me to independently verify, not trust, the F67 finding from `pre-record-ux-audit-2026-05-25.md`. Confirmed: F67 is real, present, and unchanged. The exact red banner string ("Policy regression detected. Replay output would have failed the current policy. Operator notified.") lives at `web/components/skycot/audit/ReplayHeader.tsx:32–34` and renders for any action ID whose `mockVerdict()` resolves to `drifted-misaligned` (last char-code mod 3 == 2) — including unknown 32-char-hex IDs after a real gateway 404. I would classify this as a P0 demo-blocker for the v1.0 MP4 recording, NOT as low-quality signal that the operator was right to reject in bulk; the prior audit document's failure was likely the surrounding noise (87 findings, much of it dressing flagged as function), not this specific finding.

---

## Resolved since last audit

The prior audit's broad complaint about "no DataSource discriminator on approvals" has been resolved: `web/lib/api/approvals.ts` now exposes `source: "live" | "mock-offline" | "mock-auth-bypass"` and gates the mock fallback on `isDemoMode()`. The same hardening has not been applied to the **replay** endpoint client — F67 remains.

---

## Orientation score: 52 / 100

The page designs themselves are well-built and faithful to the locked specs when seeded data is present. The infrastructure for honest mock-fallbacks (the `source` discriminator + `isDemoMode()` gate) exists for the approvals and audit feed paths. But:

- The replay route's fabrication path is the single highest-leverage bug remaining for the v1.0 demo MP4 — fix that one finding and Scene 6 stops being a customer-trust risk.
- The audit-list / audit-detail ID disagreement (F2) makes the Scene 6 drill-in narrative literally unrecordable against the current backend.
- Without `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` set, the operator cannot record the locked ops-dashboard at all (F3, F8, F9). The first thing to verify before pressing record is that flag.

If F1 + F2 + F3 are addressed, the score jumps to ~85; the remaining items are P1/P2 polish that don't break the script.
