# UX e2e audit — T0119MEL delta vs prior

**Run**: 2026-05-27T01:19 Melbourne (2026-05-26T15:19Z)
**Prior**: 2026-05-27T00:15 Melbourne (2026-05-26T14:15Z) — 64 min earlier
**5 audit reports**: onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals, whole-site MVP coverage
**Synthesis**: 16 net-new root-cause clusters authored (out of ~62 findings, 18 P0-graded); ~47 findings deduped against the 268 fix-* briefs already on origin/main
**Ledger insertion**: clean — header `▼ ▼ ▼ UX-AUDIT-T0119MEL BATCH (16 briefs)` inserted at top of DEMO-RECORDING UNBLOCK section (above the T0015MEL batch), **1 P0 + 7 P1 + 6 P2 + 2 P3 rows queued**

## P0 punch-list — this run

This run identified **1 net-new P0** (single root cause that cascades into multiple symptoms) plus reaffirmed the persistent P0 cluster from prior runs.

### Net-new P0 (the headline)

**fix-spa-client-use-same-origin-gateway-rewrite** — `web/lib/api/client.ts:80` returns an absolute gateway URL (`http://127.0.0.1:7400`) in browser context, causing every client-side fetch to fail CORS. Cascades into:

- `/audit` stuck in `audit-loading` skeleton forever (Scene 6 cannot drill in)
- `/workers` deck falling back to `SAMPLE_TEAM` with the visible mock-offline pill
- `/skills` showing `0 skills` SSR flash before fallback
- `/approvals` inbox shows "Loading approvals…" indefinitely

**Single 1-line fix** (return empty string in browser context → Next.js `/api/v1/*` rewrite handles same-origin) lifts the whole-site orientation score from 72 → ~85.

### Persistent P0 cluster (unchanged from T0015 — most now IN_REVIEW)

| # | brief | state | demo-camera-sees? |
| - | - | - | - |
| 1 | fix-genesis-orphan-discoverability-and-provider-key-503 | IN_REVIEW #2420 | YES — Genesis Studio investor moment |
| 2 | fix-hire-demo-mode-stub-uuid-worker-roster-invisibility | IN_REVIEW #2419 | YES — Scene 1 hire commit |
| 3 | fix-workflows-dark-mode-card-white-on-white | MERGED #2421 | YES (if dark-mode recorded) — **closed since T0015** |
| 4 | fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect | IN_REVIEW #2411 | YES — Scene 1 connect-provider |
| 5 | fix-signup-no-email-required-button-honest-copy | IN_REVIEW #2412 | YES — Scene 0 signup beat |
| 6 | fix-spa-sessionprovider-disable-in-demo-bypass | IN_REVIEW #2413 | YES — console error noise |
| 7 | fix-approvals-ops-dashboard-inline-approve-reject-buttons | IN_REVIEW #2414 | YES — Scene 6 approval beat |
| 8 | fix-gateway-mod-rs-loc-overage-extract-store-helpers | IN_REVIEW #2422 | NO (but blocks PR #2407, a 4-of-4 demo-MP4 P0) |
| 9 | fix-onboarding-pick-vertical-demo-mode-honest-install-relabel | IN_REVIEW #2423 | YES — Scene 1 pick-vertical |
| 10 | fix-in-product-hire-ctas-rewire-to-onboarding-canonical | IN_REVIEW #2424 | indirect — every in-app Hire CTA |
| 11 | fix-runbook-scene-1-2-align-with-mariah-actual-duties | IN_REVIEW #2425 | doc-only |
| 12 | fix-workflow-execute-demo-principal-capability-grant | CLAIMED | YES — Scene 4/5 "Run now" |
| 13 | fix-sample-workflows-add-peta-as-editor | IN_REVIEW #2426 | YES — Scene 4 card variety |
| 14 | fix-demo-banner-copy-branch-on-data-source-or-suppress | IN_REVIEW #2427 | YES — every (app)-group page |
| 15 | **NEW THIS RUN: fix-spa-client-use-same-origin-gateway-rewrite** | TODO | YES — every gateway-backed surface |

## Delta vs prior (T0015MEL)

### Closed since prior run (64 min window)

- **PR #2421 MERGED** (fix-workflows-dark-mode-card-white-on-white) — first P0-closing merge in the audit history of this batch. Closes the dark-mode Scene 4/5 demo-camera risk.
- **+5 new PRs opened** as IN_REVIEW: #2422 (gateway-mod-rs LOC), #2423 (pick-vertical relabel), #2424 (hire-ctas rewire), #2425 (runbook align), #2426 (Peta as editor), #2427 (demo banner).
- **+1 brief CLAIMED**: fix-workflow-execute-demo-principal-capability-grant.

**Net**: 1 P0 closed (merged), 1 P0 newly identified (spa-client cascade), 6 P0s converted to IN_REVIEW. The conversion rate is the leading indicator — the swarm is actively pumping the backlog through the review pipeline.

### Newly opened P0s (NET-NEW — not covered by prior briefs)

**1 net-new P0** (vs T0015's 0):

- **fix-spa-client-use-same-origin-gateway-rewrite** — the whole-site agent surfaced this at F01 + F02 + F03 cluster. None of the 268 existing fix-* briefs name `gatewayBase()` or the absolute-URL bug directly. Confirmed novel.

### Newly opened P1s (NET-NEW — not covered by prior briefs)

7 P1 briefs authored this run:

- **fix-onboarding-first-hire-populate-persona-bio-and-voice-preamble** — wizard freeform text goes to charter only, not persona.bio → worker-profile About section empty
- **fix-onboarding-install-vertical-receipt-memory-and-gates** — install-vertical response only confirms skills, not memory schema + approval gates (operator's promise mismatched)
- **fix-worker-activity-tab-poll-for-streaming** — activity tab is static, no `refetchInterval` → Scene 2 "watch the run land" beat shows stale page
- **fix-audit-filter-key-action-type-vs-kind-vocab-alignment** — gateway accepts `?action_type=` but SPA + runbook use `?kind=` (silent-ignore), and existing fix-audit-list-kind-query-param-ignored covers a sibling concern but not the synonym-alias direction
- **fix-appshell-user-menu-radix-popover-hydration** — Radix popover SSR mismatch visible in DevTools on every (app)-group page
- **fix-settings-stub-routes-branding-integrations-danger** — typed URLs to 3 sub-paths 404 (no redirect to canonical workspace#anchor)
- **fix-sidebar-live-vs-demo-mode-stamp** — no visible "Live mode" / "Demo mode" stamp; operator can't tell at a glance which the dev server is in pre-record

### Newly opened P2s + P3s (NET-NEW)

6 P2 + 2 P3 net-new briefs covering: welcome-step eyebrow pattern (P2), pillars-as-bridge instead of repitch (P2), gateway-error envelope shape consistency (P2), `/changelog` this-week date rail (P2), marketing pages mobile breakpoints (P2), `/team` empty-state Slack promise (P2), connect-provider $0.001 claim honesty (P3), changelog pagination (P3).

## 6-MISSION-claim coverage delta (this run vs T0015)

| claim | T0015 verdict | T0119 verdict | shift |
| - | - | - | - |
| 1 — Teammate, not tool | 🟡 SPA mocks; hire-flow mints invisible stub | 🟡 same; vocab drift in 2 marketing steps + 3 non-canonical names on /for/[vertical] | stable |
| 2 — Delegation is the unit | 🟡 DelegationChainCard renders fixture handoffs | 🟡 unchanged; loading-approvals never resolves on `/` (gateway URL bug) | stable |
| 3 — Platform builds workers | 🔴 Genesis unreachable + 503s | 🔴 unchanged — #2420 still IN_REVIEW | stable |
| 4 — Skills shared + learned | 🟡 marketplace UI live, 0 skills in gateway | 🔴 **downgraded** — skill install button still missing, SSR flash shows 0 skills | -1 (visibility) |
| 5 — Any-PC execution | NOT IN SPA SCOPE | NOT IN SPA SCOPE — same | stable |
| 6 — Multi-channel + multi-tenant + trust | 🟡 ChannelStripCard renders; audit drill-in returns "Unknown worker" | 🟡 audit page now **stuck on loading skeleton** (gateway URL bug) — worse user experience but same root | stable |

**Net shift**: Claim 4 was downgraded from 🟡 → 🔴 by the whole-site agent (no install button + 0-skills flash is more visible than prior audit captured). One MERGE (#2421 dark-mode) closed a Claim-2-visible defect but didn't flip claim color. Single biggest unlock would be the new spa-client P0 — fixing it cascades-resolves Claims 2 + 4 + 6 visibility in one PR.

Orientation score: **72** (whole-site agent's verdict) vs **71** at T0015 — fractionally improved by dark-mode merge.

## Distance to v1.0 ship-gate

**Ship-recordable today: NOT YET. The bottleneck remains PR throughput, with one new P0 (spa-client) now added.**

T2214 → T2309 → T0015 → T0119 trajectory:

- **T2214**: 3 operator-side gates + several P0 demo-blocker briefs to author
- **T2309**: 3 operator-side gates + 3 NEW P0 demo-blocker briefs (Genesis, stub-uuid, dark-mode) + 13 in-flight
- **T0015**: 3 operator-side gates + 0 new P0s + IN_REVIEW pipeline at 9 PRs
- **T0119**: 3 operator-side gates + **1 new P0** (spa-client) + IN_REVIEW pipeline at **14 PRs** + 1 MERGED (#2421)

The conversion-to-PR rate is high (+5 IN_REVIEW in 64 min) and merge rate finally non-zero (+1 merged). The new P0 is a 1-line fix that cascades into 3 claim-visible surfaces — high-leverage.

The 3 operator-side gates remain unchanged:

1. **Kill orphan pnpm dev + relaunch with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`** — 30-second op
2. **Grant macOS Accessibility + Screen Recording perms on argo binary** — closes claim-5 runtime gap
3. **Sweep the 14 IN_REVIEW PRs to merged** — `gh pr list --search "is:open is:pr in:title fix-"`, run pre-merge-gate on each, admin-merge per the T2309/T0015 operator authorisations

## Top 3 highest-leverage fix-briefs (this run)

1. **fix-spa-client-use-same-origin-gateway-rewrite** [P0] — single 1-line fix to `web/lib/api/client.ts:80` (`return ''` in browser context). Cascades into `/audit` + `/workers` + `/skills` + `/approvals` all becoming live-data-functional. Most leveraged finding in the audit history.
2. **fix-worker-activity-tab-poll-for-streaming** [P1] — Scene 2 of the v1.0 demo runbook requires the activity tab to show a new audit row landing live. Currently static. 1-line `refetchInterval: 3000` addition.
3. **fix-onboarding-first-hire-populate-persona-bio-and-voice-preamble** [P1] — the operator's freeform "describe what they should do" text is the most emotionally-loaded UX moment of the hire wizard. Today it disappears into charter and never shows up on the worker profile. Routing the same text into `persona.bio` makes the wizard feel like onboarding a colleague (claim-1).
