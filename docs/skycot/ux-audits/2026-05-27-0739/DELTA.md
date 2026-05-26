# UX e2e audit — T0739MEL delta vs T0623MEL (~76m prior)

**Run timestamp**: 2026-05-26T21:39Z (2026-05-27T07:39Z Melbourne) — scheduled-tick `skycot-ux-e2e-audit-4hourly`
**Prior run**: 2026-05-26T20:23Z (~76 min earlier)
**Main SHA at run start**: `0977044fe` (the T0623MEL DELTA + heartbeat commit — same HEAD as when the prior audit pass finished)

## Headline

**4 net-new P0 + 8 P1 + 8 P2 + 4 P3 = 24 briefs queued; 29 findings deduped to existing fix-* briefs (queued or shipped).** The hour was a **stall interval** — **zero PRs merged**; the swarm pushed 4 PRs into IN_REVIEW (#2433 / #2434 / #2435 / earlier #2432 promoted to MERGED last interval but no new MERGED this hour) and claimed 2 more T0623MEL rows (worker-activity-endpoint + duties-card-vocab). The synthesiser uncovered a deeper layer of demo-blocking defects this pass: 4 net-new P0s (vs 1 last hour), reflecting the synthesizer doing tighter cluster-mapping rather than runtime regression. **None of the T0519MEL P0s shipped in the interval. None of the T0415MEL inherited P0 shipped either.**

**No operator unlocks this interval** — the macOS-perms unlock landed mid-T0623MEL interval; `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` is IN_REVIEW (#2434) but not yet merged. Claim 5 still 🟡 LIKELY-LIVE-pending-cold-rerun.

## P0 delta

| | T0623MEL | T0739MEL |
| - | - | - |
| New P0s authored | 1 | **4** |
| P0s closed (MERGED) since prior | 1 (#2432) | **0** |
| P0s still open (cumulative TODO/CLAIMED) | 5 | **9** (3 T0519MEL TODO + 1 T0415MEL TODO + 1 T0623MEL IN_REVIEW + 4 new T0739MEL) |
| P0s in IN_REVIEW (cumulative) | 0 | **1** (#2433 T0623MEL pre-record seed check) |

**T0519MEL P0s — status today**:
- `fix-fetchworker-400-bad-request-no-branch-infinite-retry` — **TODO** (unmoved 140 min — across 2 audit cycles)
- `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — **TODO** (unmoved 140 min; still observable this pass — Scene 4 still unrecordable, see whole-site F1)
- `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak` — **TODO** (unmoved 140 min)

**T0415MEL P0s — status today**:
- `fix-worker-detail-run-now-action-and-endpoint` — **TODO** (worker zombie-released, salvageable WIP at `/tmp/fix-worker-detail-run-now-action-and-endpoint`; brief still ostensibly CLAIMED so no second worker can pick it up)

**T0623MEL P0 — status today**:
- `fix-pre-record-seed-check-script-fail-on-roster-shortfall` — **IN_REVIEW #2433** (good motion)

**T0739MEL P0s — net-new this run** (4):

1. **`fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`** — Demo-mode wizard step 5 calls `buildDemoFirstHireResult` which discards user prompt + redirects to fixed sample worker; the appended roster never grows. Closes 3 findings (onboarding-hire F1 P0 + F2 P0 + F7 P1). Fix recipe: append to demo-roster-store + emit honest banner.
2. **`fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail`** — Worker activity-row hrefs use `ap_NAME_N` IDs that don't resolve in audit-detail table. Demo drill-in lands on placeholder. Either rename hrefs to real action IDs OR extend audit-detail lookup to resolve the legacy format.
3. **`fix-skills-search-substring-misses-runbook-phrase-competitor-pricing-scrape`** — Scene 4 demo phrase mismatches description word-order. Two paths: rewrite description (1-LOC) or switch filter to word-tokenized AND (more durable). Investor narrative breaks on "watch me search for X".
4. **`fix-launch-json-add-demo-live-env-var-and-runbook-preflight`** — `launch.json` missing `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`. #2432 wired the opt-out env var but the dev-launch profile never enables it, so operator running `npm run dev` from launch.json still hits mock-mode SPA. Add the env triplet + a runbook pre-flight checklist for the recording session.

## P1 punch-list — net-new this run (8 briefs)

1. **`fix-onboarding-save-and-exit-button-destructive-rename`** — SAVE & EXIT is a destructive default that triggers `reset()` on hire-submit; rename to EXIT + defer reset so draft survives.
2. **`fix-onboarding-welcome-headline-singular-teammate-vs-workforce`** — Welcome hero copy says "workforce"; rewrite to singular "Hire your first teammate". Drop pillar 2 click handler that deep-links to /genesis mid-wizard.
3. **`fix-workflow-viewer-run-history-mock-offline-fabricated-rows`** — Workflow viewer renders fabricated runs[] from sample fixtures when gateway 404s; replace with empty-state matching /audit pattern. **No theater in demo.**
4. **`fix-workflows-build-vertical-picker-add-marketing-and-compliance`** — `/workflows/build` hardcodes 5 verticals; cohorts has 7; Marketing+Compliance not selectable. Source from gateway with richer fallback.
5. **`fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport`** — Post-#2414 inline Approve/Refuse collides with SLA bar at <1280px viewports; add md: breakpoint + stack actions.
6. **`fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`** — `(app)/layout.tsx:20` default title `"Approvals"` STILL leaks to `/brand`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`, `/brand/components`. Change default to SKYCOT + add per-page metadata. **Demo camera reads the tab bar.**
7. **`fix-gateway-workers-collection-route-alias-404-add-canonical`** — `/api/v1/workers` STILL 404; only `/api/v1/team` works → SPA falls back to sample fixtures (the 5-fictional-roster the demo runbook narrates is fictional, not real). Add alias on gateway.
8. **`fix-team-page-duplicate-h1-and-settings-shell-h1-hierarchy`** — `/team` has 2 H1s; settings shell + sub-page renders 2 H1s; 7 settings sub-pages have no own H1. Enforce single H1 per page.

## P2 cluster — net-new this run (8 briefs)

`fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped`, `fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance`, `fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key`, `fix-onboarding-wizard-footer-trust-strip-fictional-companies-count` (hardcoded "2,847 COMPANIES THIS MONTH" → replace with "Founding 100 · EARLY ACCESS COHORT"), `fix-worker-dynamic-tab-placeholder-strip-brief-id-leak` (6 worker tabs leak "E4-c" engineering brief-ID), `fix-sample-workflows-author-chip-rebalance-and-risk-pill-brand-tokens` (Mariah on 3/8 + HEX literals off-brand), `fix-workflows-templates-title-chain-break-and-genesis-h1-studio-framing`, `fix-retro-ssr-loading-state-no-empty-state-fallback`.

## P3 — 4 briefs

`fix-worker-detail-responsibilities-new-orphan-route-decision` (delete-or-wire decision), `fix-worker-overview-retraindialog-onadjust-noop-callback-decision` (no-op callback), `fix-onboarding-hire-success-copy-help-route-not-support` (point at /help), `fix-onboarding-add-done-page-with-meet-your-teammate-handoff` (celebration moment between submit + /workers/<id>).

## Mid-interval state changes (zero merges, 8 ledger commits)

| Commit | What happened |
| - | - |
| 0977044fe | (prior tick) ux-audit-T0623MEL DELTA + heartbeat |
| 0f556a264 | claim `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` |
| 02f39f263 | claim `fix-pre-record-seed-check-script-fail-on-roster-shortfall` |
| 354de50a0 | `fix-pre-record-seed-check-script-fail-on-roster-shortfall` IN_REVIEW (#2433) |
| d70d120c2 | `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` IN_REVIEW (#2434) |
| 59e91a20a | claim `fix-onboarding-dashboard-hire-button-routes-to-step-5` |
| d561d7c3f | `fix-onboarding-dashboard-hire-button-routes-to-step-5` IN_REVIEW (#2435) |
| b48ac31c3 | claim `fix-worker-activity-endpoint-filter-spa-self-traffic` |
| 141284e7c | claim `fix-duties-card-vocab-responsibilities-to-schedules` |

**Net**: 0 MERGED. 3 PRs queued for review. 2 more rows in CLAIMED. The swarm is alive but no fix has crossed the merge gate this hour.

## 6-MISSION-claim coverage rollup — verdict this run

| # | Claim | T0739 verdict | Direction vs T0623 |
| - | - | - | - |
| 1 | Teammate, not tool | 🟡 LIKELY-LIVE (backend) / 🟡 IMPROVED-IN-SPA | **flat** — `/api/v1/workers` still 404, fetchWorker-400 still TODO; T0623 wins (#2419, #2430) preserved |
| 2 | Delegation is the unit | ❓ NOT-OBSERVABLE-FROM-SPA | flat — audit feed worsened (10/10 self-traffic rows this pass vs 5/5 last pass) |
| 3 | Platform builds workers | 🟡 LIKELY-LIVE | flat — `/verticals` still empty |
| 4 | Skills shared + learned | 🔴 BROKEN-IN-SPA | flat — `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` still TODO at 140 min |
| 5 | Any-PC execution | 🟡 LIKELY-LIVE-pending-cold-rerun | flat — #2434 IN_REVIEW, not yet MERGED |
| 6 | Multi-channel + multi-tenant | 🟡 LIKELY-LIVE | flat |

**Distance to all-🟢 demo-MP4 recordable**: **9 cumulative P0s open** (1 inherited T0415 worker-run-now TODO + 3 T0519MEL TODO + 1 T0623MEL IN_REVIEW + 4 new T0739MEL) + ~22 P1s cumulative + record session. The interval added P0s rather than closing them — the audit converged on tighter clusters than T0623's broad sweep. **Estimate**: 3-5 swarm passes if merge cadence picks back up; this hour's 0/hour cadence is below the trailing-3h average (~5/hour). Bottleneck is **merge throughput**, not brief authoring.

## Top 3 highest-leverage fix-briefs this batch

1. **`fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`** (P0) — closes 3 findings; THE investor-narrative moment of "I'm hiring my first teammate" is hollow as long as the prompt is discarded. Single-file change in the demo-mode resolver. ~30-60 LOC.
2. **`fix-launch-json-add-demo-live-env-var-and-runbook-preflight`** (P0) — operator-safety gate. The demo recording session opens dev tools from launch.json; if the env triplet is wrong, the camera records mock-mode without the operator noticing. ~5 LOC + 1 runbook section. Saves an entire re-take loop.
3. **`fix-gateway-workers-collection-route-alias-404-add-canonical`** (P1) — single gateway alias unblocks Claim 1 in the SPA (Scene 1 narrates a fictional 5-roster because `/api/v1/workers` returns 404). Same root-cause family as the T0519MEL seed-tenant P0 — closing both restores the entire onboarding+workers-page demo loop to real data.
