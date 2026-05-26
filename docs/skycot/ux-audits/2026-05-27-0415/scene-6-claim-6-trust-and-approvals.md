# Scene 6 (Claim 6 — Multi-channel + multi-tenant + audit trust) — UX audit

**Date**: 2026-05-27 04:15 AEST (audit run at ~04:18 local).
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 + /approvals).
**Method**: live SPA at `http://127.0.0.1:3000` (Next dev, demo-mode + auth-bypass both enabled per `.claude/launch.json`), gateway at `http://127.0.0.1:7400` (schema v83, 13h uptime). Where the Claude Preview MCP froze mid-walk, I switched to source-code walk + targeted curl probes against the live gateway. The earlier audit at `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md` was reviewed to verify what has shipped since.
**Prior auditor verdict**: 52 / 100 (3 P0s, 6 P1s).

---

## TL;DR

Scene 6 is in materially better shape than 2 days ago. The three P0 blockers from the prior audit (`/audit/action/{id}/replay` fabrication, audit-list/detail ID disagreement, `/approvals` empty-state short-circuit) have all been resolved. Replay client now gates the mock fallback on demo-mode AND the canonical sample table, audit-detail returns real provenance with a real `verdict:"same"` self-hash recompute, and `/approvals` always renders the locked E-ops-dashboard.

The remaining concerns are **architectural / demo-fidelity**, not crash blockers:

- **1 new P0**: when the SPA runs in demo+auth-bypass (i.e. the configuration the v1.0 MP4 will record in), the audit drill-in and approval-decision flows **never reach the gateway** — they short-circuit to sample data. The agent runtime is never exercised end-to-end by the on-camera click-through. The "Real provenance" claim of Scene 6 step 7 is, in this configuration, theatre that the camera sees.
- **2 P1s** carried over: hash-chain "prev → this" affordance still doesn't exist (single-row self-hash only); live-gateway audit data is 100% `unknown / http.request` self-traffic so the audit page is unusable without demo-mode.
- **5 P2/P3s** new or carried: dual `/approvals` vs `/` surfaces both labelled "Approvals" but show different aesthetics, SLA timer can print "2h 60m left", `placeholderAuditDetail` invents a `claude-opus-4-7` model id, Discord channel is referenced nowhere in the SPA despite shipping in the gateway, `Active reviewers` KPI hardcoded to `—`.

If the operator records the MP4 in the default demo-mode + auth-bypass config and never narrates "this is hitting the real backend" during the drill-in, **Scene 6 is recordable today**. If the operator wants the live gateway path on camera (e.g. to satisfy due-diligence "show me the audit row that proves you just approved"), Finding F1 below is the next gating fix.

---

## Resolved since 2026-05-25 audit

| Prior | Status | Evidence |
|---|---|---|
| F1 / F67 — `/audit/action/{id}/replay` fabricates forensic content for unknown IDs | RESOLVED | `web/lib/api/replay.ts:242-272` — `sampleOrFailed()` gates the mock fallback on `isDemoMode() OR isAuthBypassed()` AND the ID being in `findSampleReplay()`'s canonical TABLE. Otherwise a `{kind:"failed", reason: "Action not found — the replay sandbox only covers the seeded demo personas."}` envelope renders `<FailedView>` (not a fabricated red banner). |
| F2 — audit-list and audit-detail endpoints disagree | RESOLVED | Curl: `GET /api/v1/audit/019e65172b667083bea5ab07647eb8ee` returns real `detail.decision_chain` + `provenance.hash_chain_anchor` + `replay` returns `verdict:"same"` with matching `expected_self_hash == actual_self_hash`. Both endpoints now accept the gateway-issued ID round-trip. |
| F3 — `/approvals` empty-state short-circuit | RESOLVED | `web/app/(app)/approvals/page.tsx:34-46` always renders `<ApprovalsOpsDashboard>`; the dashboard's KPI strip + tier sections handle the 0-state internally. Empty state ("Open the inbox" CTA routing to `/`) is gone. |
| F4 — 7-day approval-rate ring hardcoded 78% | RESOLVED | `ApprovalsOpsDashboard.tsx:91-102, 168, 218-243` derives `approvalRate` from `items.filter(item => item.decision != null)` and only renders the ring when `approvalRate != null`. Sub-label uses real percentages. |
| F5 — `$ at risk` donut fixed 82% visual fill | RESOLVED | `ApprovalsOpsDashboard.tsx:196-203` replaces the donut with `MetricStatCard tone="risk"` — value only, no fake conic gauge. |
| F6 — Approval rows unclickable | RESOLVED | `ApprovalsOpsDashboard.tsx:303-310, 26-35` — `<ApprovalRow>` is a real `<button>` with `onClick={() => window.location.assign(actionHref)}`, mapping each `ap_01H9Z*` sample approval to its `act_*` audit ID via the `SAMPLE_APPROVAL_ACTION_IDS` table. |
| F10 — Approval-rate fallback "78%" + "7-day baseline" sub-label | RESOLVED | `ApprovalsOpsDashboard.tsx:138-145` — value is now `"—"` and sub-label `"no resolved approvals"` when no decisions exist. |

---

## Headline blockers (new)

### F1 [P0] — In demo + auth-bypass config the entire audit/replay/approve path bypasses the gateway

`web/.claude/launch.json` launches the SPA with both `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` AND `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. In this state:

- `web/lib/api/audit.ts:346-348` short-circuits the audit feed to `mockPage()` — no `GET /api/v1/audit` call ever fires.
- `web/lib/api/audit-detail.ts:99-101` short-circuits `fetchAuditDetail()` to `mockDetail()` — no `GET /api/v1/audit/{id}` call.
- `web/lib/api/replay.ts:259-261` short-circuits `replayAction()` to `sampleOrFailed()` — no `POST /api/v1/audit/{id}/replay` call.
- `web/lib/api/approvals.ts:207-209, 249-251` returns `{ok: true, source: "mock-auth-bypass"}` instantly on `submitDecision` / `submitUndo` — the gateway's `/api/v1/approvals/{id}/decide` route is never hit.

This is **theatre the camera sees** if Scene 6 narrates "and the real audit trail proves it". The locked runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) and `MISSION.md` claim 6 lean on "every action is a real audit row, hash-chained, replay-verifiable". In the recording config, none of that round-trips to live SQL — the user clicks a row, the SPA loads `SAMPLE_AUDIT_DETAILS` from a hand-shaped TypeScript object, and the screen shows pre-baked seed text.

Per the function-vs-dressing rule: the audit-row drill-in IS function-side (it claims to do cryptographic provenance lookup against the real audit chain). Showing canonical seed JSON here is a function-side fabrication for the on-camera click-through. The narration must either (a) avoid the "this hits the real backend" claim, or (b) the SPA must record against a non-auth-bypass mode so the real gateway path fires.

**Reproduction**:
- Open `http://127.0.0.1:3000/audit` (auth bypass + demo mode on).
- Snapshot the network panel — there is no `GET /api/v1/audit?range=7d`.
- Click any row, snapshot network panel for the drill-in — there is no `GET /api/v1/audit/{id}`.
- Click "Replay" — there is no `POST /api/v1/audit/{id}/replay`.

**Suggested fix**: introduce a third mode `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` that keeps the seeded copy + persona avatars but routes audit/replay/approve through the real gateway (which already has matching seed data via `argo demo seed`). Record the MP4 in this mode. Auth-bypass + demo-mode stay as today for fast local UI work.

**File paths**:
- `web/lib/api/audit.ts:346`
- `web/lib/api/audit-detail.ts:99`
- `web/lib/api/replay.ts:259`
- `web/lib/api/approvals.ts:207`
- `.claude/launch.json:5-14`

---

## All findings (in order, grouped by surface)

### `/approvals` (E-ops-dashboard)

#### F2 [P1] — SLA timer renders impossible "2h 60m left"

`web/components/skycot/approvals/SLABar.tsx:46-50` computes `minutes = Math.round((ms - hours * HOUR) / 60_000)`. When `ms` is e.g. 2h 59m 30s the rounding gives `minutes = 60`, producing the literal string "2h 60m left". I observed this in the live screenshot of the MEDIUM tier — a refund row read "2h 60m left" rather than the intended "3h left". The bug is sub-frame-rate visible (it only fires for the ~30s before each whole-hour boundary), but the screen-recording camera will catch one.

**Repro**: leave `/approvals` open and watch any approval whose `slaExpiresAt` is currently 2-3h out cross a half-minute boundary near 2h 59m.

**Suggested fix**: change line 47-50 to `Math.floor` or round-with-carry into hours. One-line fix.

#### F3 [P2] — `Active reviewers` KPI is hardcoded `—` regardless of state

`ApprovalsOpsDashboard.tsx:146-151` always sets `value: "—"` and `sub: "reviewer roster pending"`. Camera-side this is dressing (no Scene 6 narration touches it), but it's a visible KPI tile in the strip that says "we haven't shipped this yet" in plain English. Either hide the tile or wire it to the real reviewer roster query (the gateway's `/api/v1/team` returns role-bearing humans).

**File**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-151`

#### F4 [P2] — Two screens labelled "Approvals" with different aesthetics

`web/components/skycot/nav-items.tsx:23` — the AppShell sidebar's "Approvals" row routes to `/`. The typed URL `/approvals` renders the locked E-ops-dashboard. The two surfaces share a name + page title ("Approvals · SKYCOT") but render completely different React trees:

- `/` (`app/(app)/page.tsx`): day-grouped inbox with bundles, batch decisions, redaction policy, filter chip rail, notifications bell, trust-ratchet sections.
- `/approvals` (`app/(app)/approvals/page.tsx`): brand-loud hero + ops-dashboard with KPI strip, severity tiers, $-at-risk + confidence + 7-day-rate donuts.

A buyer typing `/approvals`, then clicking the sidebar Approvals row, lands on a visibly different screen with the same label. Scene 6's recording script needs to pick one and stay on it. Per the locked design memory `project_approvals_page_design_locked`, the ops-dashboard is the canonical surface — recommend pointing the sidebar to `/approvals` and either retiring `/` or relabelling it ("Decide queue" / "Inbox").

#### F5 [P2] — Demo mode banner sits sticky on every Scene 6 frame

`web/components/skycot/DemoBanner.tsx:35-46` — the gradient banner ("Demo mode — showing sample data. Sign in to see your own workspace.") sticks to the top of every in-app page when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set. For a POC/investor demo this is arguably honest framing, but it visually competes with the BrandLoudHero on every page transition. The dismiss button writes to localStorage, so pre-recording the operator can clear it once. Worth confirming the recording starts from a state with the banner dismissed.

### `/audit` (Etherscan-style timeline)

#### F6 [P1] — Hash-chain "prev → this" affordance still does not exist

The Scene 6 brief calls hash-chain hover "the centerpiece". `web/components/skycot/ActivityRow.tsx:225-264` `HashCell` still shows only the row's own `selfHash` with a click-to-copy tooltip. There is no rendering of `parentHash` (even though `web/components/skycot/audit/AuditDayGroup.tsx:91` plumbs `parentHash: row.parentHash` through to the activity) and no chain-edge visual.

Worse, the seed-data hash is still `sampleHashForIndex(index)` at `web/lib/sample-audit.ts:155-158` — eight FNV-shaped chunks per row, with comment "matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade". No row depends on any previous row's hash. If the camera lingers on the hash column AND the narration claims "every row hash-chained", this is function-side theatre.

**Suggested fix**: render a second hash row in `HashCell` showing `0xparentSlice ↗` linked to the previous row's drill-in; in `sample-audit.ts`, replace `sampleHashForIndex` with a real recursive chain (`prevHash + rowContent → SHA-256`). Both are bounded changes (~30 LOC).

#### F7 [P1] — Live-gateway audit feed is unusable: 29 rows all `unknown / http.request` from SPA self-audit

Curl `GET /api/v1/audit?range=7d` against the live gateway returns 29 rows where `worker.name == "unknown"`, `verb == "http.request"`, and `action_type` is one of `audit.replay` or `audit.detail.viewed`. This is the SPA's own audit telemetry of itself. There are no real agent-turn rows in the live database for the demo tenant.

Implication: if the operator records the MP4 in any non-demo-mode + non-auth-bypass configuration (e.g. cookie-auth signed-in), Scene 6's audit page shows only meta-traffic — no Mariah/Peta/Priya/Jamal/Nori rows the runbook narrates. Demo-mode is mandatory for Scene 6 to be recordable today. Belt-and-braces: confirm the recording session has `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` set before pressing record.

This also relates to F1: the live gateway HAS real audit infrastructure (it returns a real `decision_chain`, `provenance.hash_chain_anchor`, `replay verdict:"same"`, the cross-tenant 404 wall is enforced) — but there's no seed data to demonstrate it. Running `argo demo seed --tenant <demo>` to fill the live audit table would let the camera record against the real path, satisfying both F1 and F7.

#### F8 [P3] — `placeholderAuditDetail()` invents a real-shaped `claude-opus-4-7` model id

`web/lib/sample-audit-detail.ts:285-310` — `placeholderAuditDetail` is the last-resort fallback (used when the gateway returns a row in the list whose detail endpoint 4xxs and demo-mode finds no matching seed). It synthesises a fake provenance block with `model: { id: "claude-opus-4-7", version: "2026-04-30" }`, `promptTemplate: { id: "generic.placeholder", hash: "placehold00000000" }`, etc.

This is unreachable in the SPA's current demo+bypass config (we never fetch from the gateway), but if the auth-bypass shortcut is ever lifted, a buyer drilling into any non-canonical action ID lands here and sees a card that looks like real provenance. Per the function-vs-dressing rule, a "Provenance" card is function-side (it claims to identify the exact code/prompt/skill/policy that produced the action). Synthesising "claude-opus-4-7" provenance text is a small fabrication.

**Suggested fix**: change the placeholder to leave all provenance fields blank so the `<ProvenanceBlock>` empty-state ("Provenance unavailable — this is a meta-event") renders instead of a fake card.

### `/audit/action/{id}` (drill-in)

#### F9 [P2] — Replay client never reaches gateway in auth-bypass — gateway adapter code is dead in demo mode

`web/lib/api/replay.ts:259-261` returns `sampleOrFailed(actionId)` whenever `isAuthBypassed()` is true. This skips the entire `adaptGatewayReplay(await apiPost(...))` path, even though the gateway DOES return well-shaped `{verdict:"same", expected_self_hash, actual_self_hash, ...}` data (verified by curl). The adapter is well-written and contract-locked but exercised by tests only — never by the running SPA in demo recording.

This means the replay UI's three success verdicts (`reproducible | drifted-aligned | drifted-misaligned`) are all driven by `findSampleReplay()`'s hand-crafted TABLE, NOT by real gateway state. Coupled with F1, it widens the theatre surface.

**File**: `web/lib/api/replay.ts:259`

#### F10 [P2] — `AuditActionNotFound` is only reachable via gateway 404 (so today: never)

`web/components/skycot/audit/AuditActionNotFound.tsx` ships a clean in-shell 404 with an action-id echoed back. In the current demo+bypass config it's unreachable — any unknown action ID falls into `placeholderAuditDetail()` (F8) instead of the not-found page. Worth a test pass to confirm the not-found surface still works end-to-end when bypass is lifted.

### Cross-tenant isolation

#### F11 [P3] — Gateway tenant isolation IS enforced on `x-argo-tenant-id`; SPA never sends it

Initial curl probe with `X-Tenant-Id: tenant_b_other` returned the same audit row — I almost flagged this as a cross-tenant leak. The actual canonical header is `x-argo-tenant-id` (per `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:192-193` and the matching replay route). Re-curling with the correct header:

```
GET  /api/v1/audit/019e65172b667083bea5ab07647eb8ee  → 404 (not found)
POST /api/v1/audit/019e65172b667083bea5ab07647eb8ee/replay → 404 (not found)
GET  /api/v1/audit?range=7d                          → DIFFERENT items
```

Isolation is working correctly. **However** — `web/lib/api/client.ts:155-177` never sends an `x-argo-tenant-id` header from the SPA. The gateway derives the tenant from the bearer-token / session-JWT principal via `extract_tenant(req)`. This is correct for single-workspace-per-session, but if the SPA ever adds a workspace switcher (multi-tenant identity per user), the client will need to forward an explicit `x-argo-tenant-id`. Not a blocker; flag for the multi-workspace track when it lands.

### Multi-channel touchpoints (voice / SMS / Discord)

#### F12 [P2] — Discord is referenced nowhere visible in the SPA, despite shipping in gateway

The gateway has Discord interaction signature verification (PR #2222 per the CLAUDE.md status block). In the SPA:

- `web/components/ui/ChannelBadge.tsx:6` `ChannelKind = "slack" | "email" | "sms" | "voice"` — no Discord.
- `web/lib/sample-approvals.ts:17` `ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp"` — no Discord.
- `web/lib/notify-channels/registry.ts:148` Discord is listed in the notify-channels registry (the connector-config UI), but its absence from the badge / approval / audit ChannelKind unions means a real Discord approval would render with an unknown-channel fallback.

If Scene 6's Claim 6 vocabulary includes Discord, the badge + types need extension. If it doesn't, the gateway investment is invisible to the demo.

#### F13 [P3] — Voice + SMS approvals exist in sample data but no channel badge surfaces in `/audit` rows

Sample data has `channel: "voice"` (Jamal refused PII to unverified caller) and `channel: "sms"` (Mariah booked viewing for Tomás). The audit feed renders these as ActivityRow entries — but `ActivityRow.tsx` doesn't render a channel badge anywhere on the row. The audit-detail page renders `<ActionSummaryHero>` which does include channel info, but the list view is channel-blind. If Scene 6 wants to show "your AI teammates work across voice, SMS, Slack" at-a-glance on the audit feed, the badge needs to be added to the row.

### General

#### F14 [P3] — `approve` button in demo+bypass shows green tick before any audit row exists

`web/lib/api/approvals.ts:207-209` returns `{ok: true, source: "mock-auth-bypass"}` synchronously. The optimistic-mutation UI flips to "decided" and the row disappears. The user's next instinct is to navigate to `/audit` and find the decision — but there's no new row because the gateway was never called. If the operator narrates "and now you can see the audit trail of that decision", the audit feed shows no new row (it shows the pre-seeded `SAMPLE_AUDIT_ROWS` which are static). This is the same root cause as F1; flagging separately because it breaks a specific click-narrate-click loop the runbook may want to demonstrate.

---

## Orientation score: 78 / 100

Up from 52 / 100 two days ago. The three prior P0s have all shipped real fixes; the audit/approvals trust narrative is functionally sound and the visual surfaces are well-built. What's left is a single architectural decision: **does Scene 6's on-camera click-through run through the real gateway, or against canonical sample data?** If "sample data" is acceptable (because the runbook narration is careful about the framing), nothing else here is a recording-blocker. If the operator wants the live path on camera for due-diligence credibility, F1 + F7 are the next pair of fixes.

Beyond Scene 6 itself, the hash-chain "prev → this" affordance (F6) is the one remaining visible promise the audit page hasn't shipped against — likely a single-PR fix that materially strengthens Claim 6's "forensic-grade" framing.

---

## Notes on method

The Claude Preview MCP locked up after ~5 successful interactions (eval + screenshot both timed out at 30s, no console or server errors logged). I switched to source-code walk + curl probes against the live gateway to finish the run. All 14 findings are reproducible from either a clean SPA open or a single curl command; reproduction steps are inline where the finding isn't trivially obvious.
