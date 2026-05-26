# UX e2e audit — T20:07 MEL (2026-05-26T10:07Z) delta vs prior

**Prior run**: T14:41 MEL (`docs/skycot/ux-audits/2026-05-26-1441/`), ~5.5h ago.

**Method this run**: 5 parallel audit agents (onboarding-hire-flow / scene-1+2 / scene-4+5 / scene-6+approvals / whole-site) **plus** operator-interject mid-tick: "use the chrome browser and e2e test all the tests." Added: live operator-driven Chrome walk via Claude-in-Chrome MCP at 1440×900 + full playwright e2e suite (38 spec files) — captured at `chrome-live-walk-and-playwright.md` and `playwright/run.log`.

---

## Headline

**Playwright e2e suite at 22 pass / 53 fail (71% failure rate).** This is the load-bearing new datum the prior 5-agent passes did not surface — none of them ran playwright. The previously-deferred test-suite reality is brutal: the hire-flow / onboarding / workflow-builder / vocabulary / dark-mode test clusters are all in regression.

**3 of 6 MISSION claims are demo-blocked at the SPA tier**, despite all 6 being marked 🟢 LIVE in the CLAUDE.md runtime status block. The runtime claim and the SPA reality have drifted apart.

**Distance to v1.0 ship-gate: WIDER than prior tick.** The prior T14:41 run thought the site was "in surprisingly good shape" (whole-site agent, 0 P0). This run's playwright + live-Chrome ground-truth disagrees — the visible-in-demo blockers and the test-suite regressions land 7 new P0s, 13 P1s, 14 P2s in the ledger.

---

## P0 delta

### New P0s this run (7 — all queued to DEMO-RECORDING UNBLOCK)

1. `fix-onboarding-provision-demo-mode-bypass` — wizard step 2 → Continue leaks raw `X-Argo-Principal` gateway error in demo mode. **The investor narrative cannot be walked end-to-end in demo mode right now.** (Discovered via live Chrome walk; the 5 audit agents did not click through to step 2 in demo mode.)
2. `fix-audit-action-detail-demo-mode-fallback` — `/audit/action/{id}` returns 500. (Discovered via live Chrome walk; prior scene-6 audit found related issues but not the demo-mode 500 specifically.)
3. `fix-skills-marketplace-perpetual-loading` — `/skills` shows "0 skills · 0 shown" + perpetual "Loading skills from the marketplace…". (Restated from prior 14:41 F1 + new live observation.)
4. `fix-approvals-inbox-waiting-on-you-fixture-drift` — `/team` says 4 of 5 workers "Waiting on you" while `/approvals` shows empty inbox. (Cross-page data integrity finding new to this pass — only visible when walking both pages back-to-back.)
5. `fix-playwright-hire-flow-regression-cluster` — 8 hire-flow specs failing (shell × 2, step2 × 2, step3 × 3, step4 × 2, onboarding, celebration × 2). (Brand new — no prior agent ran playwright.)
6. `fix-playwright-workflow-builder-regression-cluster` — 13 builder/save/publish/rollback specs failing. (Brand new — no prior agent ran playwright.)
7. `fix-onboarding-welcome-pillar-vs-step5-promise-mismatch` — welcome page promises "voice/SOP upload/comprehension verify" but step 5 delivers a single textarea. (Surfaced by onboarding agent this pass.)

### Carried P0s from prior 14:41 run still standing

- **Skills demo path**: prior F1 (no "competitor pricing scrape" results) + F2 (no install button) — both still apply; covered under the new `fix-skills-marketplace-perpetual-loading` plus existing `fix-skills-detail-install-button-and-flow` (reused).
- **Workflows MCP+CU demo path**: prior F3 (no `customer_research_followup`) + F4 (no `mcp_call`/`computer_use` step kinds) + F5 (Run-now disabled when source≠gateway) — all still apply; covered under existing `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` (reused) + `fix-workflow-detail-run-now-disabled-when-source-not-gateway` (reused).
- **Onboarding first-hire**: prior F1 (name parser → "A") + F2 (worker 404s) + F3 (tenant-resolver inconsistency) + F4 (provider-key 500) + F5 (Genesis not invoked) — all still apply; covered under existing `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` + `fix-onboarding-first-hire-invoke-genesis-plan` (reused).
- **Seed-data gap**: prior F1 (1 worker "Maya" not the 5-card deck) + F2 (no Schedules/Duties tab + no Run-now) — both still apply; covered under existing `fix-demo-seed-cli-for-real-roster` (reused).

### Closed since prior 14:41

- Prior whole-site F02 root-redirect — RESOLVED in source (per whole-site agent this pass)
- Prior whole-site F08 /help-redirect — RESOLVED
- Prior whole-site F17 /support-redirect — RESOLVED
- Prior whole-site F18 /retro + /billing documentation gap — RESOLVED
- Prior whole-site F04 gateway /api/v1/workers alias — fix MERGED in source via PR #2395, but running gateway binary is stale (started 14:34, fix landed 17:00). Operator restart needed.

---

## 6-MISSION-claim SPA-tier rollup (delta vs prior)

| Claim | Prior T14:41 verdict | This run T20:07 verdict | Change |
|---|---|---|---|
| 1 — Teammate not tool | demonstrable | 🟢 demo-walkable (5-card C2 deck via fallback fixture) | UNCHANGED |
| 2 — Delegation is the unit | demonstrable | 🟡 visible but loop broken (approvals empty) | DEGRADED — new approvals-inbox finding |
| 3 — Platform builds workers | demonstrable | 🔴 **demo-blocked** (wizard step 2 fails in demo mode) | DEGRADED — new finding from Chrome walk |
| 4 — Skills shared + learned | demonstrable | 🔴 **demo-blocked** (marketplace empty + perpetual loading) | UNCHANGED (still blocked) |
| 5 — Any-PC execution | no SPA surface | 🟡 unchanged (needs macOS perms) | UNCHANGED |
| 6 — Multi-channel + multi-tenant | partly there | 🔴 **demo-blocked** (audit 500 + approvals empty) | DEGRADED — new audit-drill 500 |

**Net**: prior run thought 4 of 6 claims were demonstrable; this run finds 3 of 6 are actively demo-blocked. Direction of travel is wrong.

---

## Headline assessment for the operator

The good news: every demo-blocker on this list is **shallow** (sample-data drift, demo-mode bypass plumbing, single error-handler routes) rather than load-bearing architecture. The platform-side work that hasn't been done is small.

The bad news: the demo MP4 the operator wants to record **cannot be recorded today**. The onboarding flow — the load-bearing investor narrative — surfaces a raw `X-Argo-Principal` gateway error on step 2. The audit drill-in 500s. The skills marketplace is empty. The approvals inbox shows empty while the team cards say 4 are waiting. Any one of these would derail a recording. All four together make the v1.0 demo MP4 ship-gate slip until the highest-leverage cluster (`fix-onboarding-provision-demo-mode-bypass` + `fix-skills-marketplace-perpetual-loading` + `fix-audit-action-detail-demo-mode-fallback` + `fix-approvals-inbox-waiting-on-you-fixture-drift`) lands.

**Highest-leverage 4-PR sprint to unblock the demo MP4 (rough estimate)**:
1. `fix-onboarding-provision-demo-mode-bypass` — claude — ~2h
2. `fix-approvals-inbox-waiting-on-you-fixture-drift` — claude — ~1.5h
3. `fix-skills-marketplace-perpetual-loading` — claude — ~2h
4. `fix-audit-action-detail-demo-mode-fallback` — claude — ~1h

~6.5h of focused work to make Scenes 3 / 6 / 4 / 6 walkable end-to-end. The playwright cluster failures (hire-flow + workflow-builder, 21 specs combined) are independently load-bearing for shipping quality but do not strictly block the MP4 recording if the live-demo paths are wired.

---

## Inserted into ledger

34 briefs queued under `▼ ▼ ▼ UX-AUDIT-T2007MEL BATCH ▼ ▼ ▼` immediately under the `🚨🚨🚨 DEMO-RECORDING UNBLOCK 🚨🚨🚨` header. Workers pick first-TODO by document order, so the 7 P0s claim first.

Priority breakdown: **7 P0 · 13 P1 · 14 P2.**

28 existing fix-* briefs reused (synthesis agent matched findings to slugs already on disk rather than authoring duplicates) — see `chrome-live-walk-and-playwright.md` + the 5 audit reports for the mapping table.
