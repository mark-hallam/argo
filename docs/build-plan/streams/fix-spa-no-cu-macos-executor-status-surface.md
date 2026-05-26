# fix-spa-no-cu-macos-executor-status-surface

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`. May-touch widened by conductor T03 on 2026-05-26 vs `48dc620b1` after codex pre-claim BLOCK identified 3 missing paths.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-no-cu-macos-executor-status-surface`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` "Claim 5" rollup row [P0 for the claim-5 demo surface].

🔴 **P0 DEMO-BLOCKER for claim 5 — The SPA has zero affordance to confirm computer-use-macos is connected. No "Executor status" pill, no Settings sub-tab, no run-overlay badge. The operator cannot self-verify before recording, and the camera has no UI to land on that says "this worker is driving a real Mac."**

## Root cause

Per `project_v1_strategic_decisions_2026_05_14`, macOS is the v1 demo path for computer-use. Per `feedback_no_theater_in_demo`, the demo must show real backend wiring. Claim 5 is 🟡 LIKELY-LIVE-pending-perms in CLAUDE.md — operator just needs to grant macOS Accessibility + Screen Recording on the `argo` binary.

But there is no SPA surface that:
- Tells the operator whether the binary currently has those permissions
- Shows that `argo-executor-computer-use-macos` is running and connected to the gateway
- Distinguishes a workflow step that will fire computer-use from one that fires MCP
- Renders an "executor: macOS computer-use" pill on the worker / workflow / run-overlay

`grep -rn 'executor\|computer-use\|cu-macos' web/components/skycot/` returns connector-config comments + audit-detail kind labels only — no live runtime status surface.

## DOM evidence

- `web/components/skycot/settings/SettingsTabs.tsx` (D top-tabs) — no "Executors" or "Native runtime" tab
- `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx` — no per-step backend badge
- `web/components/skycot/audit/ProvenanceBlock.tsx` — no "executor" provenance row
- Settings → Connectors lists MCP-style integrations only

## Findings this brief closes

- whole-site-mvp-claims-coverage Claim 5 rollup — "NOT VISIBLE in SPA"; no page/panel surfaces cu-macos connected status
- scene-4-and-5-claim-4-and-5 F11 [P1] — no computer-use indicator on /workflows or workflow detail (subsumed)

## Fix recipe

1. **Add a Settings sub-tab "Executors"** under `web/app/(app)/settings/executors/page.tsx` (or fold into an existing tab). Renders:
   - One row per executor (`computer-use-macos`, `e2b-cloud`, `mcp-pool`).
   - For each: status (`connected` / `not-granted-accessibility` / `not-granted-screen-recording` / `offline`), last heartbeat, runtime version.
   - For cu-macos: a "Grant permissions" CTA that opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` (best-effort URL scheme; falls back to a copy-link with instructions).
2. **Add a GET `/api/v1/executors/status` endpoint** that returns:
   ```json
   {
     "executors": [
       {"id":"cu-macos","status":"connected|degraded|offline","last_heartbeat_at":"...","capabilities":["accessibility","screen-recording","input"],"missing_permissions":[]}
     ]
   }
   ```
3. **In the run-overlay (`build/RunOverlay.tsx`)**, render a small icon next to each step's name that reflects the executor that will fire it (laptop icon for cu-macos, plug icon for MCP, gear for skill). Reuse the icon set from the step-kind metadata once `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` lands.
4. **In the audit-detail Provenance block** (`web/components/skycot/audit/ProvenanceBlock.tsx`), add a sixth row "Executor" if `detail.provenance.executor` is present.
5. **Add a pre-record check** to `docs/skycot/demos/v1-mission-mvp-runbook.md`: "Open Settings → Executors before recording. Confirm `computer-use-macos` reads `connected` with both permissions."

## Acceptance criteria

- [ ] `/settings/executors` page exists and renders one row per executor with live status.
- [ ] The cu-macos row shows current Accessibility + Screen-Recording grant state and a clear path to System Settings if missing.
- [ ] Workflow Run-overlay renders an executor icon per step.
- [ ] Audit-detail Provenance includes an Executor row when present.
- [ ] Demo runbook pre-record checklist mentions the Executors sub-tab.
- [ ] Web smoke test asserts the Executors page renders without errors.

## May-touch

- `web/app/(app)/settings/executors/page.tsx` (NEW)
- `web/components/skycot/settings/SettingsTabs.tsx` (add tab entry — corrected from non-existent `SettingsLayout.tsx`)
- `web/components/skycot/audit/ProvenanceBlock.tsx` (one new row)
- `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx` (executor icon)
- `crates/argo-cli/src/handlers/gateway/routes_executors_status.rs` (NEW handler)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (declare `mod routes_executors_status;`)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` (add `/api/v1/executors/status` dispatch arm — match the existing `routes_*::dispatch(req, store)` pattern)
- `crates/argo-executor-computer-use-macos/src/lib.rs` and a new `crates/argo-executor-computer-use-macos/src/health.rs` (NEW) — author the heartbeat surface; integrate via the existing `executor.rs` / `preflight.rs` modules (no `health.rs` file existed before this brief; brief originally listed it as "verify" but it needs to be authored)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record line)

## Out of scope

- Granting permissions programmatically (operator-only step per CLAUDE.md "operator-only" remaining-launch item (c))
- E2B / Linux / Windows executor parity (DEFERRED-v1.x per project_v1_strategic_decisions_2026_05_14)
- Permission-revoke mid-run UX (separate brief: `fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift`)

## Effort

~200-300 LOC across web + 1 new gateway route + test. ~4-6 hours. This is the visible counterpart to the operator's macOS-permissions step and is required for claim 5 to read as LIVE on camera.
