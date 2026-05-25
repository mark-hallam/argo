# Scene 6 + approvals — Claim 6 (multi-channel + multi-tenant trust) UX audit

Run date: 2026-05-26 08:20 AEST.
Scope: `/approvals`, `/audit`, audit row drill-in, hash chain verification, cross-tenant isolation, `/audit/action/{id}/replay`.
Method: READ-ONLY against the running dev SPA (localhost:3000) and gateway (127.0.0.1:7400). Source + curl only. No browser MCP.
Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — Scene 6 is camera-visible and the drill-in IS narrated, so theater in the audit/replay flow is P0.

## TL;DR

**11 findings: 4 P0, 4 P1, 3 P2.** The headline blocker is a **schema mismatch between the gateway's `POST /api/v1/audit/<id>/replay` response and the SPA's `ReplayOutcome` discriminated union** — they are not the same contract, so the live demo path for the most camera-narrated drill-in (Replay) cannot render anything but a `failed` state against real rows. Compounding that: **the live workspace has zero seeded demo audit rows** (just two self-audit `http.request` rows from this audit's own gateway probes), so Scene 6 has literally nothing to point a camera at on `/audit` and the locked E-ops-dashboard never renders on `/approvals` (it short-circuits to the empty-state CTA when `pendingCount === 0`). The dev `.env.local` ships the **wrong env-var name** for the client-side mock fallback (`ARGO_GATEWAY_AUTH_BYPASS=1` vs the required `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`), so the SPA can't even fall back to seed data in this dev session. Cross-tenant negative test surfaced a real concern: the cross-tenant override path returns rows **without enforcing capability** in this dev/auth-bypass environment.

## Headline blockers (P0)

| # | Finding | Scene impact |
|---|---|---|
| F1 | Replay endpoint schema mismatch between gateway and SPA — replay UI can render nothing real | Scene 6 drill-in → "Replay" button is unrecordable as a success case |
| F2 | Live workspace has 0 seeded audit rows; `/audit` shows "Quiet day" empty state | Scene 6 audit feed has no rows to demo |
| F3 | `/approvals` falls back to CTA empty state because gateway returns `{items:[]}` — the locked E-ops-dashboard never renders | Scene 6 approvals screen is not the locked design |
| F4 | `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` is missing from `.env.local`; the file sets the non-public alias, which the client code does not read | All client-side mock fallbacks for audit/approvals/replay are silently disabled |

---

## All findings

### F1 [P0] — Replay endpoint contract mismatch: gateway returns hash-recompute verdict, SPA expects sandboxed-rerun verdict

The gateway's `POST /api/v1/audit/<id>/replay` returns a body shaped like:

```
{"audit_id":"...","seq":1840,"kind":"http.request","verdict":"same",
 "expected_self_hash":"...","actual_self_hash":"...","diff":{},...}
```

The SPA's `ReplayOutcome` discriminated union expects:

```
{ kind: "success"|"unavailable"|"failed"|"leak-detected",
  verdict: "reproducible"|"drifted-aligned"|"drifted-misaligned",
  result: { provenanceDiff: { model, promptTemplate, skill, memorySnapshot, policy }, ... } }
```

These are not the same concept. The gateway is recomputing the chain hash and reporting whether it equals the stored value (good — that's a real integrity check). The SPA is rendering a "rerun the action in a sandbox and diff outputs + provenance" view. The two never overlap.

When the SPA POSTs against the live gateway and gets the hash-recompute body, the discriminator (`outcome.kind`) is `undefined`. `OutcomeView` walks the if-chain and falls through to `<LeakView>` (the last `return`), which would render the destructive "Replay aborted — side-effect leak detected" banner with `details: undefined`. The camera narration in Scene 6 calls out replay as proof of "deterministic re-execution"; what the demo would actually render is a red leak-detected alert or a render crash, neither of which is the locked design.

Confirmed live:
- `curl -s -X POST "http://127.0.0.1:7400/api/v1/audit/<real_id>/replay"` returns `{"verdict":"same","expected_self_hash":"...","actual_self_hash":"...","kind":"http.request",...}` (no `kind:"success"` discriminator).

Reproduction:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=lq&as_tenant=ws-default" -H "X-Argo-Audit-Cross-Tenant: 1" -H "x-argo-tenant-id: ws-default"` to grab a real id
- `curl -s -X POST "http://127.0.0.1:7400/api/v1/audit/<id>/replay"` — see the verdict shape
- Compare to `web/lib/api/replay.ts:27-46` discriminated union

File path: gateway `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs:81-94`, SPA `web/lib/api/replay.ts:27-46` + `web/app/(app)/audit/action/[actionId]/replay/page.tsx:132-159`.

Suggested fix: either (a) the gateway returns a `kind:"success"|...` envelope per the SPA contract (preferred — gateway is the contract-laggard here per the file comment), or (b) the SPA adds a thin adapter that maps the gateway's hash-recompute shape into the `ReplayOutcome` union (e.g. `verdict:"same"` → `kind:"success", verdict:"reproducible", result:{provenanceDiff:<empty>}`). Option (a) needs server work; option (b) is web-only and unblocks the demo path immediately.

### F2 [P0] — Live audit store has 0 seeded demo rows

The live store has only two audit rows total, both self-audit `http.request` entries from gateway query handling. `GET /api/v1/audit?range=lq` with cross-tenant override returns `count=2, channels={'web'}, action_types=['audit.feed.queried'], workers={'unknown'}`. There is no Mariah / Priya / Sam / Jamal / Nori activity in the chain at all.

Scene 6 of the runbook narrates "≥10 rows in Etherscan-style table"; the camera would see "Quiet day" pristine empty state. Even the fallback-mock path is disabled in this dev env (see F4) — but even if it were enabled, the mock is not a real audit; it's static JSON in `web/lib/sample-audit.ts:181`.

Reproduction:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=lq&as_tenant=ws-default" -H "X-Argo-Audit-Cross-Tenant: 1" -H "x-argo-tenant-id: ws-default"` — returns 2 rows, both `audit.feed.queried`

File path: missing seed run; runbook `docs/skycot/demos/v1-mission-mvp-runbook.md` likely depends on `argo demo seed --tenant <demo>` or a `mission-demo-seed-data-bootstrap` brief. Per memory `feedback_no_theater_in_demo`, the operator rewrote that brief to "Option B (real backend seed CLI)" — that CLI does not appear to have been run against this workspace.

Suggested fix: run the real seed CLI to populate the audit chain with multi-channel agent turns BEFORE recording. The seed must produce rows on at least the 4 channels the runbook narrates (web, slack, sms, voice/discord) so Claim 6 "multi-channel" is observable in the feed.

### F3 [P0] — `/approvals` collapses to CTA empty state instead of rendering the locked E-ops-dashboard

`web/app/(app)/approvals/page.tsx:25-48` short-circuits to `<ApprovalsEmptyState />` whenever `pendingCount === 0`. With the live gateway returning `{items:[]}`, the locked design (E-ops-dashboard with 5-KPI strip, 3 severity tiers, $-at-risk donut, 7-day approval-rate ring) **never renders**. The camera in Scene 6 would see "No approvals waiting · Open the inbox" — a generic empty state, not the locked enterprise ops surface.

Compounding: the side-nav's "Approvals" link points to `/` (the canonical inbox at `web/components/skycot/nav-items.tsx:23`), NOT `/approvals`. So a viewer who clicks the nav "Approvals" item lands on the *root* page (a totally different UI built around BatchGroupBlock + filter rails), not the locked dashboard. `/approvals` exists as a deep-link landing only, and only when there are pending items.

Reproduction:
- `curl -s "http://127.0.0.1:7400/api/v1/approvals"` → `{"items":[]}`
- Visit `/approvals` in browser → "No approvals waiting" empty state instead of E-ops-dashboard
- Click "Approvals" in left nav → goes to `/` not `/approvals`

File path: `web/app/(app)/approvals/page.tsx:25-48`, `web/components/skycot/nav-items.tsx:23`.

Suggested fix: (a) seed live approvals so `pendingCount > 0` and the dashboard renders, or (b) point the nav "Approvals" link at `/approvals` for the demo recording so the locked design is what the camera sees. Both are required for the locked-design narration to land.

### F4 [P0] — `.env.local` sets `ARGO_GATEWAY_AUTH_BYPASS=1` (server-only); client code reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`

`web/.env.local:6` has `ARGO_GATEWAY_AUTH_BYPASS=1`. The mock-fallback branches in `web/lib/api/audit.ts:57-62`, `web/lib/api/audit-detail.ts:35-40`, `web/lib/api/replay.ts:54-59`, and `web/lib/api/approvals.ts:74-76` all read `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. Without the `NEXT_PUBLIC_` prefix the variable is not inlined into the client bundle. Result: every mock fallback is silently inert. The dev SPA does what production does — hits the (empty) live gateway and shows empty states.

This is at the same time good (no theater leaking into dev) and bad (the runbook script and design memos all assume seeded fixtures are visible during dev). It also means the F3 empty-state condition cannot be worked around by flipping a flag.

Reproduction:
- `grep ARGO_GATEWAY_AUTH_BYPASS web/.env.local` → `ARGO_GATEWAY_AUTH_BYPASS=1` only
- `grep ARGO_GATEWAY_AUTH_BYPASS web/.env.example` → both names visible; `.example` says `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0`
- Source: `web/lib/api/audit.ts:57-62`

File path: `web/.env.local:6`.

Suggested fix: either rename the env-var in `.env.local` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (one-line edit) OR remove the bypass entirely and seed the workspace with real data. The second is preferred for the POC demo (real > mocked, per `feedback_no_theater_in_demo`).

### F5 [P1] — Audit-row hash is a non-cryptographic FNV-1a pretender; gateway feed JSON doesn't carry hash at all

`web/lib/sample-audit.ts:162-179` generates row.hash via 8 FNV-1a passes (`rowHash`). The function comment is candid: "Not a cryptographic hash; the real chain hash arrives from the gateway in production." That would be fine — except `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` does NOT include `hash` in its JSON envelope. Verified: live `GET /api/v1/audit` rows have no `hash` field. Only `GET /api/v1/audit/<id>` (detail) and `POST .../replay` (recompute verdict) expose hash data.

So the camera-visible "Etherscan-style 0xabc1…2345 click-to-copy" cell at `web/components/skycot/ActivityRow.tsx:166-197` will only render hashes if (a) the row came from the SPA's sample-audit fallback (FNV digests) or (b) someone backfills `hash` into the feed envelope. With live data, the column is blank.

Note: this is NOT a hash-chain integrity bug — the real SHA-256 chain is intact server-side (`crates/argo-audit/src/lib.rs:10` documents `self_hash = sha256(parent_hash || kind || 0x00 || canonical_payload)`). The bug is that the feed JSON projection drops the hash, AND the demo fallback synthesises a hex string that looks SHA-256-shaped but isn't.

Reproduction:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?..."` → no `hash` field on items
- `web/lib/sample-audit.ts:162-179` FNV `rowHash` function

Suggested fix: add `self_hash` (hex-encoded) to the feed JSON envelope server-side. Drop the FNV `rowHash` and either render no hash on the seed rows or seed real audit rows whose stored `self_hash` flows through naturally.

### F6 [P1] — `/audit/action/<arbitrary-id>` synthesises a fake decision chain + provenance from any feed row

`web/lib/sample-audit-detail.ts:244-265` (`synthesizeAuditDetail`) fabricates an 8-step decision chain, model `claude-opus-4-7`, skill `generic-skill v1.0.0`, policy `generic-policy v1.0`, and an output body `"Synthetic output reconstructed from feed row <id>. Production wires this to the gateway detail endpoint."` for any audit id that matches a fallback-feed row but has no hand-crafted detail.

This is theater the camera DOES see — if the demo clicks any row that isn't one of the 5 hand-crafted personas (Priya/Sam/Mariah/Jamal/Nori), the drill-in renders a fabricated chain that reads as forensic but is invented at render time. The output body is literally the string above. Operator memory `feedback_no_theater_in_demo` 2026-05-24: "function-side theater is forbidden."

In the current dev env F4 disables this code path, but the moment seed mode is enabled or the env-var is fixed, every non-canonical click into a row generates fake provenance.

Reproduction:
- Source: `web/lib/sample-audit-detail.ts:244-265`
- `synthesizeAuditDetail()` returns a fake `provenance.model`, `provenance.skill`, `provenance.policy`, and a templated `output.body` string

Suggested fix: remove the `synthesizeAuditDetail` fallback entirely. If a feed row doesn't have a real detail in the store, the drill-in should hit `AuditActionNotFound`. Stub provenance is worse than 404 for an investor demo.

### F7 [P1] — Cross-tenant audit override returns rows without capability enforcement (auth-bypass env only)

`GET /api/v1/audit?as_tenant=<other>` with header `X-Argo-Audit-Cross-Tenant: 1` is supposed to require `Action::AuditCrossTenant` capability (per `routes_audit_feed_scope.rs:28-34` comment and the `cap_granted` variable). In this dev gateway (auth-bypass mode), the cross-tenant override succeeds and returns rows from the target tenant — the cap check appears to evaluate `true` for the bypass principal `system:dev-bypass`.

Reproduction:
- `curl -s -H "x-argo-tenant-id: tenant_a" -H "X-Argo-Audit-Cross-Tenant: 1" "http://127.0.0.1:7400/api/v1/audit?as_tenant=tenant_b"` → returns tenant_b rows (HTTP 200)
- Same call without the header → empty array (no leak)
- Without cap, the spec says cross-tenant should be denied or fall back to the workspace tenant

This is the auth-bypass dev mode, not production, so the live consequence is limited to local dev. But the capability gate either auto-grants in bypass mode (acceptable IF documented) or the cap-check is genuinely missing — needs operator decision. Bonus risk: if any preview build also flips bypass on, it would behave the same.

File path: `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-43`, `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:441-445`.

Suggested fix: confirm that auth-bypass mode grants `AuditCrossTenant` deliberately; if so, add a banner in the SPA when auth-bypass is on so operators don't mistake bypass behaviour for production behaviour. If accidental, add a deny rule in bypass mode.

### F8 [P1] — Approvals row click target-IDs come from a hand-rolled map, not the approval payload

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:25-34` defines `SAMPLE_APPROVAL_ACTION_IDS` — a hardcoded `Record<approvalId, actionId>` that lights up drill-in only for the 8 sample approvals (`ap_01H9Z0` through `ap_01H9Z7`). The fallback `?? item.id` means non-sample approvals use the approval id as the action id, which won't match any audit row, so `/audit/action/<id>` will land on `AuditActionNotFound`.

For the POC demo this is fine IF the only approvals shown are the 8 seeded ones. The moment a real approval arrives via gateway, the click-through breaks unless the server payload includes an `action_id` field (which `rowToItem` at `web/lib/api/approvals.ts:153-177` does support via `row.action_id`). So the contract is half-wired: real payloads must carry `action_id`, and the hardcoded fallback masks any time the server omits it.

File path: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:25-34, 296`.

Suggested fix: drop the hardcoded map; rely on `item.actionId ?? item.id` only. If a click-through breaks, log it so the missing-actionId server-side bug is visible instead of papered over.

### F9 [P2] — KPI strip "Approval rate" is computed from in-memory items, not the 7-day rollup the design promises

`ApprovalRateRing.tsx:7-9` documents the ring as sourcing from `/api/v1/approvals/stats` (rolling 7-day). The dashboard at `ApprovalsOpsDashboard.tsx:90-101` (`computeApprovalRate`) computes the rate from the **currently-loaded items only** (items where `decision != null`). With an empty list it shows "—"; with sample data it'd show the static fixture rate, not the 7-day truth.

`/api/v1/approvals/stats` endpoint may not be wired yet. Not camera-blocking — the ring renders sensibly either way — but the locked design promises a 7-day rollup and the implementation doesn't deliver it.

File path: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:90-101, 213-241`.

Suggested fix: wire the dashboard to `/api/v1/approvals/stats` when it ships. Until then, accept the in-memory approximation but update the deck text to match ("Resolution rate on shown approvals" not "7-day approval rate").

### F10 [P2] — "Active reviewers" KPI cell is hardcoded to "—" with "reviewer roster pending" deck

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-150` permanently emits value `—` for the 5th KPI cell. Not theater (it's transparent about being unwired), but the locked design's 5-KPI strip looks incomplete with one cell always dashed. Camera would see "Active reviewers · —".

Suggested fix: either ship the reviewer-roster wire-up or remove the cell so the strip is 4 lively cells rather than 4 + 1 dashed.

### F11 [P3] — Hash-cell tooltip says "Click to copy" but the value copied is the FNV pretender on seed rows

If F5 is fixed and feed JSON carries real `self_hash`, this is automatic. If not, operators clicking the chain-hash on `/audit` rows during dev see the FNV digest copied to clipboard — a "real-looking" hash that wouldn't match anything in the audit chain on the server. Edge case; not load-bearing for the demo.

File path: `web/components/skycot/ActivityRow.tsx:177-186`.

---

## Resolved since last audit

The 2026-05-25 11:00 prior pass (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`) is from yesterday on a stale workspace. Diffing the findings is out of scope for this run, but the **gateway audit detail synthesised forensic-report problem** (the F67-style "fabricate from URL") is still wired in `synthesizeAuditDetail` — re-surfaced as F6.

## Orientation score

**52 / 100** for the POC + investor demo path.

Reasoning: the UI components themselves are well-built and the locked designs are clearly implemented in source (KpiStrip, ApprovalRateRing, ConfidenceDonut, DollarAtRisk all exist and render). The brand-loud hero on `/approvals` and the Etherscan-style hash cell on `/audit` are visually present. What's broken is the **end-to-end demo path**: the live gateway has no seeded data, the env-var name is wrong so the mock fallback is dead, the locked E-ops-dashboard collapses to an empty-state, and the most camera-narrated drill-in (Replay) talks to a gateway endpoint whose response shape the SPA cannot decode.

A 3-hour focused session can land all 4 P0s: (1) fix the env-var rename in `.env.local`, (2) seed real audit rows via the demo CLI, (3) wire the Replay endpoint contract on either side, and (4) point the nav "Approvals" item at `/approvals` for the recording. After that the score jumps to ~80.
