# UX e2e audit — T0519MEL delta vs T0415MEL (1h prior)

**Run timestamp**: 2026-05-26T19:19Z (2026-05-27T05:19Z Melbourne) — scheduled-tick `skycot-ux-e2e-audit-4hourly`
**Prior run**: 2026-05-26T18:15Z (~1h earlier)
**Main SHA at run start**: `24695710d`

## Headline

**3 net-new P0s + 6 P1s + 7 P2s + 1 P3 queued.** Zero T0415 P0/P1 briefs MERGED in the 1h interval — all 3 T0415 P0s remain CLAIMED/IN_REVIEW (#2431 IN_REVIEW; other two CLAIMED). The merge cadence is the gate; the audit pass continues to mine fresh defects faster than the worker pool clears them.

## P0 delta

| | T0415MEL | T0519MEL |
| - | - | - |
| New P0s authored | 3 | **3** |
| P0s closed (MERGED) since prior | n/a | **0** |
| P0s still open (cumulative TODO/CLAIMED) | 3 | **6** |
| P0s in IN_REVIEW (cumulative) | 0 | **1** (#2431 skills-try-chips) |

**T0415 P0s — status today**:
- `fix-worker-detail-run-now-action-and-endpoint` — **CLAIMED** (claude, mac-host, 2026-05-26T19:51Z)
- `fix-skills-try-chips-zero-results-rewrite` — **IN_REVIEW** #2431
- `fix-spa-demo-live-mode-route-real-gateway-on-camera` — **CLAIMED** (claude, mac-host, 2026-05-26T19:57Z)

**T0519 P0s — net-new this run**:

1. **`fix-fetchworker-400-bad-request-no-branch-infinite-retry`** — fetchWorker only branches on 404, lets 400 (non-UUID synthetic id) bubble up → React Query retries forever → onboarding completion shows ~15s blank spinner on camera. Single-file change in `web/lib/api/workers.ts`.
2. **`fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`** — post-#2407 boot-seed for `/api/v1/skills` + `/api/v1/verticals` doesn't materialise for the SPA-resolved tenant. Scene 4 (skills marketplace) still unrecordable; verticals wizard step falls back to 5-entry SEED instead of 30+ loaded in the gateway.
3. **`fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`** — duty toggle is `console.warn` stub (flips UI, doesn't persist); edit-toast leaks engineering brief-ID `E4-h` into the customer surface.

## P1 punch-list — net-new this run (6 briefs)

1. **`fix-wizard-verticals-rewrite-scope-and-dead-fallback`** — wizard hits `/verticals` (not in next.config rewrite list); 7-entry FALLBACK constant is dead code.
2. **`fix-sidebar-workers-vs-team-nav-label-consolidation`** — sidebar duplicates Workers + Team for the same surface, URL bar flicker on nav.
3. **`fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed`** — copy promises schedule extraction the API doesn't perform.
4. **`fix-workflow-execute-csrf-token-wiring-verify-before-demo`** — workflow Run-now POST returned 403 `csrf_missing` on direct curl; verify browser path + add cargo test.
5. **`fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`** — live audit feed = 31 rows of `worker=unknown, verb=http.request` SPA self-pings; agent turns invisible.
6. **`fix-billing-subscribe-missing-h1-and-metadata`** — `/billing/subscribe` has zero h1 + leaks default `Approvals · SKYCOT` title.

## P2 cluster — net-new this run (7 briefs)

Polish (`fix-onboarding-wizard-polish-cluster-wrap-scroll-eyebrow`), placeholder-body brief-ID leaks on Memory/Skills/Settings tabs, missing `/api/v1/workers/<id>/activity` endpoint, hardcoded team-page hero KPI multipliers, hero overflow-menu stubs, dead `/workflows/runs|edit` routes, hex-literal risk-pill colours bypassing brand tokens.

## P3 — 1 brief

Skill-card icon rotation index-modulo → semantic name-prefix map.

## 6-MISSION-claim coverage rollup — verdict this run

| # | Claim | T0519 verdict | Direction vs T0415 |
| - | - | - | - |
| 1 | Teammate, not tool | 🟡 LIKELY-LIVE (backend) / 🔴 BROKEN-IN-SPA (worker fetch 400 + onboarding redirect-to-synthetic-id) | **regression-flag** — new P0 fetchWorker-400 surfaces a previously latent failure path |
| 2 | Delegation is the unit | ❓ NOT-OBSERVABLE-FROM-SPA | flat |
| 3 | Platform builds workers | 🟡 LIKELY-LIVE | flat |
| 4 | Skills shared + learned | 🔴 BROKEN-IN-SPA | flat — #2407 merged but boot-seed still not surfacing for SPA tenant (new P0) |
| 5 | Any-PC execution | ❓ NOT-OBSERVABLE-FROM-SPA | flat (computer-use needs operator macOS perms) |
| 6 | Multi-channel + multi-tenant | 🟡 LIKELY-LIVE | flat |

**Distance to all-🟢 demo-MP4 recordable**: 6 cumulative P0s (3 inherited CLAIMED/IN_REVIEW + 3 new TODO) + ~12 P1s (cumulative) + 1 operator-only blocker (macOS perms for claim 5) + record session. **Estimate**: 3-5 swarm passes if the merge cadence picks up; the T0415→T0519 hour added 0 merges to the demo-blocker punch-list, which is the actual risk signal.

## Top 3 highest-leverage fix-briefs this batch

1. **`fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`** (P0) — unblocks two scenes (skills marketplace + verticals picker) in one fix; persistent failure mode across two consecutive audit passes despite #2407 ship.
2. **`fix-fetchworker-400-bad-request-no-branch-infinite-retry`** (P0) — single-line branch fix in `web/lib/api/workers.ts`; eliminates the 15-second blank-spinner camera-glitch immediately after the demo's onboarding completion beat.
3. **`fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`** (P0) — both demo-camera-visible (toast displays engineering ID) and trust-undermining (UI lies about persistence); a worker can land this in <50 LOC.
