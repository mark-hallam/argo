# UX e2e audit — T2152MEL (2026-05-26 21:52 AEST · 11:52 UTC) delta vs prior

**Prior run**: [`2026-05-26-2007`](../2026-05-26-2007/) (~1h45m earlier; operator-driven Chrome walk + Playwright e2e).
**This run**: scheduled-tick `skycot-ux-e2e-audit-4hourly`. 5 parallel UX audit agents (~30 min wall) + synthesis agent (~20 min) + ledger insert.

## Pass counts

| | Prior T2007MEL | This run T2152MEL + T2214MEL (subagent extras) | Direction |
|---|---|---|---|
| P0 (root-cause new briefs) | 7 | 11 (7 synthesis + 4 subagent extras) | +4 (mostly newly-uncovered post-#2404 demo-theatre regressions) |
| P1 (root-cause new briefs) | 13 | 12 (9 synthesis + 3 subagent extras) | better |
| P2 (root-cause new briefs) | 14 | 6 (5 synthesis + 1 subagent extra) | much better |
| P3 (root-cause new briefs) | 0 | 1 (0 + 1 subagent extra) | flat |
| Findings deduped to existing briefs | 28 | 43+ | better (audit history is doing the dedupe work) |
| New briefs queued total | 34 | 30 (21 synthesis-manifest + 9 subagent-authored) | -4 |

Lower total + higher dedupe rate is a positive signal: the existing brief library now absorbs more findings than the prior run did, which means the queue is converging on a steady-state punch-list rather than re-discovering the same defects. The 4 new P0s are mostly **post-#2404 demo-mode regressions** (the camera now sees the demo-bypass shortcut faking provider validation, minting synthetic worker IDs that 404, and the synthesizeAuditDetail/placeholderAuditDetail paths fabricating decision chains the camera reads as real).

## Closed P0s since prior run

Four of the prior run's top-of-queue P0s have shipped or are in review:

| Brief | Status | PR |
|---|---|---|
| `fix-onboarding-provision-demo-mode-bypass` | 🟢 MERGED | #2404 |
| `fix-audit-action-detail-demo-mode-fallback` | 🟢 MERGED | #2405 |
| `fix-approvals-inbox-waiting-on-you-fixture-drift` | 🟢 MERGED | #2406 |
| `fix-skills-marketplace-perpetual-loading` | 🟡 IN_REVIEW | #2407 |

That is exactly the "highest-leverage 4-PR sprint to unblock the demo MP4" called out in the prior run's headline assessment. **3 of 4 landed in ~1h45m.**

## Persistent P0s (still standing — high-leverage)

Carried from prior run, still surfaced as P0s this tick:

- `customer_research_followup` MCP+CU combo workflow is still functionally orphaned — Scene 5 beat broken (`fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`, brief exists on disk)
- `/skills/<id>` has no Install button — the runtime path is on `/workers/<id>/skills` (`fix-skills-detail-install-button-and-flow`, brief exists)
- `Run now` disabled for every fixture workflow (`fix-workflow-detail-run-now-disabled-when-source-not-gateway`, brief exists)
- Audit drill-in renders empty decision chain on gateway HTTP self-traces (`fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`, brief exists)
- Approval-row click hard-codes window.location.assign (`fix-approvals-drop-hardcoded-action-id-map`, brief exists)
- Gateway binary stale by 7.5h (`fix-gateway-binary-freshness-auto-restart-or-warn`, brief exists)
- Welcome step promises Genesis Studio (voice/SOP/comprehension); step 5 is single textarea (`fix-onboarding-welcome-pillar-vs-step5-promise-mismatch`, brief exists)

These are NOT being re-authored — they map to existing TODO briefs already queued. They stay P0 in the live punch-list.

## New P0s introduced this run (root-cause, queued under UX-AUDIT-T2152MEL)

7 net-new P0 briefs landed in the ledger:

1. **`fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`** — `THE` post-prior-merge regression. The new demo-bypass shortcut from #2404 mints synthetic worker IDs that then 404 on read-back, and fakes provider-key validation visibly (the camera sees the fakery). Highest leverage P0 of this run.
2. **`fix-auth-demo-mint-route-returns-gateway-unreachable`** — `/api/auth/demo-mint` returns `gateway_unreachable` even when the gateway is healthy on :7400. Blocks signup→wizard handoff.
3. **`fix-signup-no-email-required-button-honest-copy`** — signup demo button promises "no email required" then requires an email. One-line copy + auto-mint fix.
4. **`fix-spa-sessionprovider-disable-in-demo-bypass`** — NextAuth `SessionProvider` spams `ClientFetchError` on every navigation in demo bypass. Console-pollution that the camera will see when devtools opens.
5. **`fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge`** — the 29-connector MCP catalog exists, the builder canvas has no MCP / computer-use step kinds. Scene 5's "build a workflow from MCP+CU" beat cannot be composed.
6. **`fix-approvals-ops-dashboard-inline-approve-reject-buttons`** — `/approvals` ops-dashboard has no inline Approve / Refuse / Undo buttons. Scene 6's central click beat is unscriptable.
7. **`fix-docs-sub-routes-all-404`** — all 10 `/docs/*` sub-routes 404. Customer-pitch chain dies if anyone clicks a doc link on camera.

## 6-MISSION-claim SPA-tier rollup (delta vs prior)

| Claim | Prior T2007MEL | This run T2152MEL | Direction |
|---|---|---|---|
| 1 — Teammate not tool | 🟢 demo-walkable (5-card C2 deck via fallback fixture) | 🟢 strongly visible, end-to-end teammate framing | UNCHANGED — strong |
| 2 — Delegation is the unit | 🟡 visible but loop broken (approvals empty) | 🟡 mostly visible via `DecisionChainCard` components; audit-row delegation chain still empty | IMPROVED (approvals loop closed via #2406) |
| 3 — Platform builds workers | 🔴 **demo-blocked** (wizard step 2 fails in demo mode) | 🟡 wizard works; `/genesis` Studio is the clearest expression of Claim 3 — but Genesis Phase-2 (voice/SOP/comprehension) deferred + new demo-mode-theater P0s introduced | IMPROVED — no longer blocked, residual theater |
| 4 — Skills shared + learned | 🔴 **demo-blocked** (marketplace empty + perpetual loading) | 🟡 marketplace fix in review (#2407); install-button + run-now still P0 | IMPROVED — one shipped, two persistent |
| 5 — Any-PC execution | 🟡 unchanged (needs macOS perms) | 🟡 substrate-chip still all HEADLESS; perms-guidance still operator-only | UNCHANGED |
| 6 — Multi-channel + multi-tenant | 🔴 **demo-blocked** (audit 500 + approvals empty) | 🟡 approvals + audit-drill closed via #2405/#2406; inline-approve buttons + hash-chain hover still missing | IMPROVED — un-blocked, polish gaps |

**Net direction**: prior run found 3 of 6 claims actively demo-blocked; this run finds **0 claims fully demo-blocked**, with 4 of 6 carrying visible-polish gaps that the operator can script around. Direction of travel is positive.

## Distance to v1.0 ship-gate

**Prior assessment**: "demo MP4 cannot be recorded today" — 4 hard blockers.
**This assessment**: **the POC demo IS recordable today** (per the whole-site rollup summary) provided the operator (a) pre-warms routes, (b) does NOT click into `/docs/*` sub-pages on camera, (c) accepts that Genesis is single-textarea not multi-modal yet, and (d) the IN_REVIEW marketplace PR #2407 merges before the recording.

The 7 new P0s queued this tick are **shallow polish + theater fixes**, not architectural. None individually blocks the recording if the operator scripts around them. They DO accumulate into "the demo feels less crisp than it should" — so worth landing before the final take.

**Highest-leverage 4-brief sprint for the next ~6h**:

1. `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect` — claude — ~2h. THE highest-priority because it closes the post-#2404 regression where the demo-mode shortcut produces 404s + fake provider validation.
2. Merge `fix-skills-marketplace-perpetual-loading` (PR #2407 already IN_REVIEW) — operator/conductor — ~10 min.
3. `fix-approvals-ops-dashboard-inline-approve-reject-buttons` — claude — ~1.5h. Scene 6 click beat.
4. `fix-docs-sub-routes-all-404` — claude — ~1h. Either stub or coming-soon badge.

After that 4-brief sprint, the POC demo MP4 is genuinely recordable end-to-end without scripting around defects.

## Inserted into ledger

30 net-new briefs queued under the `🚨🚨🚨 DEMO-RECORDING UNBLOCK 🚨🚨🚨` header in two sibling batches:

1. **`▼ ▼ ▼ UX-AUDIT-T2214MEL BATCH (9 briefs) ▼ ▼ ▼`** — authored directly by the audit subagents during their investigation (timestamp drifts ~22 min from this run's T2152 anchor because the subagents wrote at completion-time, not start-time). The 9 are the highest-leverage post-#2404 demo-theatre regressions + the zombie-pnpm-:3000 demo-readiness footgun.
2. **`▼ ▼ ▼ UX-AUDIT-T2152MEL BATCH (21 briefs) ▼ ▼ ▼`** — synthesised by the conductor's synthesis agent from the 5 audit reports. Lower-leverage polish + ergonomics + new-finding clusters.

Workers pick first-TODO by document order, so the 4 T2214MEL P0s claim FIRST (zombie-pnpm + activity-row-synth-fixture + placeholder-fabrication + demo-seed-workflows), then the 7 T2152MEL P0s.

Priority breakdown (combined): **11 P0 · 12 P1 · 6 P2 · 1 P3.**

43+ findings mapped to existing fix-* briefs on disk (no duplicate authoring). See the synthesis agent's manifest in the parent conductor's transcript for the mapping table.
