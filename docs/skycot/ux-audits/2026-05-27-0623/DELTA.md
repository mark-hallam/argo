# UX e2e audit — T0623MEL delta vs T0519MEL (1h prior)

**Run timestamp**: 2026-05-26T20:23Z (2026-05-27T06:23Z Melbourne) — scheduled-tick `skycot-ux-e2e-audit-4hourly`
**Prior run**: 2026-05-26T19:19Z (~1h earlier)
**Main SHA at run start**: `d2aeaf3de`

## Headline

**1 net-new P0 + 5 P1s + 5 P2s + 1 P3 queued.** The hour was a **drain win** — 9 PRs merged into main (the largest interval-merge count in the last 5 audit passes), including the big T0415MEL P0 `spa-demo-live-mode-route-real-gateway-on-camera` (#2432). But **0 of 3 T0519MEL P0s shipped**; they remain TODO behind the drain batch. The audit pass surfaced fewer net-new defects this run (-6 vs T0519MEL's 18) because most freshly-observable surfaces had already been mined; the new P0 is an **operator-safety process gate**, not a runtime defect.

**Operator unlock landed mid-interval**: macOS perms granted for Claim 5 (commit `4585055d3`) — `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` brief authored. Once that rerun passes, Claim 5 flips 🟡→🟢 and item (c) on the v1.0 ship-gate list closes.

## P0 delta

| | T0519MEL | T0623MEL |
| - | - | - |
| New P0s authored | 3 | **1** |
| P0s closed (MERGED) since prior | 0 | **1** (`fix-spa-demo-live-mode-route-real-gateway-on-camera` #2432 — a T0415MEL P0) |
| P0s still open (cumulative TODO/CLAIMED) | 6 | **5** (3 T0519MEL still TODO + 1 T0415MEL still TODO + 1 new T0623MEL) |
| P0s in IN_REVIEW (cumulative) | 1 (#2431) | **0** (#2431 + #2432 both MERGED) |

**T0519MEL P0s — status today**:
- `fix-fetchworker-400-bad-request-no-branch-infinite-retry` — **TODO** (unmoved 1h)
- `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — **TODO** (unmoved 1h; still observable this pass — scene-4-5 F6, whole-site F1+F8)
- `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak` — **TODO** (unmoved 1h; still observable this pass — scene-1-2 F4+F5)

**T0415MEL P0s — status today**:
- `fix-worker-detail-run-now-action-and-endpoint` — **TODO** (worker zombie-released, salvageable WIP at `/tmp/fix-worker-detail-run-now-action-and-endpoint`)
- `fix-skills-try-chips-zero-results-rewrite` — **MERGED #2431** (prior pass)
- `fix-spa-demo-live-mode-route-real-gateway-on-camera` — ✅ **MERGED #2432** (this interval)

**T0623MEL P0 — net-new this run** (1):

1. **`fix-pre-record-seed-check-script-fail-on-roster-shortfall`** — operator-safety process gate, not a code bug. `scripts/demo-dry-run.sh` returns PASS even when `/api/v1/team` ships only 1 worker (Maya) instead of the 5 canonical personas (Mariah/Peta/Priya/Jamal/Nori). The operator can finish dry-run, roll camera, and discover the missing roster mid-take. ~40-60 LOC across script + roster file + smoke test. The other P0-category runtime bugs in this audit already had existing briefs (the synthesizer mapped 10+ findings to T0415/T0519 briefs); the only net-new P0 is this dry-run hardening.

## P1 punch-list — net-new this run (5 briefs)

1. **`fix-onboarding-dashboard-hire-button-routes-to-step-5`** — Dashboard `Hire teammate` button routes to `/onboarding/first-hire` (step 5) instead of `/onboarding/welcome`; wizard state empty + silent-failing on submit.
2. **`fix-worker-activity-endpoint-filter-spa-self-traffic`** — Per-worker `/api/v1/workers/{id}/activity` returns `worker.kpi_rolled` + `http.request` rows; sibling to the existing global audit-self-traffic filter brief.
3. **`fix-duties-card-vocab-responsibilities-to-schedules`** — DutiesCard CardTitle says `Responsibilities`; production-UI rename missing from the existing playwright-sweep brief. Operator vocab lock violation.
4. **`fix-workflow-run-history-fabricated-rows-in-mock-offline`** — Workflow viewer renders fabricated `runs[]` from sample fixtures when `source=mock-offline`; replace with empty-state component. **No theater in demo** per operator.
5. **`fix-settings-sub-pages-add-missing-metadata`** — 7 of 11 `/settings/*` sub-pages have no metadata export → tab title leaks `Settings · SKYCOT` instead of the page name. 7 × 4-LOC additions.

## P2 cluster — net-new this run (5 briefs)

`fix-cohorts-ssr-empty-preview-flash-before-hydrate`, `fix-retro-page-ssr-loading-no-empty-state`, `fix-onboarding-pick-vertical-demo-mode-chip-clutter` (DEMO chip on every vertical card), `fix-workflow-card-author-chip-mariah-over-indexed` (8 workflows distribute Mariah×3), `fix-workflows-page-try-chips-zero-results-rewrite` (mirror of #2431 Skills fix — 3 of 4 chips return 0 results).

## P3 — 1 brief

`fix-audit-isolation-shape-docs-list-vs-detail` — list endpoint returns `200 []`, detail returns `404`; both correct but shape differs. Doc-only.

## Mid-interval merge wins (9 PRs)

| PR | What shipped |
| - | - |
| #2432 | demo-live mode — audit/replay/approve route through real gateway (T0415MEL P0) |
| #2430 | persona.bio + voice_preamble populated from operator brief |
| #2420 | Genesis nav + clickable pillar + provider-key alias (investor demo) |
| #2419 | Real-shaped UUID on hire + session-local roster |
| #2416 | Stub 10 sub-routes under /docs (T0519MEL P3 — docs-sub-routes-404) |
| #2415 | Wire MCP connector catalog + cu_action into builder palette |
| #2414 | Inline Approve / Refuse / Undo on `/approvals` ops dashboard |
| #2411 | Drop demo-mode fake provider-key validation + synthetic worker redirect |
| #2410 | demo-mint proxy disambiguates gateway_unreachable vs mint-route failure |

**Plus operator-side**: macOS perms granted (Accessibility + Screen Recording) — `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` brief authored at `4585055d3`.

## 6-MISSION-claim coverage rollup — verdict this run

| # | Claim | T0623 verdict | Direction vs T0519 |
| - | - | - | - |
| 1 | Teammate, not tool | 🟡 LIKELY-LIVE (backend) / 🟡 IMPROVED-IN-SPA | **forward** — #2411 + #2419 + #2430 (real-shaped UUID + session-local roster + persona.bio populated) close the worst onboarding theater; fetchWorker-400 + duty-toggle stub remain |
| 2 | Delegation is the unit | ❓ NOT-OBSERVABLE-FROM-SPA | flat |
| 3 | Platform builds workers | 🟡 LIKELY-LIVE | flat |
| 4 | Skills shared + learned | 🔴 BROKEN-IN-SPA | flat — `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` still TODO; scene-4 still unrecordable |
| 5 | Any-PC execution | 🟡 LIKELY-LIVE-pending-cold-rerun | **forward** — macOS perms granted; cold-rerun brief authored; flips to 🟢 on rerun PASS |
| 6 | Multi-channel + multi-tenant | 🟡 LIKELY-LIVE | flat |

**Distance to all-🟢 demo-MP4 recordable**: 5 cumulative P0s open (1 inherited T0415 worker-run-now TODO + 3 T0519MEL TODO + 1 new T0623MEL) + ~14 P1s cumulative + record session. Item (c) macOS perms — operator-actioned this hour. **Estimate**: 2-4 swarm passes if merge cadence holds at 9/hour; the trend reversed (~80 findings → ~50 findings, 17 net-new briefs → 12 net-new briefs) — the punch-list is starting to converge.

## Top 3 highest-leverage fix-briefs this batch

1. **`fix-pre-record-seed-check-script-fail-on-roster-shortfall`** (P0) — operator-safety gate. A roster-mismatched dry-run that returns PASS is the single most expensive failure mode for the demo recording. ~30 min agent work, saves an entire re-take loop.
2. **`fix-onboarding-dashboard-hire-button-routes-to-step-5`** (P1) — silent-failing CTA on the post-onboarding dashboard. Investor narrative breaks on the second hire attempt. Single-file change in `web/app/(app)/page.tsx:460`.
3. **`fix-workflow-run-history-fabricated-rows-in-mock-offline`** (P1) — demo-camera-visible theater (fabricated workflow runs from sample fixtures). Operator's "NO THEATER IN DEMO" rule violation; ~40 LOC viewer-side conditional.
