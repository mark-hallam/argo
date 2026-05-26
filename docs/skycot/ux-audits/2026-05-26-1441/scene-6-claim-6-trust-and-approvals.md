# Scene 6 + Approvals — Claim 6 trust-and-approvals UX audit

**Date**: 2026-05-26 14:41 AEST
**Scope**: `/approvals` (E-ops-dashboard) + `/audit` (B3-Etherscan) + hash-chain hover + audit-row drill-in (REAL vs fabricated) + cross-tenant isolation + `/audit/action/{id}/replay` fabrication check.
**Method**: read-only — SPA HTTP probes + gateway HTTP probes + source review. (Preview MCP browser tab hung mid-session after a `window.location.assign('/')` call; subsequent visual checks deferred to direct HTTP. Code-level evidence is authoritative for function-side fabrication classification.)
**Positioning rule applied**: function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`. Demo is POC/investor, not public release. Theatre the camera DOESN'T see ≠ blocker. Theatre the camera DOES see = blocker.
**Run config observed**: SPA on `:3000` and previewer on `:62927`, both with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_ARGO_DEMO_MODE=1`. Gateway alive on `:7400` (schema FRESH per CLAUDE.md status block).

---

## TL;DR

12 findings. 3 P0 demo-blockers (Scene 6 hash-chain beat unrecordable both ways; SPA audit-detail adapter silently throws against the real gateway shape so the live path is fully broken; the approval-row click destination on the locked ops-dashboard hard-codes a `window.location.assign` instead of router.push and loses any in-flight state). 4 P1s (synthetic hashes + persona ids look forensic but aren't; client never sends a tenant header; approvals API tenant binding is cookie-only with no SPA-side guard; KPI "Active reviewers" is always `—`). 5 P2/P3 polish nits.

**Biggest blocker for the camera**: the Scene 6 hash-chain hover beat — the runbook tells the operator "click a row, watch the hash propagate up the chain"; under live gateway there's no hash to hover, under auth-bypass the hash is a deterministic FNV-1a digest, not the real chain. The investor sees Etherscan-style chrome on top of theatre.

---

## Headline blockers

1. **F1 [P0] — Hash-chain hover is theatre under auth-bypass AND missing under live gateway** — function-side, camera-visible during Scene 6.
2. **F2 [P0] — SPA `fetchAuditDetail` cannot deserialize the real gateway response shape** — audit-detail drill-in works only because auth-bypass short-circuits to mocks. Live drill-in is functionally broken.
3. **F3 [P0] — `ApprovalsOpsDashboard` row click uses `window.location.assign` rather than the SPA router** — full page reload mid-Scene-6, loses TanStack cache, and the destination url path is composed from a fragile id-mapping table that has zero overlap with the gateway's real action ids.

---

## All findings, in order

### F1 [P0] — Etherscan hash-chain hover is fabrication when auth-bypass is on, and absent when it's off

The audit-row Etherscan-style truncated hash + hover is the visual centerpiece Scene 6 narrates. Two failure paths, both function-side:

- **Under auth-bypass** (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`, current dev config): `SAMPLE_AUDIT_ROWS` synthesises `hash` via 8 FNV-1a passes seeded by `worker.id|verb|timestamp` (`web/lib/sample-audit.ts:162-179`). The code's own comment is honest: *"Not a cryptographic hash; the real chain hash arrives from the gateway in production"*. The investor sees `0xabc1…2345` in monospace + a "Click to copy" tooltip; what they're copying is fabricated. The drill-in destination on this id then synthesises a fake provenance with `policy.hash: "policepolicepolicy"` (literally that string — `web/lib/sample-audit-detail.ts:257`). That string is camera-visible if anyone clicks the provenance copy button or reads the row.
- **Under live gateway** (auth-bypass off): the gateway audit-feed response has **no `hash` field at all** (verified `GET /api/v1/audit?range=7d` against `:7400` returns rows with keys `['action_type','approver','channel','cost','data_classification','id','status','timestamp','verb','worker']` — no `hash`). The SPA's `ActivityRow` (`web/components/skycot/ActivityRow.tsx:82-86`) only renders `HashCell` when `activity.hash` is present, so the Etherscan column collapses. Scene 6 "watch the hash chain" beat unrecordable.

**Why this matters for the POC demo**: the hash-chain hover is the prop the camera lingers on for Scene 6 ("Claim 6 — trust + multi-channel"). The narration cites the hash as evidence of forensic-grade audit. Either path makes that beat untrue. The drill-in destination synthesises `"policepolicepolicy"` as the policy hash, which is unambiguously fabrication.

**Repro**:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | python3 -m json.tool | head -15` → no `hash` field
- Open `web/lib/sample-audit.ts:162` and read the comment + `rowHash` impl
- Open `web/lib/sample-audit-detail.ts:257` and search for `policepolicepolicy`
- Walk Scene 6 of `docs/skycot/demos/v1-mission-mvp-runbook.md` and note which beat the hash hover anchors

**Files**:
- `~/code/argo/web/lib/sample-audit.ts:162-179` (synthetic hash generation)
- `~/code/argo/web/lib/sample-audit-detail.ts:252-265` (synthesizeAuditDetail's `"policepolicepolicy"`)
- `~/code/argo/web/components/skycot/ActivityRow.tsx:82-86` (HashCell-only-if-hash branch)
- gateway: missing `hash` projection — likely `crates/argo-gateway/...routes_audit*.rs`

**Suggested fix**: gateway adds a `hash` field to the audit-feed row equal to a chain hash; SPA stops synthesising the FNV one under auth-bypass (let the column collapse when no hash, never fabricate one). Then either record Scene 6 with the camera-visible explanation "audit chain hash" pointing at real bytes, OR cut the hash-hover beat.

---

### F2 [P0] — `fetchAuditDetail` cannot deserialize the live gateway envelope, so the live drill-in path is silently broken

`web/lib/api/audit-detail.ts:65-67` calls `apiGet<AuditDetail>(path)` with **no normalisation**. The gateway response wraps the payload in `{ detail: { ... } }` with snake_case fields and an extra `provenance.hash_chain_anchor` + `provenance.signature` that the SPA's `AuditDetail` type doesn't declare. Probe (`GET /api/v1/audit/019e61d28dc77db0a38693ff74d99fe7` → ` :7400`) returns this envelope, which TypeScript-typed-as-AuditDetail succeeds but every consumer reads `.id`/`.verb`/`.provenance.model.id` directly off the top-level object → those are undefined because they're actually under `.detail.summary.*` and `.detail.provenance.*`. Under `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` the path is short-circuited to `mockDetail(actionId)` so dev never hits this. **Switch off auth-bypass and the drill-in renders blanks** — no narration possible.

Compare to the working pattern: `web/lib/api/audit.ts:209-222` (`normalizeAuditFeed`) explicitly unwraps `{ items, next_cursor }`, maps snake→camel, etc. The detail client just doesn't.

**Repro**:
- `curl -s "http://127.0.0.1:7400/api/v1/audit/019e61d28dc77db0a38693ff74d99fe7"` → confirm `{ detail: { summary: { id, verb, ... }, provenance: { hash_chain_anchor, ... }, decision_chain, output, inputs } }`
- Read `web/lib/api/audit-detail.ts:58-77` — `apiGet<AuditDetail>(path)` is the entire body of `fetchAuditDetail`, no adapter call

**Files**:
- `~/code/argo/web/lib/api/audit-detail.ts:58-77`
- `~/code/argo/web/lib/types/audit-detail.ts:84-90` (AuditDetail shape)
- Gateway shape that lives in `~/code/argo/crates/argo-gateway/src/...` (search `hash_chain_anchor`)

**Suggested fix**: add `normalizeAuditDetail(raw: unknown): AuditDetail` mirroring `normalizeAuditFeed`. Unwrap `detail`, map `summary` fields up to the top level, rename `prompt_template`→`promptTemplate`, `data_classification`→`dataClassification`, `decision_chain`→`decisionChain`, `memory_snapshot`→`memorySnapshot`, `follow_up`→`followUp`. Surface `hash_chain_anchor` as part of provenance (currently has no home in the SPA shape).

---

### F3 [P0] — Approval-row click hard-codes `window.location.assign` AND composes drill-in id from a stub table with zero gateway-id overlap

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:293-306`:

```
onClick={() => { window.location.assign(actionHref); }}
```

Two problems for Scene 6:

1. **`window.location.assign` causes a full page reload**, not a Next.js client transition. The SPA's TanStack Query cache, the React tree, the demo banner timer, the AppShell layout — all reset. Subjectively the camera sees a flash of the loading state plus a reflow.
2. The `actionHref` is built from `SAMPLE_APPROVAL_ACTION_IDS` (`ApprovalsOpsDashboard.tsx:25-34`) — a hand-keyed lookup table mapping the 8 sample approval ids (`ap_01H9Z*`) to `act_*` ids. Under live gateway the approvals would arrive with completely different ids and the table miss → fallback is `item.id` (an approval id, not an action id) → the destination `/audit/action/ap_01H9Z7` 404s into `AuditActionNotFound`. (`web/components/skycot/audit/AuditActionNotFound.tsx`.) The function-side promise of "click the approval → see its audit row" silently breaks the moment data is real.

Compare to the canonical pattern in `web/components/skycot/ActivityRow.tsx` which uses `<Link href={href}>`, no window.location, no static lookup table.

**Repro**:
- Click any P0 row in `/approvals` under auth-bypass → URL changes via assign, full reload visible
- Mentally swap to live mode: approval ids would be ULIDs from the gateway, hash table miss, drill-in 404s

**Files**:
- `~/code/argo/web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:25-34, 293-306`

**Suggested fix**: take `actionId` from the gateway approvals JSON (the optional `action_id` already on `ApprovalRowJson` in `web/lib/api/approvals.ts:122`); render the row as a `<Link>` or `router.push`; delete `SAMPLE_APPROVAL_ACTION_IDS`. When gateway omits `action_id`, hide the click target rather than fabricate a destination.

---

### F4 [P1] — Synthetic FNV hash + persona action-ids (`act_01J0PRIYA_001`) look forensic but the real audit-chain in the gateway uses 32-char hex ULID-like ids

The two id schemes are incompatible. Verified: `GET /api/v1/audit/act_01J0PRIYA_001` → 404 `audit row not found`; `POST /api/v1/audit/act_01J0PRIYA_001/replay` → 400 `audit id must be 32-char hex (16 bytes)`. The SPA's mocks never test the real round-trip. If anyone records Scene 6 against the live gateway, they're presented with hex ids the operator's CLI/db scripts know nothing about; the seeded persona narrative breaks. Dressing-side, but the camera does read the id.

**Files**:
- `~/code/argo/web/lib/sample-audit.ts:136` (id format)
- `~/code/argo/web/lib/sample-audit-detail.ts:46, 85, 123, 161, 197` (persona detail ids)

**Suggested fix**: regenerate sample-audit-detail ids as 32-char hex when the seed CLI lands (or align the gateway to accept the human-readable format under demo mode).

---

### F5 [P1] — SPA audit / approvals / replay clients don't send a tenant header; cross-tenant isolation is invisible from the SPA layer

`web/lib/api/client.ts` + `web/lib/api/audit.ts` + `web/lib/api/audit-detail.ts` + `web/lib/api/replay.ts` + `web/lib/api/approvals.ts` — none contain the strings `X-Tenant`, `tenant_id`, or `tenantId`. Tenant binding is left entirely to the cookie-based session resolution at the gateway. That's architecturally fine for prod (gateway resolves tenant off the JWT), but under `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` there is no tenant binding at all. The cross-tenant-isolation negative-test path Scene 6 wants to demonstrate (claim 6 is "multi-tenant") cannot be observed at this layer — you'd have to drive two tenant sessions through different browser profiles.

Not a P0 because the gateway-side regression test (`claim_6_live_cross_tenant`) pins isolation in code and passed cold per CLAUDE.md status block. P1 because the camera/operator have no SPA-visible affordance to show isolation working ("here's tenant A's audit, log in as tenant B, refresh, see only B's rows").

**Files**:
- `~/code/argo/web/lib/api/client.ts`
- All four `web/lib/api/*` modules touching audit/replay/approvals

**Suggested fix**: out of scope for v1.0 demo (no UX surface), but flag for v1.1: a workspace switcher in the AppShell that explicitly demos "no rows leak between workspaces" would make Scene 6's multi-tenant claim recordable.

---

### F6 [P1] — `Active reviewers` KPI permanently `—` on a fully-mocked dashboard

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-150` hard-codes `value: "—", sub: "reviewer roster pending"`. The KPI strip has 5 cells; the 5th always reads as a "shrug emoji" next to the populated four. Camera-visible on Scene 6 if the operator pans the dashboard. Dressing-side, but it makes the dashboard look unfinished.

**Suggested fix**: either populate from `pending.flatMap(p => p.approver?.name)` distinct-count (cheap, honest), or drop the cell to a 4-up KPI strip.

---

### F7 [P1] — Approval-rate ring shows `—` because every seeded approval has `decision: null`

The 8 `SAMPLE_APPROVALS` are all pending, so `computeApprovalRate(items)` returns `null` (`ApprovalsOpsDashboard.tsx:90-101`). The right-hand aside renders the "—" empty ring with body "No resolved approvals yet". This is consistent with the empty-state design but it also means the ring — the page's most decorative element — never animates on camera unless someone clicks approve/refuse during the take. Operator should plan the take to include at least one approve to populate the ring.

**Suggested fix**: seed 2-3 of `SAMPLE_APPROVALS` with `decision: "approve"` so the ring lands populated on first paint, then the live approve still demos the optimistic update.

---

### F8 [P2] — Gateway audit rows from `audit.detail.viewed` and `audit.replay` get rendered with `worker.name: "unknown"` + `verb: "http.request"`

Verified: `curl /api/v1/audit?range=7d` returns 14 rows all of which are the gateway's own self-audit of the previous SPA queries — `verb=http.request, worker={id:"", name:"unknown"}, action_type=audit.detail.viewed|audit.replay`. If anyone toggles auth-bypass off mid-demo, the audit feed shows 14 rows of "unknown · http.request" — meta-circular noise no investor wants to see. This is real data, but it's not narrative.

Dressing-ish (the rows are real, not fabricated), but the camera does see it if it lands here. P2 because the POC demo plans to stay in auth-bypass mode for Scene 6.

**Suggested fix**: gateway filters `action_type` starting with `audit.` from the default feed (or moves them to an "internal" category), OR the seed CLI inserts narrative rows ahead of these.

---

### F9 [P2] — `redirect /approvals` and `/` both route to the same Approvals shell, but the breadcrumb / title / hero copy diverge

`/approvals/page.tsx` has its own `BrandLoudHero` + own ops-dashboard rendering; `/page.tsx` (the canonical inbox at `/`) has a different header pattern (KpiStrip + FilterChipRail + DataSourceChip). Sidebar Approvals nav points at `/`. Anyone typing `/approvals` directly into the URL gets a slightly different page than the sidebar link — same dashboard component, different chrome. Not a recording-blocker but a polish gap.

**Suggested fix**: pick one canonical path. The note in `/approvals/page.tsx:11-23` says the file exists to avoid a `not-found.tsx` for the typed URL — fine, but it should just `redirect('/')` rather than render a divergent shell.

---

### F10 [P2] — `EvidenceBundleExporter` is hard-coded `tenantId="tenant_default"` in both the audit feed and the audit detail page

`web/app/(app)/audit/page.tsx:261-262` and `web/app/(app)/audit/action/[actionId]/page.tsx:152-153` both pass `tenantId="tenant_default", tenantLabel="Default tenant"`. Export bundle is a function-side artefact (a real ZIP the camera might download); the tenant id baked in is a constant string, not from session. Under live gateway this would mean every export is filed under "tenant_default" regardless of who clicked export. P2 because the demo MP4 doesn't record an export click (per runbook).

**Suggested fix**: thread `useViewerPolicy()` or a `useSession()` workspace id into the export dialog. Test that two tenant sessions get different `tenantId` in the bundle envelope.

---

### F11 [P3] — `formatHashForDisplay` ignores rows that come back without the `0x` prefix from the gateway

`web/components/skycot/ActivityRow.tsx:202-206` slices the body assuming hex chars. Gateway's `provenance.hash_chain_anchor` lives at `/api/v1/audit/<id>` is plain hex (no `0x` prefix). When/if F1 lands and the hash propagates, the formatter prepends `0x` automatically, so this is fine — flagging only because the rendered prefix is decorative ("looks like Etherscan") and not the actual on-chain format. Nit.

---

### F12 [P3] — Replay button posts to gateway with the persona id, gets 400, falls through to sample — silent failure pattern

`web/lib/api/replay.ts:259-272`: if auth-bypass is on, `sampleOrFailed(actionId)` returns hand-crafted outcome; if auth-bypass is off but `isMissingOrUnreachable`, falls through to sample. Verified `POST /api/v1/audit/act_01J0PRIYA_001/replay` returns 400 (not 404, because the id-format check is server-side). 400 is not handled by `isMissingOrUnreachable` — it would propagate as an `ApiError` and the mutation hook marks `isError: true`, rendering `<FailedView>` with `reason = "Replay sandbox unreachable"` (the default in `replay/page.tsx:87`). The investor reads "replay sandbox unreachable" instead of "id format mismatch". Cosmetic, but the error label is misleading. P3.

**Suggested fix**: distinguish 400 from network in `useReplay`'s error→outcome mapping, surface the 400 message verbatim.

---

## Resolved since last audit

The 2026-05-25 prior audit at the same path (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`) flagged:

- **F67-style fabrication on arbitrary `/audit/action/<arbitrary-id>/replay`** — RESOLVED. `web/lib/api/replay.ts:230-244` (`UNKNOWN_ACTION_FAILED` + `sampleOrFailed`) returns a real `{ kind: "failed" }` envelope rather than synthesising a "Policy regression detected" banner from the URL. Verified by reading the path-through logic. Drill-in to `AuditActionNotFound` also confirmed clean (no URL-derived content).
- **Audit-detail page missing for unknown ids** — RESOLVED. `<AuditActionNotFound>` renders a clean in-shell 404.

What's NOT resolved (carried forward): F1, F2, F3 above.

---

## Orientation score

**61 / 100**.

The visual register of /approvals (E-ops-dashboard) and /audit (B3-Etherscan) is locked and beautiful. The drill-in destination renders the right cards in the right order — when fed real data, it would narrate well. The replay-fabrication hole is closed.

But the function-side substrate is held together by mocks: the hash chain is fabricated (F1), the live-gateway adapter doesn't deserialize (F2), the approval-row click drops down to `window.location.assign` and an id-map stub (F3). For a POC demo recorded under auth-bypass mode (current config), the camera sees a polished dashboard backed by `policepolicepolicy`-tier theatre. The investor's "wait, click that hash again?" question lands on fixture data.

Two interventions get this to 85+ pre-recording:

1. **Decide where Scene 6 records from**. If it's auth-bypass mode, swap the synthetic FNV hash for a deterministic but honest-looking SHA-256-of-(id) (still fake, but at least labeled "demo audit chain" in the hover) — and re-seed the 5 persona detail ids in 32-char hex so the camera-visible id matches what the gateway would produce. If it's live mode, fix F2 (the detail adapter) and gateway-side add the `hash` field on feed rows.
2. **Fix F3** so the approval-row click is a Next router transition that survives the camera's eye, with no static id-mapping table.

F4 / F5 / F8 carry deferred — they're real but they live on the seam between v1.0 POC and v1.1 polish.
