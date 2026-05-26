# Scene 6 — Claim 6 (Multi-channel + multi-tenant + trust boundary) UX audit

**Date**: 2026-05-26 10:28 AEST
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 + `/approvals` + `/audit` + `/audit/action/[id]` + `/audit/action/[id]/replay`)
**Method**: source-walk + live curl against gateway on `127.0.0.1:7400` (schema v83, healthy) + driving the SPA via preview MCP on `:3000`. Demo banner visible in preview, so `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set in the dev env. Preview MCP wedged for some of the audit drill-in calls so part of the evidence is curl-against-gateway + source-walk; the live SPA proxy was reached and `/approvals` rendered the empty state under demo-mode (see F1).
**Prior audit referenced**: `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`. F67 is **FIXED** (see "Resolved since last audit"); the audit-list/detail ID disagreement (prior F2) is **FIXED**. A new function-side P0 has surfaced (F1).

---

## TL;DR

Scene 6 has **2 P0 demo-blockers**, **5 P1 visible issues**, and **3 P2 polish items**. The single highest-leverage bug is **F1: the SPA's replay client doesn't match the gateway's response shape** — the gateway now returns real replay data (verdict, expected/actual self-hash, diff) but the SPA expects a discriminated `{kind: "success" | …}` envelope and will fall through to the **red "side-effect leak detected" view for every successful replay** in dev-bypass / live mode. The drill-in click-through is recordable; the replay-button click on the destination page is not. Cross-tenant isolation is also unenforced at the gateway in dev-bypass — the same 10 audit rows are returned regardless of `X-Tenant-Id` header, and the detail endpoint serves any row to any caller (F2). Two well-classified wins since the prior audit: the F67 fabrication-on-unknown-id path has been closed in `web/lib/api/replay.ts`, and the list-vs-detail ID disagreement is gone.

---

## Headline blockers

### F1 [P0] — Replay client treats every successful gateway replay as a "side-effect leak detected" red banner

The gateway's replay endpoint now returns real data — I confirmed via `curl -X POST /api/v1/audit/<real-id>/replay`:

```json
{"actual_self_hash":"d37efe0d…","audit_id":"…","diff":{},
 "expected_self_hash":"d37efe0d…","kind":"http.request",
 "verdict":"same", …}
```

But the SPA client `replayAction()` casts that response to its discriminated `ReplayOutcome` envelope (`web/lib/api/replay.ts:101`):

```ts
return await apiPost<ReplayOutcome>(path);
```

The cast is a TypeScript-only no-op. The runtime object has `{verdict: "same", …}` with no `kind` field, so `outcome.kind === "success"`, `"unavailable"`, and `"failed"` all evaluate falsy in `OutcomeView` (`web/app/(app)/audit/action/[actionId]/replay/page.tsx:138-157`). Control falls through to:

```ts
return <LeakView details={outcome.details} />;
```

`LeakView` renders the red `role="alert"` shield card: **"Replay aborted — side-effect leak detected. This should never happen. A self-audit row has been written. Please report to operations."** with `outcome.details` being `undefined` (Etherscan-style monospaced "undefined" string at `replay/page.tsx:237-239`).

This is the camera-side worst-case trust violation: the demo says "see, we cryptographically verify every action," click Replay → the page renders a red alarm saying we leaked side effects. The buyer sees a more catastrophic fabrication than the prior F67's "policy regression" banner.

**Reproduction**:
1. Set `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is not needed; reproduce in live mode against the gateway
2. Navigate to `/audit` and click any row → `/audit/action/<id>`
3. Click "Replay" button → routed to `/audit/action/<id>/replay`
4. The page mounts, fires `useReplay()`, the gateway returns `200` with `verdict: "same"`
5. The page renders the red `LeakView` because the response shape doesn't match `ReplayOutcome.kind`

**Code paths**:
- Server response shape: `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs:81-93`
- Client envelope contract: `web/lib/api/replay.ts:27-46`
- Wrong-fallthrough render: `web/app/(app)/audit/action/[actionId]/replay/page.tsx:158`

**Suggested fix** (do not implement): adapt the gateway response to the SPA envelope in `replayAction()` — map gateway `verdict: "same"` → `{ kind: "success", verdict: "reproducible", result: …mapped… }`, and unknown gateway verdicts → `{ kind: "failed", reason: … }`. The mapping logic belongs in `replay.ts` so the discriminated union stays the SPA contract.

### F2 [P0] — Cross-tenant isolation is unenforced on `/api/v1/audit` and `/api/v1/audit/{id}` in dev-bypass

Two curl probes against the live gateway:

```bash
$ curl -H "X-Tenant-Id: tenant_default" /api/v1/audit?range=7d | jq '.items | length'
10
$ curl -H "X-Tenant-Id: tenant_other_evil" /api/v1/audit?range=7d | jq '.items | length'
10  # IDENTICAL rows, IDENTICAL first_id
```

The detail endpoint behaves the same way:

```bash
$ curl -H "X-Tenant-Id: tenant_other_evil" /api/v1/audit/019e61a927e0…
# returns full detail (provenance + decision chain) regardless of tenant
```

This is a Claim-6 LIVE criterion failure. Per memory `project_v1_strategic_decisions_2026_05_14` and the MISSION.md operator-locked LIVE criteria, claim 6 must show real tenant isolation. The evaluator round `claim_6_live_cross_tenant` regression test (per CLAUDE.md status block) reportedly passes in cargo; that passes against an isolated harness fixture. The dev-bypass gateway running on the operator's machine — the one the demo recording will hit — does NOT enforce the gate.

**Per function-vs-dressing rule**: tenant isolation is the THIRD claim of "claim 6: multi-channel + multi-tenant." If a buyer asks during demo "can I see another tenant's data?" the demo answer must be NO. Today the answer in this gateway state is YES.

**Suggested fix** (do not implement): wire the existing `X-Tenant-Id` header into the SQL WHERE clause on both `/audit` list and `/audit/{id}` detail. The audit-replay endpoint already extracts tenant_id in `emit_replay_audit()` (`routes_audit_replay.rs:127-137`), so the plumbing is one route over. Alternatively (more honest for v1.0 POC demo): leave the dev-bypass single-tenant but add a UI-side guard in the demo MP4 script that the operator should never demonstrate the cross-tenant negative case live — only the positive happy-path.

---

## All findings, in order

### F3 [P1] — `/approvals` lands on the empty state even in demo mode because gateway returns 200 with `{items:[]}`

Tested live: navigated to `/approvals` in the preview SPA with demo-mode banner visible. The page rendered `<ApprovalsEmptyState>` ("No approvals waiting"). The locked E-ops-dashboard (KPI strip + 3 severity tiers + $ at risk + confidence donuts + 7-day approval rate ring) was never reached.

Root cause: `web/lib/api/approvals.ts:189-198` only falls back to `SAMPLE_APPROVALS` on `404` or `TypeError`. The dev gateway returns `200 OK` with `{"items":[]}`, so the demo-mode `SAMPLE_APPROVALS` path is dead. Same shape as approvals: `web/lib/api/audit.ts:307-318` is correctly seeded but never triggers when gateway is healthy-but-empty.

This was prior F3; status is **NOT FIXED** despite the prior audit's explicit recommendation. The locked design exists, has zero coverage in the demo. The Scene-6 brief's promise "/approvals — E ops-dashboard per memory `project_approvals_page_design_locked`" is unrecordable against the current backend without operator-side seeding.

**Reproduction**:
1. Open `/approvals` with demo-mode on; gateway healthy
2. The page renders `<ApprovalsEmptyState>` not `<ApprovalsOpsDashboard>`

**Suggested fix** (do not implement): treat `{items:[]}` from the gateway as "empty" only for real signed-in users; in demo mode, prefer `SAMPLE_APPROVALS`. Two-line change in `fetchApprovals()` — check `isDemoMode() && rows.length === 0` and substitute.

### F4 [P1] — Sidebar "Approvals" goes to `/`, recorded demo URL `/approvals` is a second surface

`web/components/skycot/nav-items.tsx:23` routes the sidebar "Approvals" item to `/`. The canonical approvals queue lives at `/` (rich `app/(app)/page.tsx`, 285+ LOC with BundleCard / TrustRatchetSection / FilterChipRail). The standalone `/approvals` page is the bookmark/deep-link landing that wraps `<ApprovalsOpsDashboard>` for the locked design. The demo MP4 brief says "Scene 6 — `/approvals` then `/audit`" — but a buyer who clicks the sidebar Approvals row lands on `/`, not `/approvals`.

If the operator records typing `/approvals` in the URL bar, F3 still bites. If they click the sidebar, they get the rich queue but no E-ops-dashboard.

**Per function-vs-dressing**: this is a NAVIGATION ambiguity, not theater. Pick one. Either retire `/approvals` (point the deep-link to `/`) or retire `/`'s approvals view and point everyone to `/approvals`.

### F5 [P1] — Hash-chain "prev → this" hover affordance still does not exist

The Scene 6 brief calls hash-chain hover Scene 6's centerpiece. The current `HashCell` in `web/components/skycot/ActivityRow.tsx:166-197` only shows the current row's hash on hover + click-to-copy. There is no `prevHash` field on `AuditRow` (`web/lib/types/audit.ts:117-138`), no chain edge drawing, nothing.

Worse, the seeded hash in `web/lib/sample-audit.ts:149-170` is a per-row FNV-1a digest of `worker.id|verb|timestamp`. Rows do not depend on previous rows. The function's own comment (line 161) says: *"matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade. … Not a cryptographic hash; the real chain hash arrives from the gateway in production."*

This was prior F7; status is **NOT FIXED**. If the camera narrates "every row is hash-chained to the previous" while showing the hash column → function-side theater → P0. If the narration says "every row has a verifiable hash" (the literal current truth — per-row, not a chain) → P1.

**Suggested fix** (do not implement): the gateway already computes `hash_chain_anchor` per row (visible in the detail endpoint response). The SPA needs to (a) expose it on the audit list endpoint, (b) thread it through `AuditRow.prevHash`, and (c) render a small "← 0xabc1…" linkage in `HashCell`. ~3 hours of work + an additive gateway response field.

### F6 [P1] — Live audit rows have empty `worker.name` ("unknown") and identical `verb` ("http.request") because the SPA self-audits every page-view

The gateway audit-list and detail are now wire-real (great), but in dev-bypass the only rows produced are the SPA's own self-audit entries: `audit.detail.viewed` and `audit.feed.queried`, all with `worker.name: "unknown"`, `verb: "http.request"`. The 8 rows curl returns are all SPA-internal traffic.

A buyer recording the demo wants to see Mariah / Peta / Priya / Jamal / Nori actions. Live gateway doesn't have any. Demo mode never kicks in (F3 above) → audit feed shows 8 rows of unidentified "http.request" with no worker context, no costs, no model info.

**Reproduction**: curl `GET /api/v1/audit?range=7d` against the live gateway. Every row has `worker: {id: "", name: "unknown"}, verb: "http.request"`.

**Suggested fix** (do not implement): the `argo demo seed` CLI needs to actually populate seeded audit rows for the demo personas. Or short-circuit the demo-mode audit fetch to ignore gateway response entirely (don't merge — substitute).

### F7 [P1] — Gateway exposes `hash_chain_anchor` per row in the detail response; the SPA never reads or displays it

The detail endpoint returns `provenance.hash_chain_anchor` as a 64-char hex string. The SPA `ProvenanceBlock` (`web/components/skycot/audit/ProvenanceBlock.tsx:70-78`) renders 5 rows: Model, Prompt template, Skill version, Memory snapshot, Policy version. No "Hash chain anchor" row.

This is the load-bearing trust signal — the field name itself says "this anchors the row to the chain." It is computed on the server, returned, and dropped on the floor. For a Scene-6 demo that needs to demonstrate a real audit chain, this is the existing data the UI should already be showing.

**Suggested fix** (do not implement): add a sixth `ProvenanceRow` for "Chain anchor" with `id: detail.id, full: detail.provenance.hashChainAnchor`. Also re-label the section to "Provenance + Chain" so a buyer sees the cryptographic anchor without scrolling.

### F8 [P2] — Decision Chain Card and Provenance Block render hollow rows ("Memory loaded · ", "Skill chosen · ") because gateway returns empty body/details

Live gateway response on the detail endpoint:

```json
"decision_chain": [
  {"body":"","details":{"snapshot_id":null},"n":1,"title":"Memory loaded"},
  {"body":"","details":{"skill_id":null,"skill_trust":null,"skill_version":null},"n":2,"title":"Skill chosen"},
  …
]
"provenance": {
  "model": {"id":"", "version":""},
  "skill": {"id":"", "version":""},
  …
}
```

8 chain steps, all empty body. ProvenanceBlock will render the rows but every cell is empty. Per the locked B3 design (memory `project_audit_page_design_locked`), the decision-chain visual is supposed to read "Memory loaded → snapshot abc · Skill chosen → review-skill v3 · …" — the demo experience is a hollow 8-step skeleton when looking at live gateway rows.

This compounds with F6: the gateway returns shape-correct rows but with no substance. The drill-in destination is "real" (claim 6 negative-test passes), but is theatrically empty.

### F9 [P2] — Empty-state CTA "Open the inbox" loops back to `/` (the canonical approval queue), where the user could already get from the sidebar

`/approvals` empty state's primary CTA (`web/app/(app)/approvals/page.tsx:67`) routes to `/`. The sidebar's Approvals row also routes to `/`. So the empty state CTA tells the user to do exactly what the sidebar already enables. If a buyer is showing `/approvals` to demonstrate the locked E-ops-dashboard design and sees the empty state, the only escape hatch sends them to the rich approval queue (which has its own KPI strip and is brand-loud, not the locked E-ops-dashboard).

**Suggested fix** (do not implement): change the empty-state CTA to "Seed demo approvals" (dev-mode only) or "See your workers" (`/workers`) as the only action — drop the redundant "Open the inbox" → `/` loop.

### F10 [P2] — "Active reviewers" KPI is now `"—"` with sub-label "reviewer roster pending" (improved from prior F11 hardcoded `"1"`)

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:144-150`: the KPI now correctly shows `"—"` and sub-label "reviewer roster pending" instead of the prior bogus `"1"`. This is honest. Per function-vs-dressing, this is dressing-side (camera won't drill into the cell), and the new copy correctly tells the buyer "this isn't wired yet" without lying.

Flagging as a P2 PROGRESS NOTE rather than a blocker — the team did the right thing here.

### F11 [P2] — `/approvals` ops-dashboard "$ at risk" donut has no visual fill at all (improved from prior F5 hardcoded 82%)

`ApprovalsOpsDashboard.tsx:193-199` now renders the `<MetricDonutCard>` without passing the `percent` prop, which means `pct == null` → `fill: var(--muted)` → the donut is a flat muted ring with no completion sweep. Honest improvement over the prior 82% hardcode. Flagging because the *visual affordance* of a donut still implies a measurable percentage; the value the donut should encode (relative to what cap?) is still undefined. Either remove the donut shape entirely or anchor it to a real ratio.

---

## Function-vs-dressing classification (for the demo recording decision)

| Element | Function or dressing? | POC blocker? | Notes |
|---|---|---|---|
| F1 replay → red leak banner | FUNCTION | YES P0 | The replay button is the trust surface; this is what the camera will drill into |
| F2 tenant isolation negative | FUNCTION | YES P0 | Operator must NOT demonstrate cross-tenant during demo |
| F3 `/approvals` empty state | FUNCTION | Demo P0 | Without seeding the locked dashboard never renders |
| F4 `/` vs `/approvals` ambiguity | DRESSING/NAVIGATION | NO | Pick one for the script |
| F5 hash-chain "prev → this" | FUNCTION (if narrated) / DRESSING (if not) | Conditional | Depends on script narration |
| F6 unknown workers / http.request verbs | FUNCTION | Demo P1 | Avoidable with seed data |
| F7 `hash_chain_anchor` dropped | FUNCTION | NO (not currently narrated) | Cheap to fix |
| F8 hollow decision-chain | FUNCTION | Demo P1 | Hollow with no seed; rich with seed |
| F9 empty-state loop | DRESSING | NO | Polish |
| F10/F11 dashboard cells | DRESSING | NO | Now honest |

---

## Resolved since last audit

### F67 (replay fabrication path) — **FIXED** in `web/lib/api/replay.ts`

The previous audit's headline P0 — `/audit/action/{id}/replay` fabricating "Policy regression detected" content from URL-derived hashes — has been properly fixed:

1. Mock fallback is now gated on `isDemoMode() || isAuthBypassed()` AND only returns from the canonical TABLE (line 78-81 `sampleOrFailed()`)
2. Unknown IDs return `UNKNOWN_ACTION_FAILED` (line 66-70): "Action not found — the replay sandbox only covers the seeded demo personas." The page now renders `<FailedView>` with a real explanation rather than fabricating content

The team did the right thing. New F1 above is a separate bug in the live-gateway path (the mock path is fine; the live response shape disagrees with the contract).

### Prior F2 (list/detail ID disagreement) — **FIXED**

I curled real list rows and successfully resolved every ID against the detail endpoint. The list-vs-detail disagreement reported in the 2026-05-25 audit is gone; list and detail now persist into the same backing table (or fetch from the same view).

### Prior F4 (7-day rate hardcoded 0.78) — **FIXED**

`ApprovalsOpsDashboard.tsx:90-101 computeApprovalRate()` derives `approved`, `refused`, `expired` from the real decided rows. Returns `null` when nothing is decided; KPI subline correctly says "no resolved approvals". Honest.

### Prior F5 ($ at risk donut hardcoded 82%) — **FIXED**

No more `percent={totalAtRisk > 0 ? 0.82 : 0}`. The donut renders muted (no fill) so the visual no longer lies. See F11 above for the residual concern that the donut *shape* still implies a percentage exists.

### Prior F6 (approval rows unclickable) — **FIXED**

`ApprovalsOpsDashboard.tsx:299-313` `ApprovalRow` is now a `<button>` with an `onClick` that calls `window.location.assign(actionHref)`. Action IDs are correctly resolved via `SAMPLE_APPROVAL_ACTION_IDS` map. The audit lineage IS now reachable from an approval row (was the prior gap).

### Prior F10 (Approval-rate fallback 0.78 with "7-day baseline" sub-label) — **FIXED**

Now derived from real data; falls back to `"—"` + "no resolved approvals" rather than a fake 78%.

---

## Orientation score: 67 / 100

The team did real work since the 2026-05-25 audit. F67 / F2 / F4 / F5 / F6 / F10 are all closed. The replay client gates demo fixtures correctly, the audit drill-in route resolves real IDs, the approvals dashboard derives KPIs from data and is honest about gaps. That's a significant uplift.

But two new function-side P0s have surfaced and were not on the prior radar:

- **F1 replay-response-shape mismatch** is the new "must fix before pressing record." Every successful replay against the live gateway renders the catastrophic "side-effect leak" red banner. That is camera-side worse than F67 (a fabricated "policy regression" was at least the right *category* of trust signal; a fabricated "we leaked side effects" is a credibility-ender).
- **F2 cross-tenant isolation absent at gateway level** in dev-bypass means the recording operator must NOT demonstrate the negative tenant-isolation case. Per the prior audit, the evaluator round `claim_6_live_cross_tenant` passes against a harness; the *demo gateway* doesn't honor `X-Tenant-Id`.

And one carryover P1: the hash-chain "prev → this" affordance the brief calls Scene-6's centerpiece (F5, prior F7) still doesn't exist; the gateway exposes a `hash_chain_anchor` field that the SPA simply doesn't read (F7).

**Recommended pre-record sequence**:

1. Fix F1 (~30 LOC in `web/lib/api/replay.ts` to adapt the gateway shape to the client envelope)
2. Decide what to demonstrate for F2 (operator-side directive: positive happy-path only OR push the SQL-side tenant gate forward)
3. Seed demo audit/approval rows (or fix F3 to fall back to fixtures even when the gateway returns 200 with `{items:[]}`)
4. Optionally add the `hash_chain_anchor` row to ProvenanceBlock (F7 — a 5-line change that visibly increases the cryptographic-claim density)

If those four ship, the score jumps to ~88; remaining items are polish.
