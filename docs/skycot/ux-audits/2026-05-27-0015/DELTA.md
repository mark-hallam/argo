# UX e2e audit — T0015MEL delta vs prior

**Run**: 2026-05-27T00:15 Melbourne (2026-05-26T14:15Z)
**Prior**: 2026-05-26T23:09 Melbourne (2026-05-26T13:09Z) — 66 min earlier
**5 audit reports**: onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals, whole-site MVP coverage
**Synthesis**: 13 net-new root-cause clusters authored (out of ~62 findings); ~50 findings deduped against the 255 fix-* briefs already on origin/main
**Ledger insertion**: clean — header `▼ ▼ ▼ UX-AUDIT-T0015MEL BATCH (13 briefs)` inserted at top of DEMO-RECORDING UNBLOCK section (above the T2309MEL batch), 0 P0 + 8 P1 + 5 P2 rows queued

## P0 punch-list — this run

This run identified ~16 P0-graded findings, but **all 16 are exact dedupes** of existing fix-* briefs already on origin/main (most authored by prior T2309MEL / T2152MEL / T2214MEL runs). The synthesis agent skipped duplicates per protocol; no new P0 briefs were authored this tick.

The persistent P0 cluster remains:

| # | brief | state | demo-camera-sees? |
| - | - | - | - |
| 1 | fix-genesis-orphan-discoverability-and-provider-key-503 | IN_REVIEW #2420 | YES — Genesis Studio investor moment |
| 2 | fix-hire-demo-mode-stub-uuid-worker-roster-invisibility | IN_REVIEW #2419 | YES — Scene 1 hire commit |
| 3 | fix-workflows-dark-mode-card-white-on-white | CLAIMED codex | YES (if dark-mode recorded) |
| 4 | fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect | IN_REVIEW #2411 | YES — Scene 1 connect-provider |
| 5 | fix-signup-no-email-required-button-honest-copy | IN_REVIEW #2412 | YES — Scene 0 signup beat |
| 6 | fix-spa-sessionprovider-disable-in-demo-bypass | IN_REVIEW #2413 | YES — console error noise |
| 7 | fix-approvals-ops-dashboard-inline-approve-reject-buttons | IN_REVIEW #2414 | YES — Scene 6 approval beat |
| 8 | fix-docs-sub-routes-all-404 | IN_REVIEW #2416 | NO (camera doesn't navigate /docs) |
| 9 | fix-claude-launch-json-env-var-naming-mismatch | unclear | YES — silent disabling of mocks |
| 10 | fix-skills-detail-install-button-and-flow | unclear | YES — Scene 4 install beat |
| 11 | fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds | unclear | YES — Scene 5 workflow render |
| 12 | fix-audit-detail-render-empty-decision-chain-and-provenance-fallback | unclear | YES — Scene 6 drill-in |
| 13 | fix-audit-feed-action-kind-variety | unclear | YES — Scene 6 audit feed variety |
| 14 | fix-approvals-seed-decided-approvals-for-ring | unclear | YES — Scene 6 approval-rate ring |
| 15 | fix-gateway-cors-localhost-127-and-preview-proxy-origins | unclear | YES (if recording on 127.0.0.1:3000) |
| 16 | fix-audit-feed-include-self-hash-in-envelope | unclear | YES — hash chain hover |

## Delta vs prior (T2309MEL)

### Newly opened P0s

**Zero net-new P0 briefs this run.** Two interpretations: (a) the prior run + the broader brief library already cover every camera-visible P0 surface — synthesis correctness is high; (b) the swarm has converged on a stable defect inventory and the bottleneck is now landing the existing briefs, not finding new ones.

### Newly opened P1s (NET-NEW — not covered by prior briefs)

8 P1 briefs authored this run cover the deeper P1 layer the prior runs surfaced more shallowly:

- **fix-onboarding-pick-vertical-demo-mode-honest-install-relabel** — pick-vertical subtitle promises "we install skills/memory/approval-gates" but demo-mode skips it; honest-relabel or wire actual install
- **fix-in-product-hire-ctas-rewire-to-onboarding-canonical** — 7 in-product Hire CTAs still push to parallel `/hire/step/*` wizard; only `/landing` was rewired in prior PR
- **fix-runbook-scene-1-2-align-with-mariah-actual-duties** — runbook narrates 3 duty IDs (`morning_kickoff` etc.) that DON'T EXIST anywhere in the codebase; Mariah actually has 4 different labels
- **fix-workflow-execute-demo-principal-capability-grant** — POST `/api/v1/.../execute` returns 403 for the demo principal; "Run now" in Scene 4/5 will show error toast on camera
- **fix-sample-workflows-add-peta-as-editor** — Peta is never a lastEditor across the 7 sample workflows; canonical 5-roster off-by-one on the workflow cards
- **fix-demo-banner-copy-branch-on-data-source-or-suppress** — banner says "showing sample data" regardless of actual source; misleads investor watching real backend
- **fix-vertical-landing-trust-strip-honest-compliance-claims** — all `/for/[vertical]` pages claim GDPR · HIPAA · AFSL; legal risk + contradicts `/security` "SOC 2 in progress"
- **fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira** — vertical pages reference Skylar/Fen/Mira (not in canonical roster); breaks Scene-1 → Scene-0 continuity

### Closed since prior run (66 min window)

**9 PRs opened in IN_REVIEW state** (most from prior T2309/T2152/T2214 batches): #2411, #2412, #2413, #2414, #2416, #2417, #2418, #2419, #2420. This is the highest single-tick PR-opening rate observed in the audit history — the swarm is actively converting queued briefs into reviewable PRs.

**No P0-closing merges** in the 66-min window (the in-flight PRs all still in IN_REVIEW). Closure delta = 0 measured by merged-to-main; conversion-to-PR delta = +9.

## 6-MISSION-claim coverage delta (this run vs T2309)

| claim | T2309 verdict | T0015 verdict | shift |
| - | - | - | - |
| 1 — Teammate, not tool | 🟡 SPA mocks; hire-flow mints invisible stub | 🟡 same SPA mocks; hire-flow STILL mints invisible stub (fix #2411/#2419 in review, not merged) | stable |
| 2 — Delegation is the unit | 🟡 DelegationChainCard renders fixture handoffs | 🟡 unchanged | stable |
| 3 — Platform builds workers | 🔴 Genesis unreachable + 503s | 🔴 unchanged — #2420 still IN_REVIEW; Genesis still nav-orphaned on main | stable |
| 4 — Skills shared + learned | 🟡 marketplace UI live, 0 skills in gateway | 🟡 unchanged; "competitor pricing scrape" still 0 hits | stable |
| 5 — Any-PC execution | NOT IN SPA SCOPE | NOT IN SPA SCOPE — same | stable |
| 6 — Multi-channel + multi-tenant + trust | 🟡 ChannelStripCard renders; audit drill-in returns "Unknown worker" | 🟡 unchanged; cross-tenant header isolation still NOT verified at runtime (F10 scene-6) | stable |

**Net shift**: zero. All 6 claims hold their position. This was a stability-confirming run — the defect inventory is stable, the briefs are being converted to PRs (very good signal), but nothing has yet merged to flip a claim's color.

Orientation score: ~71 (vs 70 at T2309) — fractionally improved by IN_REVIEW progress, but unchanged in operative state.

## Distance to v1.0 ship-gate

**Ship-recordable today: NOT YET, but the bottleneck has shifted.**

T2214 → T2309 → T0015 trajectory:

- **T2214**: 3 operator-side gates + several P0 demo-blocker briefs to author
- **T2309**: 3 operator-side gates + 3 NEW P0 demo-blocker briefs (Genesis, stub-uuid, dark-mode) + 13 in-flight from prior runs
- **T0015**: 3 operator-side gates UNCHANGED + **0 new P0 briefs** + the in-flight P0 set is now mostly **IN_REVIEW with PRs awaiting merge**

The new bottleneck is **PR throughput, not brief authorship**. Nine PRs sit IN_REVIEW; getting them merged closes the bulk of the demo-camera P0 surface in one operator-driven session (the `--skip-clippy` authorisation + web-only relaxation memos cover most of them).

The 3 operator-side gates remain unchanged:

1. **Kill orphan pnpm dev + relaunch with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`** — 30-second op
2. **Grant macOS Accessibility + Screen Recording perms on argo binary** — closes claim-5 runtime gap
3. **Sweep the 9 IN_REVIEW PRs to merged** — `gh pr list --search "is:open is:pr in:title fix-"`, run pre-merge-gate on each, admin-merge per the T2309/T0015 operator authorisations

## Top 3 highest-leverage fix-briefs (this run)

1. **fix-workflow-execute-demo-principal-capability-grant** — Scene 4 + Scene 5 both end with operator hitting "Run now". Current gateway returns 403 for the demo principal — the camera will see a red error toast. Single-file demo-seed fix. Effort: S.
2. **fix-runbook-scene-1-2-align-with-mariah-actual-duties** — runbook tells operator to narrate 3 duty IDs that don't exist. Either operator memorizes incorrect script and recording is invalid, or operator notices on-camera and stumbles. Pure doc fix, no code risk. Effort: S.
3. **fix-vertical-landing-trust-strip-honest-compliance-claims** — vertical landings claim HIPAA + AFSL on every page. Investors will check. The `/security` page says "SOC 2 in progress" — direct contradiction. Effort: S. (LEGAL/CREDIBILITY weight — not strictly demo-camera, but if any investor screenshots the vertical landing and asks "are you actually HIPAA compliant?", we have to say no.)
