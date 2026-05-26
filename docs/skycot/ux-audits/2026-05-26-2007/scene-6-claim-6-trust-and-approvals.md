# Scene 6 + Approvals — Claim 6 trust-and-approvals UX audit

**Date**: 2026-05-26 20:07 AEST
**Scope**: `/approvals` (E-ops-dashboard) + `/audit` (B3-Etherscan) + hash-chain hover + audit-row drill-in (REAL vs fabricated) + cross-tenant isolation negative tests + inbound channels visibility (SMS / Discord / Voice DTMF).
**Method**: read-only — direct HTTP probes of gateway + SPA, code-level review, and one camera-visible browser screenshot (subsequent visual checks deferred because concurrent worker activity in the swarm kept hijacking the preview tab — code + HTTP evidence is authoritative).
**Positioning rule applied**: function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`. The demo is POC/investor, not public release. Theatre the camera DOESN'T see ≠ blocker. Theatre the camera DOES see = blocker.
**Run config observed**: SPA on `:3000` with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` set in launch.json (but see F4: the code reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` — wrong name in launch.json so bypass is silently OFF). Gateway alive on `:7400`, schema v83 FRESH.

---

## TL;DR

13 findings. **4 P0 demo-blockers**: 3 carried forward unchanged from the 14:41 audit (F1 hash-chain theatre, F2 audit-detail adapter broken against real gateway, F3 ApprovalsOpsDashboard `window.location.assign` + id-mapping stub), plus 1 new P0 (F4: launch.json env-var typo silently disables auth-bypass so the camera-visible `/approvals` page is the EMPTY state, not the seeded SAMPLE_APPROVALS ops-dashboard; verified live screenshot shows "No approvals waiting"). 3 P1s (cross-tenant header is forge-able in dev mode; persona ids cannot round-trip to gateway; KPI "Active reviewers" still `—`). 3 P2s. 2 P3s. **One resolved since prior audit**: F10 (evidence export now binds to session tenant via `useViewerPolicy`).

**Biggest blocker for the camera**: Scene 6 cannot be recorded right now because the SPA's auth-bypass branch is silently off (F4) → /approvals shows EmptyState, /audit shows only gateway self-telemetry rows (`http.request` from "unknown") with no hash chip and no narrative content. Hand the runbook to anyone and the dashboard you've been polishing is invisible. Fix F4 first; F1/F2/F3 only matter once the page actually renders the seeded data.

---

## Headline blockers

1. **F4 [P0] — launch.json env-var typo (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` instead of `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`) means auth-bypass is OFF; `/approvals` page renders the empty state and `/audit` shows only gateway self-telemetry.** (New since 14:41 audit.)
2. **F1 [P0] — Hash-chain hover is FNV-1a fabrication under bypass and absent under live gateway** (unchanged).
3. **F2 [P0] — SPA `fetchAuditDetail` doesn't unwrap the gateway `{ detail: { ... } }` envelope; every consumer reads undefined fields off the root** (unchanged).
4. **F3 [P0] — `ApprovalsOpsDashboard` row click uses `window.location.assign` + a hand-keyed id-map stub** (unchanged).

---

## All findings, in order

### F1 [P0] — Etherscan hash-chain hover is fabrication when bypass works, and absent when it doesn't

Carried from the 14:41 audit, no movement.

- Under bypass: `web/lib/sample-audit.ts:160-177` (`rowHash`) seeds 8 FNV-1a passes from `worker.id|verb|timestamp` — the comment is honest ("Not a cryptographic hash; the real chain hash arrives from the gateway in production"), but the camera sees `0x…` monospace + a "Click to copy" tooltip and reads it as forensic. The drill-in then synthesises `policy.hash: "policepolicepolicy"` (`web/lib/sample-audit-detail.ts:257`).
- Under live gateway: `GET /api/v1/audit?range=7d` returns rows with keys `[action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker]` — verified just now, **no `hash` field on the feed**. The `HashCell` only renders if `activity.hash` is present (`web/components/skycot/ActivityRow.tsx:82-86`), so the Etherscan column collapses.

**Partial mitigation since 14:41**: the gateway's `/api/v1/audit/<id>` response now carries `provenance.hash_chain_anchor` (verified — `8d5f1019caa2a3e0daf3dbf2a3ff345009c502a52a33ae0e5f7671b9484ff4c7`, real sha256-shape) and PR #2364 added `ProvenanceBlock` rendering of a "Chain anchor" row when present. So the detail-page provenance card now CAN show a real hash. But (a) the FEED rows still lack a hash, so the Scene 6 hover-on-the-table beat stays theatrical, (b) the mock provenance under bypass doesn't include `hashChainAnchor` so the row stays invisible in dev, and (c) F2 below means the chain anchor doesn't survive deserialisation against the live gateway anyway.

**Repro**:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items[0] | keys'` → no `hash`
- `curl -s "http://127.0.0.1:7400/api/v1/audit/019e62a049057330bdae4e193e609f77" | jq .detail.provenance.hash_chain_anchor` → real sha256 hex
- `grep -n policepolicepolicy web/lib/sample-audit-detail.ts` → still on line 257

**Files**:
- `~/code/argo/web/lib/sample-audit.ts:160-177`
- `~/code/argo/web/lib/sample-audit-detail.ts:252-265`
- `~/code/argo/web/components/skycot/ActivityRow.tsx:82-86`

**Suggested fix**: (i) gateway projects `hash_chain_anchor` into the feed row; (ii) SPA stops synthesising the FNV hash under bypass (let the column collapse rather than fabricate); (iii) mock provenance carries a `hashChainAnchor` so the dev/demo card shows the real-looking row.

---

### F2 [P0] — `fetchAuditDetail` cannot deserialize the live gateway envelope

Unchanged. `web/lib/api/audit-detail.ts:58-77` calls `apiGet<AuditDetail>(path)` with NO normalisation. Confirmed gateway returns `{ detail: { summary: {...}, provenance: {...}, decision_chain, output, inputs, external_refs } }` with snake_case fields. Typescript-typed-as-AuditDetail compiles but every consumer reads `.id`/`.verb`/`.provenance.model.id` directly off the top-level object — those are undefined because they're actually nested under `.detail.summary.*`. Under bypass the path is short-circuited to `mockDetail(actionId)` so dev never hits this. **Switch off bypass and the drill-in renders blanks** — no narration possible.

Compare to the working pattern: `web/lib/api/audit.ts:209-222` (`normalizeAuditFeed`) explicitly unwraps and snake→camels. The detail client just doesn't.

**Files**:
- `~/code/argo/web/lib/api/audit-detail.ts:58-77`
- `~/code/argo/web/lib/types/audit-detail.ts:84-94`

**Suggested fix**: add `normalizeAuditDetail(raw: unknown): AuditDetail` mirroring `normalizeAuditFeed`. Unwrap `detail`, lift `summary.*` to root, rename `prompt_template`→`promptTemplate`, `data_classification`→`dataClassification`, `decision_chain`→`decisionChain`, `memory_snapshot`→`memorySnapshot`, `follow_up`→`followUp`. Map `provenance.hash_chain_anchor` → `provenance.hashChainAnchor` so PR #2364's `ProvenanceBlock` chain-anchor row renders against live data.

---

### F3 [P0] — Approval-row click hard-codes `window.location.assign` + composes drill-in id from a stub table

Unchanged. `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:300-310`:

```
const actionId = (item as ApprovalWithActionId).actionId ?? SAMPLE_APPROVAL_ACTION_IDS[item.id] ?? item.id;
const actionHref = `/audit/action/${encodeURIComponent(actionId)}`;
return <button onClick={() => { window.location.assign(actionHref); }} ...
```

Two problems for Scene 6:

1. `window.location.assign` causes a full page reload. AppShell layout, TanStack cache, demo banner timer all reset. Camera sees a flash.
2. The lookup `SAMPLE_APPROVAL_ACTION_IDS` at `ApprovalsOpsDashboard.tsx:26-34` is hand-keyed to the 8 sample `ap_01H9Z*` approval ids. Under live gateway, approvals arrive with different ids, table miss → fallback is `item.id` (an approval id, not an action id) → destination `/audit/action/ap_01H9Z7` will 404 into `AuditActionNotFound`.

Compare canonical pattern `web/components/skycot/ActivityRow.tsx` which uses `<Link href={href}>`.

**Files**:
- `~/code/argo/web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:25-34, 297-310`

**Suggested fix**: take `actionId` from `ApprovalRowJson.action_id` (already declared in `web/lib/api/approvals.ts:122`); render as a Next `<Link>` or `router.push`; delete `SAMPLE_APPROVAL_ACTION_IDS`. When gateway omits `action_id`, hide the row click target rather than fabricate a destination.

---

### F4 [P0 · NEW] — `.claude/launch.json` uses the wrong env-var name; auth-bypass is silently OFF; the seeded `/approvals` dashboard never renders

The launch config at `~/code/argo/.claude/launch.json` sets:

```
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1
```

…but every SPA module that gates the mock path on this flag reads:

```
process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
```

— note the `_ARGO_` infix. Locations:
- `web/lib/api/approvals.ts:74` (`authBypassEnabled`)
- `web/lib/hooks/use-viewer-policy.ts:116`
- `web/lib/hooks/use-redaction-policy.ts:32`
- `web/lib/hooks/use-approval-bundles.ts:95`
- `web/lib/api/retros.ts:79`
- `web/middleware.ts:58`
- `web/components/skycot/RequestElevatedView.tsx:38`

…among others. Vercel, README, playwright.config.ts, .env.example, the brief `fix-web-mock-bypass-env-var-rename` (MERGED in #2242), and a dedicated brief `fix-claude-launch-json-env-var-naming-mismatch` (BLOCKED in the ledger because the codex worker said launch.json is absent from `origin/main` — true, it's gitignored, but the file IS on the operator's disk).

The visible consequence (confirmed by a live screenshot at 20:11 AEST): `/approvals` hits the live gateway, gateway returns `{ items: [] }` (no seed data), page renders `<ApprovalsEmptyState>` ("No approvals waiting") instead of the locked `<ApprovalsOpsDashboard>`. The 5-KPI strip, 3 severity tiers, $ at risk donut, 7-day approval rate ring — none of it appears. The dashboard you're shipping is dark. The "Demo mode — showing sample data" banner up top still appears (because that one keys on `NEXT_PUBLIC_ARGO_DEMO_MODE`, a DIFFERENT env var, which IS in launch.json) — so the camera sees the banner promising sample data and a page that has none. That's worse than a clean live-mode look.

Same root cause makes `/audit` show only `http.request` rows from `worker={id:"", name:"unknown"}` (the gateway's own self-telemetry — that's all the data the live gateway has under no seed).

**Repro**:
- `grep NEXT_PUBLIC_GATEWAY_AUTH_BYPASS ~/code/argo/.claude/launch.json` → launch sets the wrong name
- `grep -n authBypassEnabled ~/code/argo/web/lib/api/approvals.ts` → reads `_ARGO_` name
- Visit http://127.0.0.1:3000/approvals → see EmptyState despite SAMPLE_APPROVALS containing 8 pending items

**Files**:
- `~/code/argo/.claude/launch.json:8`

**Suggested fix**: edit `.claude/launch.json` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. One-character-class fix (insert `ARGO_` after `NEXT_PUBLIC_`). The ledger brief `fix-claude-launch-json-env-var-naming-mismatch` should be flipped to TODO with a note that the file is operator-local-only (gitignored), so the "fix" is a manual edit on the operator's machine — no PR needed.

---

### F5 [P1 · NEW] — `X-Argo-Tenant-Id` is read straight from request headers in dev mode; in this run the gateway role-gate falls open to Owner so a forged header changes the data returned

`crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-41` and `routes_audit_detail.rs:93-95` read `X-Argo-Tenant-Id` directly off the request and use it as the SQL filter scope for `audit_record`. In production the role-gate (`role_gate.rs:296-311`) requires a verified JWT and falls closed to Guest; in dev mode (no `ARGO_GATEWAY_TRUST_ROLE_HEADER`, not strict-production) it falls open to Owner, which means the `audit.cross_tenant` capability is granted by default and `?as_tenant=` overrides are honored.

Verified live:
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items | length'` → 21
- `curl -s -H "X-Argo-Tenant-Id: tenant_acme" "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items | length'` → 0 (correctly empty for an unbound tenant)
- `curl -s "http://127.0.0.1:7400/api/v1/audit?range=30d&as_tenant=__legacy__" | jq '.items | length'` → 50 (the legacy bucket — a different tenant scope, accessed via query string without any role check on this dev binary)
- `curl -s -H "X-Argo-Tenant-Id: ws-evil" "http://127.0.0.1:7400/api/v1/audit/019e62a049057330bdae4e193e609f77" -o /dev/null -w "%{http_code}\n"` → 404 (the row's payload tenant_id doesn't match the forged header so isolation holds for THIS row)

So strict cross-tenant data exfil doesn't happen on the row I tried — but it CAN happen on tenants whose ids you can guess: nothing in the dev gateway verifies that the principal owns the tenant they're claiming via header. The `claim_6_live_cross_tenant` cargo test passes because the test runs in production-mode posture (JWT-required); the running dev gateway does not. That mismatch is invisible from the SPA — there's no UI affordance to demonstrate isolation either way.

Carry-forward of the prior audit's F5 (SPA never sends a tenant header) is still true; this finding is a sharper restatement.

**Files**:
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-41`
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:93-95`
- `~/code/argo/crates/argo-cli/src/handlers/gateway/role_gate.rs:286-311`

**Suggested fix**: out-of-scope for the demo MP4 (no UX surface, and the production posture is fine), but for v1.1: add a workspace switcher to the AppShell so the camera CAN demonstrate "log in as tenant B, see no rows leak" against a hardened dev binary that enforces principal→tenant binding on the header.

---

### F6 [P1] — Persona action ids (`act_01J0PRIYA_001`) cannot round-trip to the live gateway

Unchanged from prior audit. `web/lib/sample-audit-detail.ts` ids are `act_01J0<NAME>_NNN`; gateway requires 32-char hex. `POST /api/v1/audit/act_01J0PRIYA_001/replay` returns `400 audit id must be 32-char hex (16 bytes)`. As long as F4 stays broken the SPA never tries to round-trip these, but the moment auth-bypass is fixed AND a user clicks Replay against a persona id, the failure surfaces as a generic "Replay sandbox unreachable" — see F11.

**Suggested fix**: regenerate sample-audit-detail ids as 32-char hex now that `mission-demo-seed-data-bootstrap`'s seed CLI is landing.

---

### F7 [P1] — Approvals "Active reviewers" KPI permanently `—`

Unchanged. `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-150` hard-codes `value: "—", sub: "reviewer roster pending"`. Once F4 is fixed and the dashboard renders, the 5th KPI cell visibly says "shrug emoji" next to four populated cells. PR #2299 / #2319 fixed two other KPI-fabrication issues but left this one in.

**Suggested fix**: populate from `pending.flatMap(p => p.approver?.name)` distinct-count (cheap), or drop the cell to a 4-up strip.

---

### F8 [P2] — Approval-rate ring shows `—` because every seeded approval has `decision == undefined`

Unchanged. All 8 `SAMPLE_APPROVALS` are pending → `computeApprovalRate(items)` returns `null` → right-hand aside renders the empty ring. The most decorative element on the page won't animate on camera unless the operator approves something during the take.

**Suggested fix**: seed 2-3 with `decision: "approve"` so the ring lands populated on first paint.

---

### F9 [P2] — Inbound channels (SMS, Discord, Voice DTMF) are invisible to the operator in the SPA

PR #2224 (SMS inbound) ships `POST /twilio/sms/decision`. PR #2222 (Discord) ships `POST /channels/discord/decision`. PR #2208 (Voice DTMF) ships `POST /twilio/voice/decision`. All three routes are server-side reachable (verified — SMS returns 500 without HMAC signature, which is the secure path).

But there's no SPA surface where the operator can: (a) see the inbound webhook URLs they need to paste into Twilio / Discord, (b) see inbound activity (recent SMS replies / DTMF decisions), (c) test the webhook end-to-end with a tap. The closest thing is `/settings/notify-channels`, which is OUTBOUND notification config; the inbound URLs live only as `hint:` strings in the registry env-var requirement list (e.g. `ARGO_TWILIO_VOICE_DECISION_URL` at `web/lib/notify-channels/registry.ts:198`).

For Scene 6's "Claim 6 multi-channel" narrative, the operator can't point at the inbound surface and say "approvals come back from any of these channels". The capability is real (PRs MERGED, routes 500 on missing HMAC not 404) but unreachable from the UI.

**Files**:
- `~/code/argo/web/lib/notify-channels/registry.ts` (outbound-only)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_channels_sms.rs:32` (inbound handler)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs:26`
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_channels_discord*.rs:14`

**Suggested fix (v1.1)**: add a "Inbound" tab to `/settings/notify-channels` that shows: webhook URLs to paste, recent inbound events (last 20), a "Send test" button. Out-of-scope for v1.0 MP4 unless Scene 6 narrates this beat.

---

### F10 [P2] — `/approvals` standalone page chrome diverges from canonical `/` inbox

Unchanged. `web/app/(app)/approvals/page.tsx:30-48` renders `<BrandLoudHero>` + `<ApprovalsOpsDashboard>`; the canonical inbox at `/` uses a different header pattern (KpiStrip + FilterChipRail + DataSourceChip). Anyone typing `/approvals` directly into the URL gets a slightly different page than the sidebar link. Polish gap.

**Suggested fix**: pick one canonical path. The page comment says it exists to avoid `not-found.tsx`; either `redirect('/')` or give it a stripped-down chrome that doesn't ship the ops-dashboard component twice in different shells.

---

### F11 [P3] — Replay button on the persona id surfaces "Replay sandbox unreachable" instead of the real reason

`web/lib/api/replay.ts:259-272`: if bypass is off but `isMissingOrUnreachable(err)`, falls through to sample. 400 isn't matched, so the mutation hook marks `isError: true` and `<FailedView>` renders `reason = "Replay sandbox unreachable"` (default in `replay/page.tsx:87`). The investor reads a generic network failure instead of "id format mismatch". Cosmetic — F4 has to fix first.

**Suggested fix**: distinguish 400 from network in `useReplay`'s error mapping, surface the 400 message verbatim.

---

### F12 [P3] — `formatHashForDisplay` prepends `0x` decoration even when the gateway anchor is plain hex

Unchanged. `web/components/skycot/ActivityRow.tsx:202-206` formats hex chars assuming Etherscan style. Gateway's `provenance.hash_chain_anchor` is plain hex (no `0x`). When F1 lands, the formatter prepends `0x` automatically — that's fine for visual familiarity but it's not the on-chain format. Nit only because the camera reads "Etherscan-like" not "this is an Ethereum address".

---

### F13 [P3] — Demo-mode banner says "showing sample data" even when sample data isn't being shown (F4 cascade)

Cosmetic but worth noting: the banner copy is hardcoded at `web/components/skycot/DemoModeBanner.tsx` (or similar — gates on `NEXT_PUBLIC_ARGO_DEMO_MODE`). Under the current broken-bypass state, the banner promises sample data and the page below it is empty. After F4 is fixed this resolves on its own. Logging here only so future me doesn't waste time chasing it as a separate bug.

---

## Resolved since last audit (2026-05-26 14:41)

- **F10 (prior) — `EvidenceBundleExporter` hard-coded `tenantId="tenant_default"`** — RESOLVED. Commit `c2d5d4ea1` (2026-05-26) threads `useViewerPolicy().tenantId` into both audit-feed and audit-detail pages. Verified at `web/app/(app)/audit/page.tsx:179-180` and `web/app/(app)/audit/action/[actionId]/page.tsx:79-80`.
- **Partial movement on F1** — Gateway now emits `provenance.hash_chain_anchor` on detail responses (real sha256), and PR #2364 added `ProvenanceBlock` rendering of the "Chain anchor" row when present. But the feed still lacks the hash, so the Scene 6 hover beat stays theatrical, and F2 blocks live reading.
- **PR #2297 (Fix audit list/detail tenant scope)** — narrows the legacy-bucket inclusion when an explicit workspace tenant header is supplied. Solid fix for the scope-leak edge case; doesn't change the broader F5 picture (header is still client-trusted in dev mode).
- **PR #2367 (scrub Sam/Rio from sample fixtures)** — Scene-1 roster lock, doesn't affect this scope.

What's NOT resolved (carried forward): F1, F2, F3 (P0s). New since 14:41: F4 (P0), F5 (P1 — sharpened).

---

## Orientation score

**52 / 100** (down from 61 / 100 at 14:41).

The visual register of the locked designs is still beautiful, and the gateway-side chain-anchor + tenant-scope fixes are real improvements at the substrate. But F4 turns the camera-visible state into an empty dashboard with a confused "showing sample data" banner — worse than the polished-but-theatrical state the 14:41 audit captured. F1/F2/F3 remain unaddressed, F5 sharpens to a dev-mode trust gap, F9 surfaces the inbound-channel invisibility for the first time.

**Two interventions get this back to 70+ pre-recording**:

1. **Fix F4 first** (one-line edit to `.claude/launch.json`). The dashboard renders. Now F1/F2/F3 become visible problems again. Without F4 they're invisible problems hiding behind an empty state.
2. **Decide Scene 6's recording mode**: bypass (F1 sub-fix: honest-looking SHA-256-of-id under bypass + persona ids in 32-char hex) OR live-with-seed (F2 fix: write the detail-response normaliser; F1 sub-fix: feed-row hash projection on gateway). Pick one path and commit.

F3 is a 20-minute fix (drop the static table, use `ApprovalRowJson.action_id`, swap `window.location.assign` for `router.push`). Do it regardless of which Scene 6 path.

F9 is a v1.1 surface — flag in the runbook so Scene 6's "multi-channel" claim doesn't promise more UI than exists.
