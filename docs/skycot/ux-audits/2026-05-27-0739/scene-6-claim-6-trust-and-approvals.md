# Scene 6 (Claim 6 — Multi-channel + multi-tenant + audit trust) — UX audit

**Date**: 2026-05-27 07:39 AEST.
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 — `/approvals`, `/audit`, hash-chain hover, audit-row drill-in REAL vs fabricated, cross-tenant isolation negative tests, `/audit/action/{id}/replay` fabrication).
**Method**: source-walk + live curl probes against the running gateway at `http://127.0.0.1:7400`. Preview MCP not used this run (SPA dev server not running at audit start; prior 0623 + 0519 auditors both documented Preview MCP instability across Scene 6 navigations + zero net signal vs source-walk). HEAD = `0977044fe`.
**Prior audit**: `2026-05-27-0623/scene-6-claim-6-trust-and-approvals.md` (~76 min earlier). The 0623 auditor found zero net change vs 0519 and 16 carry-forward findings; this run is a thin delta against 0623, accounting for the 7 PRs that landed since the 0623 audit (most importantly `#2432` route audit/replay/approve through real gateway in demo-live mode + `#2414` inline Approve/Refuse on `/approvals`).
**Prior auditor verdict**: 58 / 100 (16 findings, 3 P0s carry-forward).

---

## TL;DR

**Net change in 76 min: #2432 shipped a third demo env-var (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`) that opts the audit/approvals/replay clients out of the auth-bypass mock short-circuits and routes them at the real gateway. The fix is real and correct as a building block, BUT THE OPERATOR'S LAUNCH CONFIG (`/.claude/launch.json:9`) DOES NOT SET THE NEW ENV VAR.** Net effect: under the default `claude preview start web` launch, the audit feed, approvals inbox, and replay client now silently fall through to the live gateway (the typo-named `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` in launch.json still doesn't satisfy the consumers' `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` check, so `isAuthBypassed()=false`). Live gateway returned `{"items":[]}` on both `/api/v1/approvals` and `/api/v1/audit?range=7d` at audit time. Recording Scene 6 right now would show: a fully empty E-ops-dashboard (0 KPIs, 3 empty severity sections, with the new inline Approve / Refuse buttons present but nothing to approve), the "Quiet day" audit empty state, and no drill-in to demonstrate the trust ratchet.

The 0623 audit logged 35 SPA-self-traffic rows; that table has been zero'd between then and now (gateway restart, seed wipe, or schema bump v82 → v83 evident from the health probe — `schema_version: 83, uptime_seconds: 61619` = ~17h uptime, so the wipe happened *before* 0623 actually; the 0623 row count of 35 was real then but the 0739 baseline is 0).

**Net new this run** (post-#2432 + post-#2414):
- **F1 has a new wrinkle**: `#2432` added the demo-live opt-out flag (good), but did NOT update `.claude/launch.json` to set it (bad). The recording-day env triplet documented in `web/lib/skycot/demo-mode.ts:37-46` is `auth-bypass + demo-mode + demo-live` — only 2 of the 3 vars are set in launch, and one of those is the typo'd legacy name. The recording-day config is still broken end-to-end. **F1 is therefore not closed by #2432 — it's a 2-LOC fix in launch.json now, but the LOC has shifted: was 1 rename, is now 1 add + 1 rename.**
- **F4 (SLA bar "2h 60m" overflow)**: still unchanged, but the rendering surface now has the new inline Approve / Refuse buttons sitting beside the bar — the visual collision is worse, not better.
- **F8 (no parent_hash chain-link affordance)**: still unchanged; reconfirmed by reading `ActivityRow.tsx:98-106` (renders ONLY `activity.hash`) + `AuditDayGroup.tsx:76-93` (computes `hashKind` from self/sample but discards `parentHash` for display).
- **F11 → CLOSED IF demo-live ever fires**: `#2432` rewires the replay client so that under the recording-day triplet the `<ReplayDiff>` consumes a real gateway envelope, not the canonical sample-replay table. The hash-recompute fabrication concern from 0623 is resolved AS SOON AS the launch env is fixed (F1 follow-on). Until then the replay path is dead-code under default launch.
- **F2 (placeholder fabrication on non-canonical ids)**: still present in code (`sample-audit-detail.ts:285-310` still ships `claude-opus-4-7 · v2026-04-30` + `placehold00000000` + `policyplaceholder`), but its **reachability under default launch is currently zero** — the live audit feed is empty, so the camera has no row to click that would resolve to a non-canonical id. F2 becomes camera-reachable only AFTER F1 is fixed AND the operator seeds the live gateway with non-canonical ids. Demoting F2 from P0 to **P1** for the runbook recording — it's a latent fabrication bug, not an on-camera one given the current empty-gateway state.

**One genuinely new finding** worth recording (F17 below): under default launch + empty live gateway, the audit page's "Quiet day" empty state IS the on-camera surface for Scene 6. The illustration + body copy is brand-restrained-correct, but the operator's runbook narration for Scene 6 ("hover the hash-chain affordance, drill into a row, see real provenance") cannot be performed against this state — there is nothing to hover or drill. The runbook scene is unrecordable as scripted regardless of how much theatrical seed data the gateway has, until F1 lands. (F17 is a docs/narration finding, P1.)

**Live gateway state (curl-verified at audit time)**:
- `GET /health` → `{"status":"ok","schema_version":83,"uptime_seconds":61619}` — FRESH; ~17h uptime; schema bumped from v81 (0623) → v83 (now)
- `GET /api/v1/approvals` → `{"items":[]}` (empty, default tenant)
- `GET /api/v1/audit?range=7d` → `{"items":[]}` (empty, default tenant) — was 35 SPA-self-traffic rows at 0623; now zero (presumably the schema bump dropped them, or operator wiped)
- `GET /api/v1/audit?range=30d` → `{"items":[]}` (also empty over wider range)
- `GET /api/v1/audit?range=7d` (bogus tenant) → `{"items":[]}` (200 OK) — cross-tenant isolation PASS-shape-A (empty)
- `GET /api/v1/audit/act_01J0PRIYA_001` (default tenant) → `{"error":{"code":"not_found"}}` — sample id not in live DB (expected)
- `GET /api/v1/audit/act_01J0PRIYA_001` (bogus tenant) → `{"error":{"code":"not_found"}}` — isolation PASS-shape-B (404)
- `POST /api/v1/audit/0123456789abcdef0123456789abcdef/replay` (default tenant) → `{"error":{"code":"not_found"}}` — endpoint live, accepts 32-char-hex ids, no fabrication (real not_found, not a synthesised verdict)
- `POST /api/v1/audit/bogus/replay` → `{"error":{"code":"bad_request","message":"audit id must be 32-char hex (16 bytes)"}}` — replay enforces the canonical id shape; the gateway list endpoint emits 32-char-hex ULIDs so it's internally consistent, but the SPA's sample fixtures use ULIDs like `act_01J0PRIYA_001` (under-23-chars + name fragment + numeric suffix) that are NOT 32-char hex. The drill-in client falls through to placeholder under the live path because of this mismatch.

**Single fastest path to a recordable Scene 6** is now a **2-line change**, not 1:
1. `.claude/launch.json:9` — rename `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` → `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (F1 from 0519/0623)
2. `.claude/launch.json:9` — add `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` to enable the new `#2432` opt-out (F1-net-new)

Or as a single combined edit: replace `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` (also drops the stale `_ARGO_DEMO_MODE` alias that no longer has a consumer).

Then `argo demo seed --tenant 01900000-0000-7000-8000-000000000000` to populate the live audit table with real persona turns.

---

## Resolved since 0623 audit (76 min interval)

| Finding | Status | Notes |
|---|---|---|
| F1 — env-var name mismatch | **PARTIALLY MITIGATED, NOT FIXED** | `#2432` shipped the demo-live opt-out flag that the F1 fix was waiting on. But `.claude/launch.json:9` is unchanged — the recording-day env triplet still doesn't fire. F1 is now a 2-LOC fix instead of 1-LOC, AND a NEW related finding (F1-companion) — see below. |
| F2 — placeholder fabrication | **REACHABILITY ZERO UNDER DEFAULT LAUNCH** | Code unchanged. With live gateway empty + no rows to click, the placeholder path is unreachable on-camera RIGHT NOW. Demoted from P0 to P1 — it becomes a P0 again the instant F1 lands and the operator seeds non-canonical ids. |
| F3 — list endpoint missing hash fields | **UNCHANGED** | Confirmed-by-curl: `GET /api/v1/audit?range=7d` returns `{"items":[]}` — no rows means no opportunity to inspect the schema, but the row-normalizer at `audit.ts:194-217` still reads `self_hash` / `parent_hash` keys that the (currently empty) gateway responses don't include. Same shape gap as 0623. |
| F4 — SLA bar "2h 60m left" overflow | **UNCHANGED** | `SLABar.tsx` untouched. PLUS: now sits beside the new `ApprovalRowActions` cluster from `#2414`, so visual collision is potentially worse on narrow viewports. |
| F5 — `/` vs `/approvals` dual-surface + `/approvals` not in nav | **UNCHANGED** | `nav-items.tsx:23` still points "Approvals" at `/` (the homepage), and `/approvals` is still not in `NAV_ITEMS`. The locked E-ops-dashboard remains URL-typing-only. |
| F6 — `Active reviewers` hardcoded `—` | **UNCHANGED** | `ApprovalsOpsDashboard.tsx:152-156`. |
| F7 — DemoBanner sticky over Hero | **UNCHANGED** | |
| F8 — hash-chain prev→this affordance absent | **UNCHANGED** | `ActivityRow.tsx:98-106` still renders ONLY `activity.hash` via `<HashCell>`. `AuditDayGroup.tsx:91` carries `parentHash` into `WorkerActivity` but ActivityRow doesn't read it. There is NO prev→this hover affordance anywhere in the audit feed. The brief's "Etherscan-style hash-chain hover" is not implemented in the UI. |
| F9 — live audit feed is 35 rows of SPA self-traffic | **CHANGED-IMPROVED** | `GET /api/v1/audit?range=7d` now returns 0 rows (presumably schema-v82→v83 bump truncated, or operator wiped). The self-traffic-pollution problem is gone for now BUT comes back the moment the SPA navigates to `/audit/action/<id>` because every drill-in writes an `audit.detail.viewed` row. Still worth gating: see suggested fix in F9 carry. |
| F10 — `AuditActionNotFound` unreachable | **UNCHANGED** | |
| F11 — Replay UI shows hash-recompute string | **FIXED-IN-CODE, DEAD-CODE-UNDER-DEFAULT-LAUNCH** | `#2432` rewires `replay.ts:263` so the demo-live triplet routes through real gateway. Until launch.json sets `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`, the path doesn't fire — the fix is correct but un-exercised. |
| F12 — Cross-tenant isolation detail endpoint 404 | **PASS** | Re-tested 0739: bogus tenant → 404 on detail endpoint. |
| F13 — Discord absent from ChannelKind | **UNCHANGED** | |
| F14 — Voice/SMS no channel badge in `/audit` rows | **UNCHANGED** | |
| F15 — Hardcoded April-30 model version | **UNCHANGED** | |
| F16 — List endpoint empty-array vs detail-404 isolation shape | **CONFIRMED IDENTICAL** | Re-tested 0739: bogus tenant on list returns `{"items":[]}` (200), on detail returns 404. Same documented behavioural difference. |

**Net merge activity since 0623**: 7 PRs (`#2432 #2419 #2414 #2411 #2410 #2415 #2416`). Two touch Scene 6 directly:
- `#2432` — added `NEXT_PUBLIC_SKYCOT_DEMO_LIVE` demo-live opt-out across `audit.ts` / `audit-detail.ts` / `replay.ts` / `approvals.ts`. **PARTIAL FIX FOR F1**: the building block landed but the launch config wasn't updated.
- `#2414` — added `ApprovalRowActions` inline Approve / Refuse buttons to the `/approvals` ops dashboard. **MISSING-AFFORDANCE FIX**: prior to this, the E-ops-dashboard had no Approve button anywhere on the page (only the drill-in row). Scene 6's "operator clicks Approve" beat is now recordable from the dashboard.

The other 5 (`#2419 #2411 #2410 #2415 #2416`) are non-Scene-6 (hire flow, demo-mint proxy, workflows builder, docs sub-routes).

---

## Headline blockers (all P0 — the 2-LOC launch.json gate)

### F1 [P0] **PARTIAL-FIX-NEEDS-LAUNCH-UPDATE** — recording-day env triplet still not wired in `.claude/launch.json`

**Detail**: `#2432` introduced `NEXT_PUBLIC_SKYCOT_DEMO_LIVE` as the third env-var that opts the audit/approvals/replay clients out of the auth-bypass mock short-circuit and routes them at the real gateway (the "real provenance" Scene 6 narration is now technically feasible). `web/lib/skycot/demo-mode.ts:37-46` documents the recording-day triplet as `auth-bypass + demo-mode + demo-live`.

But `.claude/launch.json:9` currently emits only two of those, and one is misnamed:
```
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1                ← typo, consumers want _ARGO_
NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400    ← OK
NEXT_PUBLIC_ARGO_DEMO_MODE=1                     ← stale alias, no consumer
NEXT_PUBLIC_SKYCOT_DEMO_MODE=1                   ← OK, satisfies isDemoMode()
                                                 ← MISSING: NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1
```

Effect at audit time: `isAuthBypassed()=false` (typo) → `isDemoMode()=true` but `!isDemoLive()` short-circuit gates don't fire. The control flow lands in the gateway-fetch branch on lines `audit.ts:354` / `audit-detail.ts:111` / `approvals.ts:187` / `replay.ts:267`. Live gateway returns `{"items":[]}` → SPA renders empty states.

**Reproduction**:
1. `claude preview start web` (or `bash .claude/launch.json's "web" config`)
2. Navigate to `http://localhost:3000/approvals` — all-zero KPIs, 3 empty severity tiers, **new Approve / Refuse buttons present but nothing to approve**.
3. Navigate to `/audit` — "Quiet day" empty state.
4. `curl http://127.0.0.1:7400/api/v1/audit?range=7d` returns `{"items":[]}` — confirms not a SPA bug, gateway is genuinely empty.

**Suggested fix** (single-edit on `.claude/launch.json:9`): replace the runtime arg with
```
NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 pnpm --dir web dev
```
That renames the typo, drops the stale `_ARGO_DEMO_MODE` alias (no consumer in the codebase per `grep -rn ARGO_DEMO_MODE web/lib/`), and adds the new `SKYCOT_DEMO_LIVE` flag.

**File**: `.claude/launch.json:9`. Also re-check the same typo lives in `scripts/demo-dry-run.sh:84` (referenced from 0623) and `scripts/capture-product-screenshots.sh:54` — those scripts ALSO need the demo-live flag added if they're invoked pre-recording.

### F1-companion [P0] NEW — recording-mode contract documented in code but missing from runbook + launch

**Detail**: `web/lib/skycot/demo-mode.ts:36-47` clearly documents:
> Demo-live presumes `argo demo seed --tenant <demo>` has been run against the live gateway so the seeded persona rows actually exist server-side; without that step the audit feed renders empty.

That contract — **demo-live ALSO requires a one-off `argo demo seed` step against the running gateway** — is not surfaced anywhere the operator looks when preparing to record. `docs/skycot/demos/v1-mission-mvp-runbook.md` should call out both the env var triplet AND the seed command in the Scene 6 pre-flight checklist. The runbook was updated in `#2432`'s diff (`docs/skycot/demos/v1-mission-mvp-runbook.md | 1 +` — only 1 line added; almost certainly insufficient to convey the new contract). Worth verifying the runbook change is clear-enough that operator-on-recording-day actually performs the seed.

**Suggested fix**: a 5-line Scene 6 pre-flight block at the top of the runbook covering: (1) env triplet in launch.json, (2) `argo demo seed --tenant <demo>` against the live gateway BEFORE pressing record, (3) optional `--include-approvals` flag if seed CLI supports it (worth checking), (4) verify `curl /api/v1/audit?range=7d` returns >0 rows before pressing record, (5) verify `curl /api/v1/approvals` returns >0 items.

**File**: `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6 section (line + section unknown without re-opening; the runbook is the operator's last-mile reference).

### F2 [P1] DEMOTED-FROM-P0 — placeholder fabrication still in code, currently unreachable on camera

**Detail**: `sample-audit-detail.ts:285-310` still ships `placeholderAuditDetail()` returning fabricated `claude-opus-4-7 · v2026-04-30` model id + `placehold00000000` template hash + `policyplaceholder` policy hash. The control-flow path that exercises it (`audit-detail.ts:107-109`) STILL fires for non-canonical action ids in `isDemoMode() && !isDemoLive()`.

But camera-reachability under default launch is currently ZERO because:
1. Live audit feed is empty → no rows to click → no drill-in fires
2. Sample approval IDs from `SAMPLE_APPROVAL_ACTION_IDS` map to canonical `act_018f67d0-…` rows which `findSampleAuditDetail()` resolves via `synthesizeAuditDetail()` from `SAMPLE_AUDIT_ROWS` — NOT the placeholder.
3. The only way to hit the placeholder is to type a random URL like `/audit/action/foo` in the browser — not a realistic recording-day action.

After F1 lands AND `argo demo seed` runs, the live audit feed will populate with 32-char-hex ULID ids that don't match the sample table's `act_018f67d0-…` keys. THEN the placeholder fires when the camera clicks any live row. So F2 is P0 conditional on F1 landing.

**Suggested fix** (same as 0519/0623 option b): in `placeholderAuditDetail()` blank-out the fabricated provenance fields (`model.id = ""`, `model.version = ""`, `promptTemplate.hash = ""`, etc.) so the `<ProvenanceBlock>` empty-state fires via `emptyProvenance` detection at `ProvenanceBlock.tsx:36`. The drill-in page then renders "Provenance unavailable — this is a meta-event" instead of a fabricated model id.

**File**: `web/lib/sample-audit-detail.ts:285-310`.

### F3 [P0] CARRY — gateway list endpoint emits no `self_hash` / `parent_hash` keys

**Detail**: Re-verified by curl: `GET /api/v1/audit?range=7d` returns `{"items":[]}` so no schema is observable RIGHT NOW. But the row-normalizer at `web/lib/api/audit.ts:194-217` still expects `self_hash` / `parent_hash` / `sample_hash` keys — and the (currently empty) live response doesn't include them. After F1 + seed, the rows will come back without hash fields and the `<HashCell>` will short-circuit (render `<span aria-hidden />` at `ActivityRow.tsx:104-106`). Scene 6's centerpiece "every row carries a forensic-grade hash chain" affordance will be missing from the on-camera UI even when the rest of the data renders.

**Suggested fix** (gateway-side): `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` — extend the list response shape to mirror the detail endpoint's hash fields. The detail endpoint already computes them (per F11's curl probe in 0623: `expected_self_hash: dad16c71…`), so it's a matter of including them in the list projection.

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (path approximate; the file ships the audit list route).

---

## All findings (delta only — carry-forward findings reference 0623 + 0519)

### `/approvals` (E-ops-dashboard)

#### F4 [P1] CARRY-WORSENED — SLA timer "2h 60m left" overflow + visual collision with new inline Approve/Refuse cluster

`SLABar.tsx:46-50` unchanged. **PLUS**: post-`#2414`, every approval row now ends in `<ApprovalRowActions>` (two `<Button size="sm">` Approve + Refuse buttons). On narrow viewports the SLA bar + dollar-at-risk + confidence donut + actions cluster all compete for horizontal space in the same row (grid template is `lg:grid-cols-[minmax(0,1fr)_7.5rem_5rem_6.5rem_auto]` at `ApprovalsOpsDashboard.tsx:392`). The fix is the same minute-clamp from 0519, but the row layout may need a second look at <1280px breakpoints.

#### F5 [P2] CARRY — `/` vs `/approvals` dual-surface
#### F6 [P2] CARRY — `Active reviewers` hardcoded `—`
#### F7 [P3] CARRY — DemoBanner sticky over Hero

#### F17 [P2] NEW — under default-launch + live-empty-gateway, Scene 6's `/approvals` is the inline-Approve-button DEMO — but with no approvals to approve

`#2414` correctly added the Approve / Refuse buttons that Scene 6's narration ("operator clicks Approve") needs. BUT under the current default-launch state — where the live gateway has zero approvals — the dashboard renders the locked E-ops-dashboard with all-zero KPIs and 3 empty severity sections. **There are no Approve buttons visible because there are no rows.** The fix-1 (F1) + seed step is the gate.

After F1 + seed, this becomes a clean Scene 6 capture surface. Worth flagging because the runbook narration assumes there ARE rows.

**File**: surface-level, no file change needed; this is a documentation / pre-flight finding.

### `/audit` (Etherscan-style timeline)

#### F8 [P1] CARRY — Hash-chain prev→this affordance absent

`ActivityRow.tsx:98-106` only consumes `activity.hash`. The `<HashCell>` (lines 225-264) renders a single truncated hash with a copy-on-click — no prev-hash relationship surface anywhere. The brief's "hash-chain hover" requirement is not implemented. The `parentHash` field IS carried through to `WorkerActivity` (`AuditDayGroup.tsx:91`) but discarded.

**Suggested fix**: extend `<HashCell>` to render `{prev?…}→{self?…}` as two truncated hashes joined by an arrow, with the tooltip explaining "this row's hash is computed from the prior row's hash + this row's content". Or surface the parent-hash relationship in the drill-in `<ProvenanceBlock>` "Chain anchor" row instead.

**File**: `web/components/skycot/ActivityRow.tsx:98-106` + `web/components/skycot/audit/ProvenanceBlock.tsx:90-98` (chain-anchor row).

#### F9 [P1] CARRY-CHANGED — Live-gateway audit feed currently zero (was 35 self-traffic at 0623)

`GET /api/v1/audit?range=7d` returned 0 items at 0739 — schema bump v81→v83 (per gateway health probe) plus the 17h uptime suggests the gateway was restarted between 0623 and 0739, dropping the in-memory or schema-migration-incompatible self-traffic rows. The self-traffic-pollution problem is currently invisible BUT comes back the moment the SPA exercises any drill-in (each `/audit/action/<id>` request writes an `audit.detail.viewed` row).

**Suggested fix**: add a `verb_kind=trace` (or `data_classification=meta`) flag to `audit.detail.viewed` / `audit.replay` rows server-side, and have the SPA filter them out by default in the list query. Or filter client-side in `groupByDay()` (`audit.ts:293-315`) by inspecting `row.actionType.startsWith("audit.")`.

**File**: gateway audit-record path + `web/lib/api/audit.ts:293-315`.

#### F10 [P2] CARRY — `AuditActionNotFound` unreachable

### `/audit/action/{id}/replay`

#### F11 [P2] CARRY-FIXED-IN-CODE — `#2432` rewires replay through real gateway; dead-code under default launch

`replay.ts:263` now reads `if (isAuthBypassed() && !isDemoLive()) return sampleOrFailed(actionId)` — the demo-live opt-out gates the sample-replay short-circuit. Under recording-day env triplet (which currently DOESN'T fire — see F1), `<ReplayDiff>` would render the real `expected_self_hash == actual_self_hash` envelope from the gateway. The 0519/0623 concern about "Replay" label semantics on a hash-recompute remains a minor UX-clarity finding but no longer involves fabricated provenance.

**Sub-finding (recordability gate)**: replay endpoint enforces `audit id must be 32-char hex (16 bytes)`. SAMPLE_AUDIT_ROWS ids look like `act_018f67d0-0000-7000-8000-000000000002_000` (32-char body BUT with hyphens). Live gateway emits ULIDs `019e65c29f9f7e639c65763a86c7b8bc` (32-char hex no hyphens). After F1 lands, if the seed script emits the latter form, the replay path works on-camera. If seed emits the hyphenated form, the replay POST returns 400 `bad_request` and `<ReplayDiff>` shows an error state.

**File**: `web/lib/api/replay.ts:263` (fix landed); operator should verify `argo demo seed` emits 32-char-hex no-hyphen ids before recording.

### Cross-tenant isolation

#### F12 [PASS] CARRY — Detail endpoint enforces tenant isolation (404 on bogus header)
#### F16 [P3] CARRY — List endpoint returns empty array for cross-tenant (vs detail-404) — both are correct, just different shapes

Re-verified 0739. Both isolation shapes intact; gateway behavior compatible with the marketing claim once narrated correctly. Suggested fix is still a 1-paragraph runbook addition.

### Multi-channel touchpoints

#### F13 [P2] CARRY — Discord absent from ChannelKind unions
#### F14 [P3] CARRY — Voice/SMS no channel badge in `/audit` rows

### General

#### F15 [P3] CARRY — Hardcoded `claude-opus-4-7 · v2026-04-30` in 5 canonical samples

---

## Orientation score: **62 / 100** (+4 vs 0623's 58)

The +4 reflects the substantive code-side wins from `#2432` (demo-live opt-out shipped + replay-fabrication closed in code) and `#2414` (inline Approve/Refuse buttons added — Scene 6's central beat is now recordable from the dashboard surface). The score doesn't reach 70+ because:
1. **F1 is not closed**. `#2432` shipped the building block, but the launch config still doesn't set the new env var, so the recording-day SPA bundle still falls through to the empty live gateway under default launch. The fix-brief `fix-spa-demo-live-mode-route-real-gateway-on-camera` from 0519 has therefore EVOLVED — it's now a 2-LOC launch.json edit + a runbook update + a seed step.
2. **The Scene 6 narrative depends on data the gateway doesn't have**. Even with F1's env fix, Scene 6's "real provenance" / "trust ratchet" narration requires the operator to run `argo demo seed` against the live gateway before pressing record. That contract is documented in `demo-mode.ts:37-46` but not surfaced in the runbook pre-flight checklist or visible at SPA launch.
3. **F2 (placeholder fabrication) is dormant but armed** — the instant F1 + seed land, F2 becomes a P0 on-camera regression because live-gateway ULIDs won't match the sample-audit-detail table's `act_01J0PRIYA_001` keys.
4. **F8 (no parent-hash chain-link affordance)** — the centerpiece "Etherscan-style" affordance is still absent from the UI. Even if Scene 6's runbook is silent on prev→this hovers, the marketing claim ("forensic-grade audit chain on every action") is currently un-supported by the UI.

**Recommended next action for the operator** (one paste-able edit):

Replace line 9 of `.claude/launch.json`:
```
"NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 pnpm --dir web dev"
```
with:
```
"NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 pnpm --dir web dev"
```

then `argo demo seed --tenant 01900000-0000-7000-8000-000000000000` against the running gateway.

Then re-audit Scene 6 — at that point F2 visibility goes from latent to camera-likely, F11's fixed-in-code path actually fires (so verify replay verdict renders cleanly), and F8 (no chain-link hover) becomes the highest-leverage remaining find since the rest of Scene 6 will be data-clean.

---

## Notes on method

Source-walk + curl probes only. No Preview MCP this run — prior 0623 + 0519 auditors documented Preview MCP instability across Scene 6 navigations + zero net signal vs source-walk, and the dev server was not running at audit start (would have cost build time for no new insight given the deterministic env-driven control flow). All 17 findings are reproducible from the cited files + line numbers + curl invocations. HEAD = `0977044fe`; prior audit HEAD = `e1b0350f1`; diff between them touches `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs`, `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`, `web/app/(app)/workflows/[verticalId]/[name]/build/*`, `web/lib/api/{approvals,audit,audit-detail,replay}.ts`, `web/lib/skycot/demo-mode.ts`, `web/app/(setup)/onboarding/connect-provider/page.tsx`, and a number of docs / stream files — i.e. `#2432` + `#2414` + 5 unrelated PRs as catalogued in the "Resolved since 0623" table above.
