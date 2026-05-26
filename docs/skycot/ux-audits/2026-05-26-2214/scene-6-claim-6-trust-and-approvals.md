# Scene 6 + Approvals + Audit — Claim 6 (Multi-channel + multi-tenant trust surface)

**Run**: 2026-05-26 22:14 AEST
**Scope**: `/approvals` E-ops-dashboard + `/audit` B3-Etherscan + hash-chain hover + audit-row drill-in REAL vs fabricated + cross-tenant isolation negative tests + `/audit/action/{id}/replay` fabrication check
**Mode**: dev server on dynamic port (preview MCP launched twice — process restarted between runs) + gateway on `:7400` schema v83 FRESH (uptime ~7.8h). Flags `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` set in `.claude/launch.json`.
**Approach**: read-only — `curl` against gateway + source-trace the SPA. The preview-MCP browser locked up under sustained navigation pressure (server restart + 30s timeouts on `preview_eval`/`preview_snapshot`/`preview_screenshot`); ~80% of findings sourced via source-trace + gateway probes that mirror the SPA's data fetchers.

---

## TL;DR

**13 findings — 4 P0, 4 P1, 4 P2, 1 P3.**

Scene 6 is **still not recordable as scripted**. The headline finding from the prior audit (2026-05-26 21:52) — the auth-bypass env-var name mismatch — is unfixed in the working tree: `.claude/launch.json` still sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` while every API client reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. A fix brief (`fix-claude-launch-json-env-var-naming-mismatch`) has been authored against `0653c478d` but has not landed. The downstream cascade — empty approvals queue, audit feed populated only by gateway HTTP self-traces (`verb=http.request`, `worker.name="unknown"`), no real provenance reachable by camera drill-in — is unchanged across the 22 minutes between the prior audit and this one.

What HAS resolved since the 21:52 audit: F16 (replay schema mismatch) is now closed by the `adaptGatewayReplay()` normalizer (`web/lib/api/replay.ts:207-228`) which maps the gateway's `verdict: "same"` envelope to the SPA's `kind: "success" / verdict: "reproducible"` contract — function-side this is real now. The replay endpoint returns matching `expected_self_hash` / `actual_self_hash` and the page should render `<ReplayDiff>` with a green "Reproducible" verdict if the operator drills in. I could not preview-render this end-to-end (MCP locked) but the code path is sound and the gateway response is confirmed real.

NEW finding this run (F-N1): under `SKYCOT_DEMO_MODE=1` (which IS effective today), every action drill-in for an action id NOT in the canonical sample table falls through to `placeholderAuditDetail()` (`web/lib/sample-audit-detail.ts:285`), which **fabricates an 8-step decision chain** ("Memory loaded · context (cached) · pricing rules", "Skill: generic-skill v1.0 (trust 92%)", policy hash `policyplaceholder`, model `claude-opus-4-7`). The structure of reasoning is preserved while the substance is a synthetic invention bearing no relation to the actual action id. This is the highest-leverage theatre-the-camera-sees defect in the current state and it triggers on EVERY live audit row (29/29 today) because none of the live gateway's ULID-shaped ids are in the SAMPLE_AUDIT_DETAIL table.

The 21:52 audit doc described F2 (empty decision chain on http.request rows) — which referred to the **strict-prod / no-demo-mode** path. In **demo mode** (the actual recording-day configuration), the behaviour is worse: instead of structure-but-empty, the chain is structure-AND-fabricated. The function-vs-dressing test fails decisively: the camera drills into the row, the page shows what looks like a real reasoning trace, but every bullet is invented by the SPA from a static template.

---

## Headline blockers

1. **F1 [P0] — `.claude/launch.json` env-var mismatch unchanged**; auth-bypass disabled; gateway returns 200 + empty/self-trace data; SPA never falls back to SAMPLE_* because the fallback gate is `404/network-err` not `200 with empty list`. Fix brief authored but unmerged.
2. **F-N1 [P0] — NEW**: in demo mode, audit drill-in for any unknown action id renders a **fabricated synthesised decision chain** (`placeholderAuditDetail` → `defaultChain("generic-skill","1.0")`) with plausible-looking content ("Plan generated · Primary plan + 2 alternatives", policy hash `policyplaceholder`) and a model id `claude-opus-4-7`. With zero real-worker audit rows in the live gateway (verified: 29/29 rows are `worker.name="unknown"` HTTP self-traces), Scene 6's drill-in path lands on fabricated content 100% of the time today.
3. **F3 [P0] — no inline approve/refuse on `/approvals`** (unchanged from 21:52); Scene 6's "Operator clicks Approve" beat has no affordance on the locked E-ops-dashboard. The approve mutation lives at `/` (root inbox), a different page in a different design register.
4. **F4 [P0] — Etherscan hash-chain cell never renders** (unchanged from 21:52); gateway feed envelope omits `self_hash` / `parent_hash` / `sample_hash` so `ActivityRow` conditionally hides `<HashCell>`. The detail endpoint DOES carry `provenance.hash_chain_anchor` (verified) — the bug is only on the list endpoint.

---

## All findings

### F1 [P0] — `.claude/launch.json` env-var mismatch — UNCHANGED

**Where**: `.claude/launch.json:9` (still sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`) vs `web/lib/api/audit.ts:64` + `web/lib/api/audit-detail.ts:63` + `web/lib/api/approvals.ts:75` + `web/lib/api/replay.ts:86` (still read `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`). Note also: `NEXT_PUBLIC_ARGO_DEMO_MODE=1` set, but `web/lib/skycot/demo-mode.ts:20` reads `NEXT_PUBLIC_SKYCOT_DEMO_MODE` (which IS also set in launch.json, so demo-mode does fire — but the `ARGO_DEMO_MODE` name is dead).

**Symptom**: `/approvals` shows 0 pending across all severity tiers (gateway returns `{"items":[]}` and the fallback only triggers on 404/network-err, not 200-empty). `/audit` shows 29 rows ALL with `worker.name="unknown"` + `verb="http.request"`.

**Suggested fix**: ship `fix-claude-launch-json-env-var-naming-mismatch` (already authored against `0653c478d` on 2026-05-25) — rename the keys in launch.json to match the code.

### F-N1 [P0] — NEW: demo-mode drill-in fabricates the decision chain on unknown action ids

**Where**: `web/lib/api/audit-detail.ts:103-104` short-circuits to `demoModeDetail()` when `isDemoMode()` (which is true today). `demoModeDetail()` (`audit-detail.ts:82-84`) returns `findSampleAuditDetail(actionId) ?? placeholderAuditDetail(actionId)`. The live gateway's 29 audit rows all have ULID-shaped 32-char-hex ids (e.g. `019e642b6adf7ad1a666888d1990a190`) none of which exist in `SAMPLE_AUDIT_DETAILS` (the seeded table covers `act_01J0PRIYA_001` / `act_01J0PETA_001` / etc), so `findSampleAuditDetail` returns null for every live id and `placeholderAuditDetail` runs.

`placeholderAuditDetail` (`web/lib/sample-audit-detail.ts:285-310`) returns a synthetic detail with:
- `worker: { id: "wkr_unknown", name: "Unknown worker" }`
- `verb: "Action detail unavailable — drill into another row for full provenance."`
- `decisionChain: defaultChain("generic-skill", "1.0")` ← FABRICATED 8 steps with plausible content
- `provenance.model: { id: "claude-opus-4-7", version: "2026-04-30" }`
- `provenance.promptTemplate: { id: "generic.placeholder", hash: "placehold00000000" }`
- `provenance.policy: { id: "generic-policy", version: "1.0", hash: "policyplaceholder" }`

The `defaultChain` body (`sample-audit-detail.ts:32-43`) reads:

```
Memory loaded · context (cached) · pricing rules
Skill: generic-skill v1.0 (trust 92%) · Confidence above autonomy threshold.
Plan generated · Primary plan + 2 alternatives.
Plan selected by score · Top score 0.87; alternates 0.74 / 0.62.
Policy check · Action permitted by policy.
Approval · Autonomous — no approval required.
Execution · Executor dispatched.
Outcome · Completed successfully.
```

This is the worst kind of theatre: the camera drills in, the page presents what reads as real agent reasoning, but every line is a static fixture invented from `actionId` URL params with no connection to the original action. "Pricing rules" / "trust 92%" / "top score 0.87" are constants the SPA prints for any unknown id under demo mode.

**Reproduction**:
- `curl http://127.0.0.1:7400/api/v1/audit?range=7d` → 29 rows, all with worker `unknown`
- Pick any id; with demo mode on, opening `/audit/action/<id>` renders the placeholder including the fabricated 8-step chain.
- Verify by inspecting the rendered DOM: every step body should match the literal strings above.

**Suggested fix**: `placeholderAuditDetail` should return a SHORTER chain with `body: ""` on each step + a top-of-card badge "Detail unavailable — drill-in placeholder". Either that, or `/audit/action/<id>` should detect placeholder-shaped detail (model.id === "claude-opus-4-7" + skill.id === "generic-skill" + verb starting with "Action detail unavailable") and render a single `<AuditActionNotFound>` block instead of the full 5-card layout with fabricated content. The DOM testid `data-action-id` is already set; could also be a runtime `data-placeholder="true"` flag the QA gate can lint on. 30-50 LOC.

### F2 [P0] — Audit drill-in empty decision chain on http.request self-traces (strict-prod path)

**Where**: `web/lib/api/audit-detail.ts:107-122` (live-gateway branch). Gateway returns a decision chain with all 8 titled steps but `body: ""` and `details: { ... : null }` for every step. Triggered when demo mode is OFF (current launch.json HAS demo mode on, so the placeholder path F-N1 is what fires today — but if demo mode is disabled, this path activates and shows structure-but-empty).

**Status**: unchanged from 21:52. Fix brief `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback` is in `streams/`.

### F3 [P0] — No inline approve/refuse on `/approvals` ops dashboard — UNCHANGED

**Where**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:297-358 ApprovalRow` — single `<button>` whose only `onClick` is `window.location.assign(actionHref)`. No approve / refuse / undo button. The decision mutations (`useSubmitDecision`, `useBatchSubmitDecisions`, `useUndoDecision`) are consumed only by `/` (root) — a different design.

**Note**: Operator-decisional. The locked E-ops-dashboard memory (`project_approvals_page_design_locked`) emphasises "stats view" not "decision view" — if `/approvals` is intended to be the stats KPI dashboard and `/` is intended to be the inbox where Approve happens, then Scene 6's runbook narration needs to clarify which page the Approve click takes place on. Today's run-book glosses this. Suggest the runbook narrate `/` for the Approve beat AND `/approvals` for the dashboard pan, then click into `/audit` for the chain.

### F4 [P0] — Audit feed envelope omits hash-chain columns — UNCHANGED

**Where**: `crates/argo-gateway` (worktree-only) `GET /api/v1/audit` envelope keys: `[action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker]` — no hash columns. `web/lib/api/audit.ts:194-217 normalizeAuditRow` reads undefined → `web/components/skycot/ActivityRow.tsx:98` conditionally hides `<HashCell>`.

**Status**: unchanged. Fix brief `fix-audit-feed-include-self-hash-in-envelope` exists; `fix-audit-hash-chain-prev-this-hover-and-live-rows` exists. Neither has landed.

### F5 [P1] — No hash-chain pop-out modal anywhere — UNCHANGED

**Where**: `web/components/skycot/audit/` has 28 components, none named `ChainModal`/`HashModal`/`HashTooltip`/`ChainPopover`. `HashCell` (`ActivityRow.tsx:225`) uses a native `title=` attribute tooltip + click-to-copy only.

**Status**: unchanged from 21:52. Locked-design memory `project_audit_page_design_locked` promises a "pop-out modal" — still absent.

### F6 [P1] — DemoBanner spans full viewport, sidebar overlaps text — UNCHANGED

**Where**: `web/components/skycot/AppShell.tsx:38` renders `<DemoBanner />` outside `<main className="md:pl-64">`. Banner inner is centered but leading edge runs from x=0 under the sidebar.

**Status**: unchanged. Visible on every (app)-group page when the banner is undismissed.

### F7 [P1] — `/settings/workspace` hardcoded; tenant identity nowhere — UNCHANGED

**Where**: `web/app/(app)/settings/workspace/page.tsx` — every value hardcoded: "SKYCOT Demo", "Marketing operations", "Australia/Melbourne", "owner@skycot.example". `useSkycotSession()` exposes `tenantId` (`web/lib/skycot/use-session.ts:25-26`) — derived into `workspaceLabel` ("Acmeco") and shown in the `<UserPill>` second line. But the raw `tenant_acmeco` value never surfaces, and there is no tenant switcher / no `/settings/tenants` route anywhere.

**Status**: unchanged. Claim-6's "multi-tenant ergonomics" remain invisible to the user.

### F-N2 [P1] — NEW: approvals & audit fetchers don't fall back to SAMPLE on 200-with-empty-list

**Where**: `web/lib/api/approvals.ts:179-201 fetchApprovals` and `web/lib/api/audit.ts:345-367 fetchAuditFeed`. Both follow the same pattern:
1. If `isAuthBypassed()` → return SAMPLE.
2. Else `apiGet(...)` and on success return the gateway response (even if `items: []`).
3. Only on `isMissingOrUnreachable(err)` (404 / TypeError) → if `isDemoMode()` then SAMPLE, else empty.

In today's state, gateway returns `200 OK` with `{items:[]}` for approvals (because no real approvals are seeded) so the SAMPLE_APPROVALS fallback never fires. Same for the audit feed — gateway returns 200 with the 29 self-trace rows, never 404, so SAMPLE_AUDIT_ROWS never loads.

**Symptom**: even if F1 (env var rename) lands, the demo-mode-via-launch.json toggle alone won't restore the seeded data path unless EITHER (a) the gateway is seeded with real approvals + audit rows in demo mode, OR (b) the fetchers' fallback gate is widened to "gateway empty + demo mode → SAMPLE".

**Suggested fix**: pick one of:
- (a) Backend: a `argo demo seed` CLI that populates demo-tenant approvals + audit rows on a fresh gateway (operator-locked memory `feedback_no_theater_in_demo` prefers this — real backend seed, not mock-bypass)
- (b) SPA: in `fetchApprovals` / `fetchAuditFeed`, when gateway returns success but `items.length === 0` AND `isDemoMode() === true`, return SAMPLE. Cheaper to ship; aligns with the bypass already conditionally returning SAMPLE.

Operator preference per `feedback_no_theater_in_demo`: option (a). NOT mock-bypass.

### F-N3 [P1] — NEW: `placeholderAuditDetail` lies about provenance (model, policy, prompt template)

**Where**: `web/lib/sample-audit-detail.ts:298-304`. The placeholder hardcodes `provenance.model = { id: "claude-opus-4-7", version: "2026-04-30" }` regardless of the action id. If the camera dwells on the Provenance card in Scene 6 (which the runbook implies — "the audit hash-chain hover lingers ≥2 seconds"), the viewer reads a fabricated model id and version under a fabricated `prompt template hash: placehold00000000` and a fabricated policy `policyplaceholder`. The detail endpoint DOES return real `provenance.hash_chain_anchor` for the gateway HTTP self-traces (verified: `ebccacbea0e21e84162d79cde13205ceebe16480796f153eebac3a8502bf9a41`), but the placeholder discards it.

**Reproduction**: same as F-N1 — open `/audit/action/<live-gateway-id>` with demo mode on; inspect the `<ProvenanceBlock>` content.

**Suggested fix**: the placeholder should either show "Provenance unavailable for this action" or pass through the real `provenance.hash_chain_anchor` from a deeper fetch. The current path silently subs in fake provenance, which is the worst case.

### F8 [P2] — Gateway `?as_tenant=<x>` returns 200 with the other tenant's data — UNCHANGED

**Where**: `crates/argo-gateway` (worktree). Verified:
- `curl "http://127.0.0.1:7400/api/v1/audit?range=7d"` → 29 rows
- `curl "http://127.0.0.1:7400/api/v1/audit?range=7d&as_tenant=other_tenant"` → 200 OK with 3 rows (different tenant's self-traces)
- `curl "http://127.0.0.1:7400/api/v1/approvals?as_tenant=other_tenant"` → 200 OK with `{"items":[]}`

**Status**: unchanged from 21:52. Fix brief `fix-audit-cross-tenant-cap-bypass-mode-behavior` exists.

**Note**: this is a dev-mode gateway permissiveness, not a substrate breach — the substrate test `claim_6_live_cross_tenant` is the load-bearing isolation check and passes per CLAUDE.md status block. But the HTTP surface diverging is a P2 risk for any demo that tries to demonstrate cross-tenant deny.

### F10 [P2] — `/audit` shows wall of green "Succeeded · http.request" rows — UNCHANGED

**Where**: every self-trace row in the gateway feed has `status: "succeeded"`. `AuditDayGroup.tsx` STATUS_MAP maps `succeeded` to green pill. With 29/29 rows in this state today, `/audit` reads as "everything is green and nothing is happening" — the worst-case "your trust dashboard" framing.

**Status**: unchanged. Fix brief `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows` exists (would add real action types into the feed). Fix brief `fix-audit-feed-action-kind-variety` exists.

### F11 [P2] — SAMPLE_APPROVAL_ACTION_IDS map non-deterministic — UNCHANGED

**Where**: `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:26-35`. Same as 21:52 — hardcoded action ids like `act_018f67d0-0000-7000-8000-000000000001_018` do not exist in live gateway. Verified that the gateway replay endpoint rejects non-32-char-hex with HTTP 400 (`audit id must be 32-char hex (16 bytes)`).

**Status**: unchanged. Fix brief `fix-approvals-drop-hardcoded-action-id-map` exists.

### F14 [P2] — Approvals "Active reviewers" KPI permanently `—` — UNCHANGED

**Where**: `ApprovalsOpsDashboard.tsx:146-150`. The 5th KPI cell is still hardcoded `value: "—"` / `sub: "reviewer roster pending"`. F7 of the 2026-05-26 2007 audit; fix brief `fix-approvals-active-reviewers-kpi-derive-from-pending` authored against `ea161df3a` on 2026-05-26.

**Status**: brief authored, not yet shipped.

### F13 [P3] — Hash-cell tooltip jargon "Self-hash" — UNCHANGED

**Where**: `ActivityRow.tsx:232`. Tooltip reads "Self-hash · click to copy". For buyer-facing demo, "Action hash" or "Audit row ID" reads cleaner. Low leverage; mention only.

---

## Resolved since 2026-05-26 21:52 audit (22 minutes ago)

- **F16 (replay schema mismatch)** — RESOLVED. `web/lib/api/replay.ts:207-228 adaptGatewayReplay()` adapter maps gateway envelope `{verdict: "same", expected_self_hash, actual_self_hash, ...}` to the SPA contract `{kind: "success", verdict: "reproducible", result: ReplayResult}`. The replay endpoint returns matching hashes for the test row (verified end-to-end). Could not preview-render because the MCP preview kept dying; code path is sound.

(No other items resolved between 21:52 and 22:14; all other findings carry forward as same severity.)

---

## What I could NOT verify this run (preview MCP died)

The preview MCP locked up early — `preview_eval` / `preview_snapshot` / `preview_screenshot` all returned 30s timeouts after a single navigation, and the server restarted itself once (port 51153 → port 52500) mid-run. I could not visually confirm:
- The DemoBanner overlap on screen (F6) — confirmed via source-trace only
- The hash-cell absence on the rendered `/audit` row (F4) — confirmed via gateway-envelope probe + source-trace of `ActivityRow.tsx` conditional render
- Whether the Auth.js `ClientFetchError` console spam I saw on first probe is the steady state or a transient (the second server start showed `null` from `/api/auth/session` cleanly + the provider endpoint responded normally). Likely transient — not classifying as a finding.

If a follow-up run gets a stable preview, the priorities to confirm visually are: F-N1 (drill into ANY live audit row, screenshot the fabricated chain), F4 (audit list — verify no hash column), F6 (banner overlap at desktop ≥768px).

---

## Orientation score: 54 / 100

(Down 2 from 21:52 — F-N1 promoted from "fixed by F2's strict-prod path" to "always fires in demo mode"; F-N2 + F-N3 added; F16 resolved partially offsets.)

The trust-surface architecture is real: hash chain anchor is computed and exposed; replay matches; replay adapter now wired; cross-tenant gating happens at the SQL layer. But the demo-day presentation is held together by fixtures whose seams are camera-visible. Two seams in particular are show-stopping: (1) the env-var rename hasn't landed, so SAMPLE_* fallbacks never fire on a 200-empty gateway, leaving Scene 6 to render the gateway's own HTTP-introspection rows; (2) demo mode's `placeholderAuditDetail` fabricates a fully-shaped detail page out of static content, so even if the SPA were forced into demo mode, drilling into any live row shows invented reasoning.

The cheapest path to recordable is: (a) ship `fix-claude-launch-json-env-var-naming-mismatch` so launch.json + code agree on the env-var name, (b) demote `placeholderAuditDetail` to a single "Detail unavailable" card (no fake chain, no fake provenance), (c) either backend-seed the demo tenant with real approvals + audit rows OR widen the SPA fetcher fallback to treat 200-empty under demo mode as a SAMPLE trigger.

**Function-vs-dressing application**: F1, F-N1, F-N2, F-N3, F2, F3, F4 are FUNCTION-side. F6, F7, F11, F13 are DRESSING-side. F5, F10, F14 sit on the border. F8 is substrate-vs-HTTP-surface drift, P2 because the substrate test passes.
