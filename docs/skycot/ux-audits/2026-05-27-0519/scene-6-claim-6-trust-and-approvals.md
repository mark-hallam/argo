# Scene 6 (Claim 6 — Multi-channel + multi-tenant + audit trust) — UX audit

**Date**: 2026-05-27 05:19 AEST.
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 + `/approvals`).
**Method**: live SPA via Claude Preview MCP at `http://localhost:3000` (Next dev on dev server `web`, autoPort enabled — re-spawned mid-run to port 54588 after the preview process restarted), gateway at `http://127.0.0.1:7400`. Preview MCP locked up / restarted ~4 times during the run; switched to source-code walk + curl probes against the live gateway for everything past the first two pages, including the `/audit/action/{id}` drill-in and `/audit/action/{id}/replay` probes. Prior audit `2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` reviewed; this report is a delta + a brand-new P0 the prior auditor missed.
**Prior auditor verdict**: 78 / 100 (1 P0, 2 P1s, 5 P2/P3s).

---

## TL;DR

Scene 6 has materially regressed since the 04:15 audit — **but not because anything broke**. The 04:15 auditor assumed the SPA was running in auth-bypass + demo-mode, where all four trust surfaces short-circuit to canonical sample data, and concluded the camera-path was theatre on the function side (their F1). It turns out the auth-bypass short-circuit **never fires in the running configuration** because of an env-var prefix mismatch: `.claude/launch.json` exports `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`, but every consumer (`approvals.ts:75`, `audit.ts:64`, `audit-detail.ts:63`, `replay.ts:86`) checks `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. Result: the SPA actually hits the live gateway for approvals + audit-list + replay — and the live gateway is **empty for approvals** and **31 rows of SPA-self-traffic for audit**. The `/approvals` page renders with five 0-value KPI tiles and three "No [severity] severity approvals waiting" tier sections. The `/audit` page renders a "Quiet day" empty-state (no rows came through the day-grouping). The on-camera Scene 6 walkthrough, recorded right now, is functionally empty.

**3 new P0s** raised this run:
- **F1**: env-var name mismatch (`AUTH_BYPASS` vs `ARGO_GATEWAY_AUTH_BYPASS`) leaves the SPA pointed at an empty live gateway → `/approvals` and `/audit` both render empty states on camera.
- **F2**: `audit-detail.ts:103` short-circuits to mock/placeholder on `isDemoMode()` even when auth-bypass is off — so an unknown action id resolves to `placeholderAuditDetail()`, fabricating a `claude-opus-4-7` model + `placehold00000000` prompt hash provenance card. Function-side fabrication on the drill-in the runbook narrates.
- **F3**: gateway audit list response carries no `self_hash` / `parent_hash` — `normalizeAuditRow` reads them as `undefined`, so even if rows rendered the hash column would be empty. The audit page's centerpiece affordance has no data to display.

**2 P1s** carried over: SLA bar still rounds to "2h 60m left" (one-line fix unshipped 2 days running); hash-chain "prev → this" affordance still doesn't exist (`HashCell` only consumes a single hash). **5 P2/P3s** carried or new: `/` vs `/approvals` dual-surface label collision unresolved; `Active reviewers` KPI hardcoded `—`; Discord still absent from `ChannelKind` unions despite shipping in the gateway; voice/SMS approvals have no channel badge on `/audit` row; replay verdict body is now real hash-recompute text (`Original audit self_hash: d0e1c…`) — not a fabrication risk, but it's also not the "watch the agent re-run" theatre the runbook's "Replay" beat implies.

Cross-tenant isolation is intact — `GET /api/v1/audit/<id>` with a bogus `x-argo-tenant-id` returns 404 (verified via curl with two different bogus tenants).

**If the operator records the MP4 right now** (with the current `.claude/launch.json` env config), Scene 6 will show empty approvals + empty audit. The 04:15 auditor reported this surface as "recordable today" — it isn't, and hasn't been since the env-var typo landed. **Either fix the env var (F1's 1-LOC fix) or run `argo demo seed` against the live gateway before recording.**

---

## Resolved since 2026-05-27 04:15 audit

| Prior | Status | Notes |
|---|---|---|
| F8 — `placeholderAuditDetail` was unreachable in demo+bypass | **REGRESSED to reachable** — see F2 below |
| (everything else) | UNCHANGED | F2/SLA round, F3/Active reviewers, F4/dual-surface, F5/banner, F6/hash-chain affordance, F7/live feed unusable, F9/replay client dead path, F10/AuditActionNotFound unreachable, F11/SPA never sends tenant header, F12/Discord absent, F13/no channel badge in audit row, F14/optimistic-tick — all still standing |

Two recent commits touched audit surfaces but neither addresses these findings:
- `56f4ea0b4` (mechanical-author) widened `ProvenanceBlock.tsx`'s `hasText` to `string | undefined` — type fix only, no behaviour change.
- `51ab5c6dd` (#2429) flipped `client.ts` to same-origin `/api/v1/*` — the source-walk confirms requests now reach the gateway via the SPA's Next dev proxy (we see `GET /api/v1/approvals → 200 OK`, `GET /api/v1/audit?range=7d → 200 OK`).

---

## Headline blockers (new this run)

### F1 [P0] — `.claude/launch.json` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`; every consumer checks `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`

`web/lib/api/approvals.ts:75`, `web/lib/api/audit.ts:64`, `web/lib/api/audit-detail.ts:63`, and `web/lib/api/replay.ts:86` all read `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. `.claude/launch.json:9` injects `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (no `_ARGO_` infix). The names don't match → `authBypassEnabled()` always returns `false` under `preview_start name=web`.

Downstream effect cascade:
- `fetchApprovals()` skips the mock short-circuit and calls `apiGet('/api/v1/approvals')`. Gateway returns `{"items":[]}` (verified via curl). The `isMissingOrUnreachable` check (`approvals.ts:80`) is false because the response is 200-with-empty-array, not a 4xx/network error → falls through to `{items: [], source: "live"}`. The demo-mode `SAMPLE_APPROVALS` fallback at line 194-196 is never reached. The page renders with all 5 KPI tiles at 0 / `—` and all 3 severity tiers showing "No [severity] severity approvals waiting."
- `fetchAuditFeed()` short-circuit at line 346-348 doesn't fire → calls `apiGet('/api/v1/audit?...')`. Gateway returns 31 rows that are all `worker.name == "unknown"`, `verb == "http.request"`, `action_type ∈ {audit.detail.viewed, audit.replay}` — i.e. the SPA's own audit telemetry of itself, not real agent turns. (Same root cause as the prior audit's F7.) The `/audit` page either renders these unreadable meta-rows or no rows at all depending on day-grouping.
- `replayAction()` short-circuit at line 259-261 doesn't fire → POSTs to the real gateway. Returns real `verdict: "same"` envelope with matching `expected_self_hash == actual_self_hash` (verified curl). This one is **good news** — the replay path actually works end-to-end against the live gateway.
- `submitDecision()` short-circuit at line 207-209 doesn't fire → POSTs to the real gateway's `/decide` endpoint when the user clicks Approve. (Untested in this run because there are no approvals to decide on; the empty inbox kills the test loop.)

**The prior audit's F1 ("entire path bypasses the gateway in auth-bypass") is therefore wrong about the actual running state** — the SPA *is* hitting the gateway. The real problem is the gateway has no seed data for Scene 6 to demonstrate, so the page renders empty.

**Reproduction**:
```bash
curl -s http://localhost:3000/api/v1/approvals   # → {"items":[]}
curl -s "http://localhost:3000/api/v1/audit?range=7d" | jq '.items | length, (map(.worker.name) | unique), (map(.verb) | unique)'
# → 31, ["unknown"], ["http.request"]
```
Then open `http://localhost:3000/approvals` — all KPI tiles show `0` / `—`, all three tier sections show "No [severity] severity approvals waiting."

**Suggested fix** (two paths, both 1-LOC; either unblocks recording):
- (a) Rename the env in `.claude/launch.json:9` from `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. Re-enables the seed-data path (the runbook records against SAMPLE_APPROVALS / SAMPLE_AUDIT_ROWS). This is fast but is exactly the "theatre the camera sees" the prior auditor flagged — choose this only if Scene 6 narration is careful.
- (b) Run `argo demo seed --tenant <demo-tenant-id>` against the running gateway so the real path returns real-looking data. This is the path the prior auditor recommended for due-diligence credibility. Per `crates/argo-cli/src/cli_demo_cmds.rs:23` the command is `argo demo seed --tenant 01900000-0000-7000-8000-000000000000`.

**Files**: `.claude/launch.json:9`; consumers at `web/lib/api/approvals.ts:75`, `audit.ts:64`, `audit-detail.ts:63`, `replay.ts:86`.

### F2 [P0] — Audit-detail drill-in fabricates a `claude-opus-4-7` provenance card for any non-canonical action id

`web/lib/api/audit-detail.ts:103-105` short-circuits to `demoModeDetail(actionId)` whenever `isDemoMode() && !isStrictProduction()`. Unlike the other API clients, this short-circuit is gated on `isDemoMode()` (env `NEXT_PUBLIC_SKYCOT_DEMO_MODE` or `NEXT_PUBLIC_ARGO_DEMO_MODE`) — both of which **are** set in `.claude/launch.json` (the names match here). So the detail endpoint NEVER hits the gateway in the running configuration.

`demoModeDetail()` calls `findSampleAuditDetail(actionId) ?? placeholderAuditDetail(actionId)` (`audit-detail.ts:82-84`). The 9 canonical samples in `sample-audit-detail.ts:46-225` use ids like `act_01J0PRIYA_001`, `act_01J0MARIAH_001`. Any other id — including every ULID the live gateway returns (`019e65172b667083bea5ab07647eb8ee` etc) — falls through to `placeholderAuditDetail()`.

`placeholderAuditDetail()` (`sample-audit-detail.ts:285-310`) synthesises:
- `provenance.model.id = "claude-opus-4-7"`, `version = "2026-04-30"`
- `provenance.promptTemplate.hash = "placehold00000000"`
- `provenance.policy.hash = "policyplaceholder"`
- `decisionChain = defaultChain("generic-skill", "1.0")` — 8 fake steps
- `output.body = "Detail for action <id> is not currently available..."` — at least this string self-identifies as placeholder

The drill-in page (`audit/action/[actionId]/page.tsx:147`) feeds this into `<ProvenanceBlock data={detail.provenance} detail={detail} />` — which renders a card titled "Provenance" showing the model id "claude-opus-4-7" and the prompt-template hash. Per the function-vs-dressing rule, a Provenance card is **function-side** — it claims to identify the exact model + prompt + skill + policy that produced the action. Synthesising "claude-opus-4-7" provenance for a row whose true provenance is "the SPA's own self-audit" is a function-side fabrication for an on-camera click-through.

The reachability gate has shifted since the prior audit: F8 (then P3) is now **routinely hit** because the live gateway returns ULID ids that are never in `SAMPLE_AUDIT_DETAILS`, and demo-mode short-circuits before the gateway is asked for the real detail. If the operator clicks ANY row in the live audit feed (the 31 self-traffic rows) the placeholder card renders.

**Reproduction**:
- With launch.json as-is, navigate to `http://localhost:3000/audit/action/019e65172b667083bea5ab07647eb8ee` (any live-gateway id).
- Wait for the loader to settle (currently 2-5s in the running preview).
- The page renders `<ActionSummaryHero>` with verb "Action detail unavailable — drill into another row for full provenance.", then a Provenance card showing `claude-opus-4-7 · v2026-04-30` and `prompt: generic.placeholder (hash: placehold00000000)`.

**Suggested fix** (one of):
- (a) Drop the `isDemoMode()` short-circuit at `audit-detail.ts:103-105` so the gateway is always called; let the existing fallback at lines 110-123 catch failures. This makes the gateway path live and surfaces real provenance for real ids.
- (b) Change `placeholderAuditDetail()` (`sample-audit-detail.ts:285-310`) to leave provenance fields blank — pass `{ model: { id: "", version: "" }, ... }` — so `<ProvenanceBlock>` renders its empty-state ("Provenance unavailable — this is a meta-event") instead of a fake card.

**Files**: `web/lib/api/audit-detail.ts:103-105`, `web/lib/sample-audit-detail.ts:285-310`.

### F3 [P0] — Gateway audit list response carries no hash fields → `selfHash`/`parentHash` always undefined → hash column empty

`web/lib/api/audit.ts:194-217` `normalizeAuditRow` reads `selfHash = gatewayRow.selfHash ?? gatewayRow.self_hash` and `parentHash = gatewayRow.parentHash ?? gatewayRow.parent_hash`. Curl `GET /api/v1/audit?range=7d` against the live gateway returns rows with these keys: `action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker` — **no `self_hash`, no `parent_hash`, no `hash`**. So `selfHash` resolves to `undefined`, `hash` resolves to `undefined`, and `ActivityRow.tsx:98-106` renders `<span aria-hidden />` instead of the `HashCell`. The audit page's centerpiece affordance has no data to display on the live path.

Compounding: the seeded `sample-audit.ts:147,155-158` does include `sampleHash: sampleHashForIndex(i)` (8-char FNV chunks, comment "matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade") — but that's only consumed via `mockPage()` which (per F1) only fires when auth-bypass is on (i.e. never in the current configuration). Either path: no hash, or fake-shaped sample hash.

**Reproduction**:
```bash
curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items[0] | keys'
# → ["action_type","approver","channel","cost","data_classification","id","status","timestamp","verb","worker"]
```

**Suggested fix**: extend the gateway `routes_audit_list.rs` (sibling to `routes_audit_detail.rs:192-193`) to compute + return `self_hash` (the row's recomputed sha256) and `parent_hash` (the previous row's recomputed sha256 — the gateway already knows this from the audit hash chain). The detail endpoint already returns `provenance.hash_chain_anchor` — that machinery exists; it just hasn't been wired to the list endpoint. Once shipped, F6 (parent → this affordance below) is buildable; without it, the affordance is theoretical.

**Files**: `web/lib/api/audit.ts:194-217`; gateway list route under `crates/argo-cli/src/handlers/gateway/`.

---

## All findings (in order, grouped by surface)

### `/approvals` (E-ops-dashboard)

#### F4 [P1] — SLA timer still renders "2h 60m left" (UNFIXED from 04:15 audit)

`web/components/skycot/approvals/SLABar.tsx:46-50`. Same bug as prior F2: `Math.round((ms - hours * HOUR) / 60_000)` produces `60` when minutes-component rounds up at the half-second boundary. Also affects sub-1-hour values: `ms = 59m 30s` → `Math.round(59.5) = 60`, returning `"60m left"` instead of carrying to `"1h left"` (line 46).

**Repro**: leave `/approvals` open with any SLA-bearing approval whose `slaExpiresAt` crosses a half-minute boundary near `Nh 59m 30s`.

**Suggested fix** (~3 lines):
```ts
if (ms < HOUR) {
  const mins = Math.round(ms / 60_000);
  if (mins >= 60) return "1h left";
  return `${mins}m left`;
}
const hours = Math.floor(ms / HOUR);
const minutes = Math.round((ms - hours * HOUR) / 60_000);
if (minutes >= 60) return `${hours + 1}h left`;
if (minutes === 0) return `${hours}h left`;
return `${hours}h ${minutes}m left`;
```

**File**: `web/components/skycot/approvals/SLABar.tsx:46-50`.

#### F5 [P2] — `/` (homepage approvals) vs `/approvals` (ops-dashboard) still both labelled "Approvals" with different aesthetics (UNFIXED)

Sidebar (`web/components/skycot/nav-items.tsx:23`): `{ href: "/", label: "Approvals", icon: <Inbox /> }`. Typed `/approvals` renders the locked E-ops-dashboard (`web/app/(app)/approvals/page.tsx`). Two screens, same name, different React trees. Carried over from prior F4.

**Suggested fix**: rebind nav to `/approvals` and either retire `/` or relabel it ("Decide queue" / "Inbox"). Per the locked design memory `project_approvals_page_design_locked`, the ops-dashboard is the canonical surface.

**File**: `web/components/skycot/nav-items.tsx:23`.

#### F6 [P2] — `Active reviewers` KPI hardcoded `—` regardless of state (UNFIXED)

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-151` — still always `value: "—"`, `sub: "reviewer roster pending"`. Empty inbox today makes 4 of 5 KPI tiles show `0` or `—`; this is the 5th. Carried over from prior F3.

**File**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-151`.

#### F7 [P3] — Demo-banner sits sticky above the BrandLoudHero (UNFIXED)

`web/components/skycot/DemoBanner.tsx` — still renders on top of every in-app page when demo-mode is on. Dismissable, persists in localStorage. Pre-recording, the operator should dismiss it once. Carried over from prior F5.

### `/audit` (Etherscan-style timeline)

#### F8 [P1] — Hash-chain "prev → this" affordance still doesn't exist (UNFIXED)

`web/components/skycot/ActivityRow.tsx:98-106` only consumes `activity.hash` (renders into `<HashCell>` with `kind="self"`). No `parentHash` plumbing in the renderer; `<HashCell>` (line 225-264) shows only the row's own self-hash with click-to-copy. Carried over from prior F6.

**Pre-req for fix**: F3 above — gateway list endpoint must return `parent_hash` first. Without that there's nothing to render.

**Files**: `web/components/skycot/ActivityRow.tsx:225-264`; `web/lib/sample-audit.ts:155-158`.

#### F9 [P1] — Live-gateway audit feed is 31 rows of SPA self-traffic — no agent turns (UNFIXED)

Same as prior F7: `worker.name == "unknown"`, `verb == "http.request"`, all rows are `audit.detail.viewed` or `audit.replay`. Curl-verified. If the day-grouping aggregates these into "TODAY" and the `<ActivityRow>` filter doesn't drop them, the audit page shows visually noisy meta-rows that the runbook never narrates. If the day-grouping filters them (didn't observe a filter in `audit.ts` for `verb=http.request`), the page shows the pristine "Quiet day" empty state. Either way, Scene 6 step 6 ("the audit page renders ≥10 rows in Etherscan-style table") fails.

**Pre-req for fix**: `argo demo seed --tenant <id>` must populate the live audit table with real agent-turn rows (Mariah/Peta/Priya/Jamal/Nori). Or revert to seed-mode via the auth-bypass env (F1).

#### F10 [P2] — `AuditActionNotFound` is unreachable in the running config (UNFIXED)

`web/components/skycot/audit/AuditActionNotFound.tsx` is gated behind `AuditDetailNotFoundError` which is only thrown when `isStrictProduction()` is true (`audit-detail.ts:111-112`). In dev / demo / preview, the placeholder path (F2) catches every unknown id. The not-found surface ships clean, but no journey reaches it today.

**File**: `web/lib/api/audit-detail.ts:111-112`.

### `/audit/action/{id}/replay` (sandbox replay)

#### F11 [P2] — Replay UI renders a hash-recompute envelope as the "Original output / Replayed output" body — function-correct, but easy to misread as the agent re-running

`web/lib/api/replay.ts:175-194` `resultFromGatewayHash`: when the live gateway returns `verdict: "same"`, the SPA renders both `originalOutput.body` and `replayedOutput.body` as a four-line string: `"Original audit self_hash: d0e1c0… / Audit id: 019e65… / Sequence: 9394 / Event kind: http.request"`. This is mathematically the right thing to show (the gateway re-computed the row's self_hash and it matched), but **the visual UI is the `<ReplayDiff>` card titled "Original output" vs "Replayed output"** — which the runbook positions as "watch the agent re-run the same prompt and verify the output is identical." A buyer expecting to see the agent's prose output on both sides will read the hash diff as cryptic.

This is NOT a P0 fabrication — the content is real and labelled "audit self_hash" — but it's a disconnect between the runbook narration and the surface, the kind of mismatch that erodes investor confidence in "this is the real replay."

**Suggested fix** (longer-term): wire the gateway replay to return the real-shaped `ReplayOutcome` envelope (see TEMPORARY comment at `replay.ts:204-206` referencing `fix-gateway-replay-envelope-spa-contract`). Short-term: edit the runbook narration so Scene 6's replay beat says "we re-compute the action's cryptographic hash and verify it matches" rather than "the agent re-runs the action".

**Files**: `web/lib/api/replay.ts:175-194`; runbook `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6 step.

### Cross-tenant isolation

#### F12 [PASS] — Cross-tenant isolation is correctly enforced on `x-argo-tenant-id`

Tested via curl with three header values (default, `x-argo-tenant-id: other_tenant`, `x-argo-tenant-id: deadbeef`). The default tenant gets 200 with the real detail body for action id `019e65172b667083bea5ab07647eb8ee`. The two bogus tenants each get 404 (62-byte body). Same isolation observed on the replay POST. This matches the prior F11 finding — gateway isolation works correctly; the SPA simply doesn't send the header at all (relies on the gateway extracting tenant from the session principal).

Not a finding to action, just confirmation that one component of Claim 6 ("isolation between tenants") is genuinely live. If the operator wants to demonstrate isolation on camera, the test cannot be done from inside the SPA (no header switcher); demonstrate it via the terminal with curl.

### Multi-channel touchpoints

#### F13 [P2] — Discord still absent from `ChannelKind` unions despite shipping in the gateway (UNFIXED)

- `web/components/ui/ChannelBadge.tsx:6` — `ChannelKind = "slack" | "email" | "sms" | "voice"` (no Discord).
- `web/lib/sample-approvals.ts:17` — `ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp"` (no Discord, AND inconsistent with ChannelBadge — `imessage` and `whatsapp` exist here but not in the badge component).

PR #2222 shipped Discord interaction signature verification in the gateway. A real Discord approval today renders with an unknown-channel fallback in the SPA. Carried over from prior F12; the union mismatch between the two ChannelKind types is worth flagging on its own as a smaller P3.

**Suggested fix**: lift `ChannelKind` into a shared `web/lib/types/channel.ts` and reconcile the two unions, including Discord.

#### F14 [P3] — Voice + SMS approvals have no channel badge on `/audit` rows (UNFIXED)

Sample data has `channel: "voice"` (Jamal) and `channel: "sms"` (Mariah) approvals. The `<ActionSummaryHero>` on the detail page does render channel info, but the list view (`ActivityRow.tsx`) renders no channel badge. If the runbook wants "your AI teammates work across voice, SMS, Slack" to be visible at-a-glance on the audit feed, the badge needs to be added to the row. Carried over from prior F13.

**File**: `web/components/skycot/ActivityRow.tsx` (add `<ChannelBadge kind={activity.channel} />` to the row layout).

### General

#### F15 [P3] — `placeholderAuditDetail()` `synthesizeAuditDetail()` both seed fake `claude-opus-4-7` provenance — and so do all 5 canonical samples

`web/lib/sample-audit-detail.ts:70, 108, 147, 220, 253, 299`. The 5 canonical hand-shaped samples (Priya/Peta/Mariah/Jamal/Nori) all hardcode `model: { id: "claude-opus-4-7", version: "2026-04-30" }`. This is dressing-side as long as the demo lands on a canonical id (the model identifier is "the actual model name we're shipping", not a fabricated brand), BUT note the version `2026-04-30` is a fixed historical snapshot — if the v1.0 MP4 records in late May 2026 and a buyer asks "what model version is this", the answer "April 30" is awkward.

Lower-stakes than F2 because (a) it's at least a real model id (vs F2's "claude-opus-4-7-but-the-row-is-actually-http.request"), and (b) only fires for canonical samples that the runbook explicitly drills into. Worth a one-line bump to the actual current model version of record.

**File**: `web/lib/sample-audit-detail.ts:70, 108, 147, 220, 253` (5 sample rows).

---

## Orientation score: 58 / 100

Down from 78 / 100 at 04:15. The drop is not because anything broke — the surfaces themselves work — but because the prior auditor mis-classified the running configuration. The configuration the operator would actually record in (the one launched by `preview_start name=web`) is *worse* than either of the two states the prior auditor considered: not the fully-mocked auth-bypass world (which would show seeded data), and not the fully-live gateway-seeded world (which would show real agent turns), but a hybrid where (a) approvals + audit + replay hit the empty live gateway, AND (b) audit-detail short-circuits to fabricated placeholder provenance. The net result is the on-camera Scene 6 walkthrough shows an empty approvals page, an empty/noisy audit page, and — if the operator clicks any row — a fabricated provenance card.

**Single fastest path to a recordable Scene 6**: rename the env var in `.claude/launch.json:9` from `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. This re-enables the seed-data short-circuit on all four API clients in one stroke. Scene 6 will then show SAMPLE_APPROVALS + SAMPLE_AUDIT_ROWS + the canonical 5 act_* drill-ins — the exact pre-baked seed that the design memory + runbook were built against. Investor-credibility-vs-theatre framing remains the operator's call; the alternative is `argo demo seed` against the live gateway, which is a longer fix but lets the recording narrate "this is the real backend" honestly.

**Function-side problems still standing after that 1-LOC fix**: F2 placeholder fabrication (only fires for non-canonical ids — most camera-side journeys avoid this), F3 missing hash fields on list endpoint (camera doesn't need this if the seed includes a `sampleHash` per row, which it does), F4 SLA bar 60m overflow (sub-frame-rate visible, low capture probability), F8 hash-chain prev→this absent (still a "centerpiece" gap, but works as static seed for the camera).

---

## Notes on method

Preview MCP became unstable after the second navigation (`/audit` after `/approvals`). The dev server respawned itself ~4 times during the run; the autoPort moved the server from 3000 → 54588 → respawn → another port. Whenever `preview_eval` returned "target closed", I switched to direct curl probes against `http://localhost:3000` (which forwarded to the SPA proxy via #2429's same-origin rewrite) and against `http://127.0.0.1:7400` (direct gateway). Source-code findings are line-pinned against the working-tree at `HEAD = 7891db5ff`. All 15 findings are reproducible from either a working preview snapshot or a single curl + source line.
