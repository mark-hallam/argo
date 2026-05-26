# UX e2e audit — T0415MEL delta vs prior

**Run**: 2026-05-27T04:15 Melbourne (2026-05-26T18:15Z)
**Prior**: 2026-05-27T01:19 Melbourne (2026-05-26T15:19Z) — 176 min earlier (~2h56m)
**5 audit reports**: onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals, whole-site MVP coverage
**Synthesis**: 18 net-new root-cause clusters authored (out of ~76 findings); ~47 findings deduped against the 284 fix-* briefs already on origin/main
**Ledger insertion**: clean — header `▼ ▼ ▼ UX-AUDIT-T0415MEL BATCH (18 briefs)` inserted at top of DEMO-RECORDING UNBLOCK section at line 243 (above the T0119MEL batch), **3 P0 + 6 P1 + 7 P2 + 2 P3 rows queued**

## Headline

- **4 P0 PRs merged in the inter-run window** (#2407 + #2422 + #2424 + #2429 — the 4-of-4 demo-MP4 P0 set that locked T18-16) closed prior-run blockers on skills seed + gateway-mod-rs LOC + hire-CTAs + the same-origin SPA-client fix
- **3 net-new P0s identified this run** (none overlap with prior run's net-new P0)
- **Orientation score dropped 72 → 62**. This is a **deeper-cut audit, not a regression**: with the same-origin fix landed (#2429), defects that CORS was previously masking are now visible (workers collection endpoint mismatch, audit-feed zero real-turn rows, skills empty seed). Fix-CORS revealed the next layer.
- **Distance to v1.0 ship-gate**: still gated on the 3 net-new P0s + ~6 inherited P0/P1 work-in-progress + the demo seed CLI not having been run with the persona roster + Run-now backend wiring.

## P0 punch-list — this run (3 net-new)

| # | brief | demo-camera-sees? | one-liner |
| - | - | - | - |
| 1 | **fix-worker-detail-run-now-action-and-endpoint** | YES — Scene 2 trigger beat | No "Run now" button on `/workers/{id}` (workflows have one, workers don't); Scene 2 has nothing to click. Adds button + `POST /api/v1/workers/<id>/run`. |
| 2 | **fix-skills-try-chips-zero-results-rewrite** | YES — Scene 4 fallback path | All 6 TRY chips on `/skills` return 0 results (chip text "competitor pricing scrape" vs seed name "Web scrape"). Chip vocab mismatch with seed corpus. |
| 3 | **fix-spa-demo-live-mode-route-real-gateway-on-camera** | YES — Scene 6 trust beat | demo+bypass mode short-circuits audit/replay/approve calls in the mock layer; the runbook narration "everything you see is real backend data" is theatre. Add NEXT_PUBLIC_SKYCOT_DEMO_LIVE third mode. |

## P0 punch-list — inherited (status check vs T0119)

| # | brief | T0119 state | T0415 state | demo-camera-sees? |
| - | - | - | - | - |
| 1 | fix-spa-client-use-same-origin-gateway-rewrite | TODO → in_review #2429 | **MERGED #2429** (2026-05-26T18:32Z) | yes (cascade-resolves) |
| 2 | fix-gateway-mod-rs-loc-overage-extract-store-helpers | IN_REVIEW #2422 | **MERGED #2422** | no (unblocks #2407) |
| 3 | fix-in-product-hire-ctas-rewire-to-onboarding-canonical | IN_REVIEW #2424 | **MERGED #2424** | indirect |
| 4 | fix-skills-marketplace-perpetual-loading (seed catalog) | IN_REVIEW #2407 | **MERGED #2407** | YES (but try-chips still 0-results — see new P0 #2) |
| 5 | fix-genesis-orphan-discoverability-and-provider-key-503 | IN_REVIEW #2420 | (status not re-checked this tick) | YES — Genesis Studio investor moment |
| 6 | fix-hire-demo-mode-stub-uuid-worker-roster-invisibility | IN_REVIEW #2419 | (status not re-checked) | YES — Scene 1 hire commit |
| 7 | fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect | IN_REVIEW #2411 | (status not re-checked) | YES — Scene 1 connect-provider |
| 8 | fix-signup-no-email-required-button-honest-copy | IN_REVIEW #2412 | (status not re-checked) | YES — Scene 0 signup |
| 9 | fix-spa-sessionprovider-disable-in-demo-bypass | IN_REVIEW #2413 | (status not re-checked) | YES — console noise |
| 10 | fix-approvals-ops-dashboard-inline-approve-reject-buttons | IN_REVIEW #2414 | (status not re-checked) | YES — Scene 6 approve beat |
| 11 | fix-onboarding-pick-vertical-demo-mode-honest-install-relabel | IN_REVIEW #2423 | (status not re-checked) | YES — Scene 1 vertical step |
| 12 | fix-runbook-scene-1-2-align-with-mariah-actual-duties | IN_REVIEW #2425 | (status not re-checked) | doc-only |
| 13 | fix-workflow-execute-demo-principal-capability-grant | CLAIMED | (status not re-checked) | YES — Scene 4/5 "Run now" |
| 14 | fix-sample-workflows-add-peta-as-editor | IN_REVIEW #2426 | (status not re-checked) | YES — Scene 4 card variety |
| 15 | fix-demo-banner-copy-branch-on-data-source-or-suppress | IN_REVIEW #2427 | (status not re-checked) | YES — every (app)-group page |
| 16 | fix-workflows-dark-mode-card-white-on-white | **MERGED #2421 prior tick** | MERGED | n/a |

**Net** (closed since prior): 1 additional P0 merged this window (#2429, the prior run's headline net-new P0). Plus #2407 + #2422 + #2424 from earlier in the window. **The conversion velocity is strong** — every prior net-new P0 that was authored in T0119 has either merged or progressed through review.

## New P0s exposed by upstream fixes (the "deeper cut" pattern)

A pattern worth naming: **fixing one P0 frequently exposes the next layer**. Examples this run:

1. **#2429 (same-origin fix) merged → workers-collection 404 now visible**. With CORS no longer masking browser-side fetches, scene-1-2 F4 revealed that the SPA calls `/api/v1/workers` (collection) but the gateway only exposes `/api/v1/team`. This was always broken — CORS just hid it behind a different error mode. (Already covered by existing brief `fix-gateway-workers-collection-alias-team`.)
2. **#2407 (skills seed) merged → try-chips empty exposed**. With the catalog seeded, the marketplace renders. But the TRY chips on `/skills` are still wired to noun-phrases that don't match the verb-stem seed names. (New P0 brief #2 above.)
3. **#2424 (hire-CTAs rewire) merged → wizard chrome polish gaps surfaced**. Hire CTAs now land users in the wizard, so the brand-voice drift on `/onboarding/welcome` and the missing demo-mode pill on wizard pages became higher-stakes. (New P1 briefs.)

This is a healthy progression — the camera's view is being cleared one layer at a time.

## P1 punch-list — net-new this run (6 briefs)

1. **fix-onboarding-welcome-hero-flat-brand-voice-drift** — Welcome H1 loses /landing brand voice
2. **fix-gateway-verticals-detail-api-prefix-route-alias** — `/api/v1/verticals/<id>` 404s while list works
3. **fix-spa-error-reporting-init-fetch-loop-on-cors-failure** — runaway fetch loop wedged 4 of 5 audit walks in this run
4. **fix-onboarding-shell-demo-mode-pill-affordance** — wizard pages lack demo-mode indicator
5. **fix-approvals-sla-bar-rounding-60-minute-overflow** — "2h 60m left" rounding glitch
6. **fix-skills-canvas-flat-route-404-or-redirect** — `/skills-canvas` 404s but docs reference it

## 6-MISSION-claim coverage rollup — verdict this run (per whole-site agent)

| # | Claim | T0415 verdict (from SPA observation) | Direction vs T0119 |
| - | - | - | - |
| 1 | Teammate, not tool | 🟡 LIKELY-LIVE (backend) / 🔴 BROKEN-IN-SPA (browser-side cluster) | flat (same root cause cluster) |
| 2 | Delegation is the unit | ❓ NOT-OBSERVABLE-FROM-SPA | flat (no delegation-chain UI surfaced) |
| 3 | Platform builds workers | 🟡 LIKELY-LIVE | flat |
| 4 | Skills shared + learned | 🔴 BROKEN-IN-SPA | partial improvement (#2407 merged) — TRY-chips still empty |
| 5 | Any-PC execution | ❓ NOT-OBSERVABLE-FROM-SPA | flat (computer-use needs operator macOS perms) |
| 6 | Multi-channel + multi-tenant | 🟡 LIKELY-LIVE | flat (multi-channel UI badges still missing — channel-kind brief above) |

**Distance to all-🟢 demo-MP4 recordable**: 3 P0s + 6 P1s (net-new this run) + ~6 inherited in-review/claimed PRs + 1 operator-only blocker (macOS perms for claim 5) + 1 doc/runbook revision (Schedules-tab vs Personality-card vocab) + the demo seed CLI needing to actually be run with the 5-persona roster. **Estimate**: another 2-4 swarm passes if the merge cadence holds; operator-only items (macOS perms + record session) are the tail.

## Top 3 highest-leverage fix-briefs this batch

1. **fix-spa-demo-live-mode-route-real-gateway-on-camera** (P0) — single new env-var mode resolves 3 scene-6 findings simultaneously and closes the "real backend on camera" trust beat
2. **fix-worker-detail-run-now-action-and-endpoint** (P0) — unblocks Scene 2 trigger beat which has no clickable affordance today
3. **fix-spa-error-reporting-init-fetch-loop-on-cors-failure** (P1) — wedged 4 of 5 audit walks this run; agent productivity multiplier
