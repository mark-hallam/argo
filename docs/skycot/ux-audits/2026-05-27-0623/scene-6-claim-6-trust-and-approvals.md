# Scene 6 (Claim 6 — Multi-channel + multi-tenant + audit trust) — UX audit

**Date**: 2026-05-27 06:23 AEST.
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 6 — `/approvals`, `/audit`, audit-row drill-in, replay, cross-tenant isolation).
**Method**: source-walk + live curl probes against the running gateway at `http://127.0.0.1:7400`. Preview MCP not used this run (the SPA was not running at audit start, and the prior auditor 60min ago documented Preview MCP instability across Scene 6 navigations; restarting the dev server would have wasted the budget). All findings line-pinned against working-tree `HEAD = e1b0350f1`.
**Prior audit**: `2026-05-27-0519/scene-6-claim-6-trust-and-approvals.md` (60 minutes earlier). The 0519 auditor identified the running-config trap that the 0415 auditor had missed; this run is a thin delta against 0519.
**Prior auditor verdict**: 58 / 100 (3 P0s, 2 P1s, 5 P2/P3s, 1 PASS).

---

## TL;DR

**Net change in 60 min: zero.** No commits since 0519 touched any Scene 6 surface (`web/lib/api/audit.ts`, `audit-detail.ts`, `approvals.ts`, `replay.ts`, `.claude/launch.json`, `ActivityRow.tsx`, `SLABar.tsx`, `nav-items.tsx`, `ChannelBadge.tsx`, gateway audit routes). The only shipped commit was `#2431` (skills TRY chips), which is Scene 4 — unrelated. The 17 fix-briefs the prior auditor authored remain TODO/CLAIMED; **none of the Scene 6 P0s are MERGED**.

**All 3 P0s from 0519 carry forward unchanged**:
- **F1 carry**: `.claude/launch.json:9` still emits `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`; every consumer (12+ files in `web/lib/api/*`) still reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. The names still don't match. `/approvals` and `/audit` still hit the empty live gateway.
- **F2 carry**: `audit-detail.ts:103-105` still short-circuits to `placeholderAuditDetail()` in demo mode for any non-canonical action id, fabricating `claude-opus-4-7` / `placehold00000000` / `policyplaceholder` provenance for a row whose true provenance is "the SPA's own HTTP self-traffic." Function-side fabrication on the drill-in the runbook narrates.
- **F3 carry**: `GET /api/v1/audit?range=7d` still returns 35 rows (was 31; +4 in the hour) with the same 10 keys — **no `self_hash`, no `parent_hash`, no `hash`**. `normalizeAuditRow` reads them as undefined. The audit page's centerpiece affordance has no data.

**One net-new minor finding (F16)** this run: cross-tenant isolation negative test on the **list** endpoint returns `0` rows for a bogus tenant header (vs the detail endpoint's clean 404). Not a bug — different shape on the same isolation guarantee — but worth recording for completeness so the next auditor doesn't re-test the same surface.

**Live gateway state (curl-verified at audit time)**:
- `GET /api/v1/approvals` → `{"items":[]}` (still empty, no seed since 0519)
- `GET /api/v1/audit?range=7d` → 35 items, all `worker.name="unknown"`, `verb="http.request"`, action types `audit.detail.viewed` / `audit.replay` (SPA self-traffic; +4 rows from this audit's own curl pings) — still no agent turns
- `GET /api/v1/audit/<id>` (default tenant) → returns real `decision_chain` envelope with 8 step titles
- `GET /api/v1/audit/<id>` (bogus tenant) → `{"error":{"code":"not_found","message":"audit row not found"}}` — isolation **PASS**
- `GET /api/v1/audit?range=7d` (bogus tenant) → 0 rows — isolation **PASS** (silent zero, not 404)
- `POST /api/v1/audit/<id>/replay` → real `verdict:"same"` with matching `expected_self_hash == actual_self_hash` (= `dad16c71…`). Hash recompute path works end-to-end on the live gateway.

**If the operator records the MP4 right now** without intervening, Scene 6 will:
1. Show all-zero KPI tiles on `/approvals` ("0 awaiting", "0 over SLA", "0 today", "—" reviewers).
2. Either show 35 unreadable `worker=unknown, verb=http.request` meta-rows on `/audit`, or show the "Quiet day" empty state if the day-grouping drops them.
3. If the operator clicks ANY of those 35 rows, render a fabricated Provenance card claiming the row was produced by `claude-opus-4-7 · v2026-04-30` using prompt template `generic.placeholder (hash: placehold00000000)`. **This is function-side fabrication the camera sees on click-through.**

**Single fastest unlock** is still the 1-LOC env rename in `.claude/launch.json` from `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. That re-enables the seed-data short-circuit on every API client in one stroke. Per the function-vs-dressing rule, the on-camera Provenance card then renders SAMPLE_AUDIT_DETAILS' canonical 5 rows (Priya/Peta/Mariah/Jamal/Nori) — those have hand-shaped real-shaped provenance (model id is the actual current model, prompt template name is `generic-skill v1.0` not `generic.placeholder`, etc.). The camera-side drill-in then shows real-looking data even if the gateway path stays empty.

Cross-tenant isolation is genuinely live (Claim 6's hardest sub-claim) and recordable on a terminal — but not from inside the SPA, which doesn't expose a tenant switcher.

---

## Resolved since 0519 audit (60 min interval)

| Finding | Status | Notes |
|---|---|---|
| F1 — env var name mismatch | **UNCHANGED** | `.claude/launch.json:9` untouched. Status of the fix brief: `fix-spa-demo-live-mode-route-real-gateway-on-camera` was CLAIMED by claude/mac-host at 2026-05-26T19:57Z per the 0519 DELTA — still CLAIMED, no PR. |
| F2 — placeholder fabrication | **UNCHANGED** | `audit-detail.ts:103-105` + `sample-audit-detail.ts:285-310` untouched. |
| F3 — list endpoint missing hash fields | **UNCHANGED** | Gateway list response still 10-keys, no hash machinery. |
| F4 — SLA bar "2h 60m left" overflow | **UNCHANGED** | `SLABar.tsx:46-50` untouched. |
| F5 — `/` vs `/approvals` dual-surface | **UNCHANGED** | `nav-items.tsx:23` still points "Approvals" at `/`. Also confirmed: `/approvals` is not in `NAV_ITEMS` at all, so the locked E-ops-dashboard is only reachable by direct URL typing today. (See new sub-note in F5 below.) |
| F6 — `Active reviewers` hardcoded `—` | **UNCHANGED** | Line 147-149 still emits `value: "—"`. |
| F7 — DemoBanner sticky over Hero | **UNCHANGED** | |
| F8 — hash-chain prev→this affordance absent | **UNCHANGED** | `ActivityRow.tsx:98-106` still only consumes `activity.hash`. |
| F9 — live audit feed is 31 rows of SPA self-traffic | **UNCHANGED-PLUS-4** | Now 35 rows; +4 SPA self-pings logged in the intervening hour (including this audit's own curls). |
| F10 — `AuditActionNotFound` unreachable | **UNCHANGED** | |
| F11 — Replay UI shows hash-recompute string | **UNCHANGED** | Confirmed real `verdict:"same"` envelope returned by live gateway with matching hashes. |
| F12 — Cross-tenant isolation PASS (detail) | **UNCHANGED** | Re-tested with a 3rd bogus tenant header. Still 404. |
| F13 — Discord absent from ChannelKind | **UNCHANGED** | |
| F14 — Voice/SMS no channel badge in rows | **UNCHANGED** | |
| F15 — Hardcoded April-30 model version | **UNCHANGED** | |

**Zero merges, zero regressions, zero net-new fixes** on Scene 6 surfaces in 60 minutes. The merge pipeline is the gate per the 0519 DELTA's headline — confirmed again here.

---

## Headline blockers (all carry-forward; no new P0s this run)

See prior audit for full reproduction of F1 / F2 / F3 — files + line numbers + curl probes all still match. Two notes worth adding rather than restating:

### F1 carry — additional propagation sites discovered during this run

Beyond the 4 sites the prior auditor catalogued (`approvals.ts:75`, `audit.ts:64`, `audit-detail.ts:63`, `replay.ts:86`), the env-var rename will need to land in **~24 files** for the full demo. A `grep -rn "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" web/lib web/components` returns hits in: `worker-settings.ts`, `worker-overview.ts`, `skill-ir.ts`, `workers.ts`, `workflows.ts`, `audit-export.ts`, `worker-policies.ts`, `worker-skills.ts`, `worker-trust.ts`, `api-keys.ts`, `worker-memory.ts`, `skill-ir-save.ts`, `ghostwriter.ts`, `skill-ir-export.ts`, `orchestration.ts`, `team.ts`, `retros.ts`, `evals.ts`, `cohorts.ts`, `elevated-view.ts`, `workplace-knowledge.ts`, `verticals.ts`, `skills.ts`, `workflow-runs.ts`, `worker-activity.ts`, `use-approval-bundles.ts`, `use-viewer-policy.ts`, `use-redaction-policy.ts`, `RequestElevatedView.tsx`, `AuthProvider.tsx`. **All read `_ARGO_`; all see `undefined` when launched via `.claude/launch.json` (which sets the non-`_ARGO_` variant).**

The fix is still single-LOC (rename in `launch.json`, not the consumers) — but worth knowing the explosion radius of the rename: 25+ surfaces silently fall through to the live-gateway path when the env mismatch fires. This explains why the prior auditor's 0415 read of "everything is mocked" was wrong — virtually nothing is mocked under the launched config.

Two stale references to the OLD env name also linger in scripts that the operator does run by hand:
- `scripts/demo-dry-run.sh:84` → `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (typo path)
- `scripts/capture-product-screenshots.sh:54` → same typo

If the operator's recording rig invokes either of these scripts before the camera rolls, they bake the broken env into the dev server. The screenshot-capture rig has been used as recently as 2026-05-16 per BUILD_STATUS.md (`claude-product-screenshots-and-imagery` MERGED at #1763), so the typo has been propagating silently for ~10 days.

**File**: `.claude/launch.json:9` (primary fix); `scripts/demo-dry-run.sh:84`, `scripts/capture-product-screenshots.sh:54` (collateral).

### F2 carry — confirmed reachability under live launch

Confirmed-by-curl: every action id the live `/api/v1/audit?range=7d` returns is a ULID (`019e65c29f9f7e639c65763a86c7b8bc` and 34 siblings) — **none** match the `act_01J0PRIYA_001` shape that `findSampleAuditDetail()` (`sample-audit-detail.ts:46-225`) keys on. Therefore the placeholder path fires 100% of the time the camera clicks a row on the live `/audit` feed today, not "sometimes" or "edge case." This is the most camera-likely fabrication-on-drill-in surface in the entire MP4 path.

### F3 carry — gateway list endpoint shape unchanged

Confirmed-by-curl: `keys` set on the live response is still `{action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker}`. The detail endpoint returns hash machinery (it's how F11/replay's hash-recompute works) — the list endpoint just doesn't compute or surface it. Same fix scope as 0519 (gateway list route needs to mirror detail).

---

## All findings (delta only — carry-forward findings reference 0519 for full repro)

### `/approvals` (E-ops-dashboard)

#### F4 [P1] CARRY — SLA timer "2h 60m left" overflow

`SLABar.tsx:46-50` unchanged. Same ~3-line fix from 0519 still applies.

#### F5 [P2] CARRY+ELEVATE — `/` vs `/approvals` dual-surface — AND `/approvals` is not in the nav at all

Re-reading `web/components/skycot/nav-items.tsx:22-30` confirms what 0519 noted plus an extra wrinkle: `NAV_ITEMS` contains `{href: "/", label: "Approvals"}` and 6 other entries — **but no `/approvals` entry**. The locked E-ops-dashboard (`/approvals/page.tsx`) is reachable ONLY by typing the URL directly. So the dual-surface confusion is worse than "two screens, same name": one of them is invisible from the navigation chrome.

If the Scene 6 runbook narration is "click Approvals in the sidebar," the camera lands on `/` (the homepage-approvals tree), NOT the locked E-ops-dashboard the design memory specifies. Operator should either (a) repoint the nav item to `/approvals`, or (b) merge the two surfaces, before recording.

**File**: `web/components/skycot/nav-items.tsx:22-30`.

#### F6 [P2] CARRY — `Active reviewers` hardcoded `—`
#### F7 [P3] CARRY — DemoBanner sticky over Hero

### `/audit` (Etherscan-style timeline)

#### F8 [P1] CARRY — Hash-chain prev→this affordance absent

#### F9 [P1] CARRY-PLUS — Live-gateway audit feed now 35 rows of SPA self-traffic (+4 since 0519)

Same root cause; just noting the row count is monotonically growing because every navigation to `/audit/action/<id>` writes another `audit.detail.viewed` row, and every replay writes an `audit.replay` row. The feed gets noisier the more the operator drives the SPA — a particularly nasty failure mode because **pre-recording exercise of the demo flow ITSELF pollutes the feed** the camera will then capture.

Suggested fix carries: `argo demo seed --tenant 01900000-0000-7000-8000-000000000000` per `crates/argo-cli/src/cli_demo_cmds.rs:23` to populate the live audit table with real agent turns before recording. As a defence-in-depth, also worth considering whether `audit.detail.viewed` / `audit.replay` rows should bear a `data_classification=meta` or `verb_kind=trace` flag the SPA list can filter — without that, every future replay-of-a-replay-of-a-replay sediments forever.

#### F10 [P2] CARRY — `AuditActionNotFound` unreachable

### `/audit/action/{id}/replay`

#### F11 [P2] CARRY — Replay UI labels hash recompute as "Original / Replayed output"

Live-curl re-verified: `POST /api/v1/audit/<id>/replay` returns `{verdict:"same", expected_self_hash:"dad16c71…", actual_self_hash:"dad16c71…", kind:"http.request"}`. The hash recompute works correctly; the SPA-side framing in `<ReplayDiff>` is still the disconnect with the runbook narration.

### Cross-tenant isolation

#### F12 [PASS] CARRY — Detail endpoint enforces tenant isolation (404 on bogus header)

#### F16 [P3] NEW — List endpoint returns empty array (not 404) for bogus tenant — different but compatible isolation shape

`GET /api/v1/audit?range=7d` with `x-argo-tenant-id: another_tenant_test` returns `{"items":[]}` (status 200), not a 404. This is correct (a bogus tenant truly has zero rows, not "row not found"), and a buyer asking "prove tenant A can't see tenant B's data" gets a more honest answer on the list endpoint (zero rows visible) than the detail endpoint (404 hides whether the row exists at all in another tenant). But it's a different response shape on what the marketing positions as one guarantee — worth documenting in the demo runbook so the operator narrates it correctly. ("List endpoints return empty arrays for cross-tenant queries; detail endpoints return 404 to avoid existence-leak.")

Not a fix needed; it's a docs / narration finding. Suggested fix: add a 1-paragraph sub-section to `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6 explaining the 404-vs-empty-array isolation shape so the operator narrates it.

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (response is correct; just a doc finding).

### Multi-channel touchpoints

#### F13 [P2] CARRY — Discord absent from ChannelKind unions
#### F14 [P3] CARRY — Voice/SMS no channel badge in `/audit` rows

### General

#### F15 [P3] CARRY — Hardcoded `claude-opus-4-7 · v2026-04-30` in 5 canonical samples

---

## Orientation score: 58 / 100 (UNCHANGED from 0519)

Same surfaces, same defects, same hour later. The score didn't move because nothing shipped against Scene 6 in the 60-minute interval — the merge-pipeline cadence is the limiting constraint (per the 0519 DELTA's headline "0 merges in 1h adds the actual risk signal"). The two newly visible items this run (F5 elevation: `/approvals` not in nav; F16 net-new: list endpoint isolation shape) are both P2/P3 and don't move the needle.

**Single fastest path to a recordable Scene 6** remains: rename the env var in `.claude/launch.json:9`. That re-enables seed-data on all 25+ API clients in one stroke and unblocks F1/F2/F3 simultaneously. The fix brief `fix-spa-demo-live-mode-route-real-gateway-on-camera` was CLAIMED by claude/mac-host at 2026-05-26T19:57Z (~10 hours before this audit) — it remains CLAIMED with no PR opened. Conductor / operator should consider whether the brief needs to be released back to TODO so another worker can pick it up, or whether the original intent (seed live gateway via `argo demo seed`) is the right path and the brief itself needs rewriting.

If the env rename ships first, the **next** standing P0 becomes F2 (placeholder fabrication on non-canonical ids) — relevant because even with seeded sample-data, the gateway path will still resolve ULIDs for any non-canonical row clicked, and the placeholder still fires. The minimal fix is the 0519 option (b) — blank-out provenance fields in `placeholderAuditDetail()` so the card renders its empty-state instead of a fabricated model id.

---

## Notes on method

Source-walk + curl probes only. No Preview MCP this run — the prior auditor 60 min ago documented Preview MCP instability across Scene 6 navigations (the dev server respawned ~4 times during their walk), and re-spawning would have eaten the time budget for no new signal given zero code change in the interval. All 16 findings are reproducible from the cited file + line + curl invocations. Working-tree HEAD = `e1b0350f1`; prior audit HEAD = `7891db5ff`; diff between them touches only `web/lib/api/skills.ts` (#2431 — TRY chips) + `docs/skycot/ux-audits/2026-05-27-0519/*` + `docs/skycot/agents/heartbeats/*` + `ux-audit-T0519MEL` commits. Zero Scene 6 surface changes.
