# UX e2e audit — T2214MEL delta vs prior

**Run**: 2026-05-26T12:14Z (22:14 Melbourne)
**Prior**: 2026-05-26T11:52Z (21:52 Melbourne) — 22 minutes earlier (off-cadence; expected 4-hourly cadence had next run ≈02:07Z)
**5 audit reports**: onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals, whole-site MVP coverage
**Synthesis**: 9 root-cause clusters identified (~64 findings deduped against the 218 fix-* briefs already on origin/main)
**Ledger insertion**: **BACKED OUT.** All 9 slugs my synthesis emitted slug-collided with the T2152 EXTRAS batch authored 22 min earlier by the prior conductor (still uncommitted on origin/main, sitting in working tree). Backing out my T2214MEL header avoids duplicate-slug rows; the 9 root causes remain queued under the T2152 EXTRAS header. The 9 brief files on disk now carry T2214 source-stamps (my synthesis overwrote the equivalent T2152 EXTRAS content). Net effect: same briefs are queued, just under a T2152 ledger label.

## P0 punch-list — this run

| # | brief | source | demo-camera-sees? |
| - | - | - | - |
| 1 | fix-zombie-pnpm-dev-port-3000-from-stale-worktree | onboarding F-NEW1 | YES — recording would launch in a stale worktree with no `NEXT_PUBLIC_*` env vars |
| 2 | fix-activity-row-audit-detail-synthesized-fixture-theatre | scene-1-2 F4 | YES — Scene 1/2 audit drill-in renders an 8-stage fabricated chain |
| 3 | fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance | scene-6 F-N1/F-N3 | YES — Scene 6 hash-chain drill-in shows `placehold00000000` / `policyplaceholder` |
| 4 | fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona | scene-1-2 F2/F3 + scene-4-5 F4 | YES — Scene 2 has no workflows; Scene 4/5 Run-now is disabled; audit feed is empty so all theatre fixtures activate |

## Delta vs prior (T2152MEL)

### Newly opened P0s (this run found, prior did not)

- **F-N1 + F-N3 (placeholderAuditDetail fabrication)** — prior run flagged self-hash jargon (`fix-audit-empty-state-distinguish-modes-and-self-hash-jargon`) but did NOT name the underlying placeholder fabrication. This run traces it to `web/app/api/v1/audit/action/[actionId]/route.ts:placeholderAuditDetail()` minting `placehold00000000`-shaped ids + a synthetic decisionChain. Material to Scene 6 credibility.
- **F-NEW1 (zombie pnpm dev)** — prior run did not check for orphan dev-server processes. This run found the running dev server (port 3000) was launched from a stale worktree without `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, which silently invalidates every demo-mode fix shipped this week (#2397, #2404, #2405, #2406, plus the prior batch's 4 P0s).
- **F4 scene-1-2 (synthesizeAuditDetail theatre)** — prior run flagged related drill-in failures but not the synthesizer call-site that fabricates the 8-stage chain. Net-new root cause.
- **F4 scene-4-5 (Run-now disabled because workflows table empty)** — prior run noted the empty workflow list but did not connect it back to demo-seed Pass C. This run names the data-side root cause and bundles it with the empty-audit-feed problem into a single seed-publish brief.

### Persistent P0s (still open from prior run, NOT redundantly briefed)

- **fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect** (T2152, still TODO) — onboarding step-3/4 demo bypass
- **fix-auth-demo-mint-route-returns-gateway-unreachable** (T2152, still TODO) — /api/auth/demo-mint proxy
- **fix-signup-no-email-required-button-honest-copy** (T2152, still TODO)
- **fix-spa-sessionprovider-disable-in-demo-bypass** (T2152, still TODO)
- **fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge** (T2152, still TODO)
- **fix-approvals-ops-dashboard-inline-approve-reject-buttons** (T2152, still TODO)
- **fix-docs-sub-routes-all-404** (T2152, still TODO)
- **fix-playwright-hire-flow-regression-cluster** (T2007, CLAIMED, zombie-released — needs rescue)
- **fix-playwright-workflow-builder-regression-cluster** (T2007, CLAIMED on mac-host since 12:08Z — in flight)

All 9 persistent P0s map to existing briefs already in the queue; no duplicate briefs authored here.

### Closed since prior run

Only 22 minutes elapsed since T2152 — no PRs merged in that window. Closure delta is effectively zero.

## 6-MISSION-claim coverage delta

| claim | prior verdict | this run verdict | shift |
| - | - | - | - |
| 1 — Teammate, not tool | YES, strongly demonstrable | YES, demonstrable | stable |
| 2 — Delegation is the unit | PARTIAL (runtime LIVE, SPA silent) | NOW DEMONSTRABLE (DelegationChainCard live via PR #2345) | 🟢 **+1** structural |
| 3 — Platform builds workers | YES, prominently | YES (Genesis SSE live) | stable |
| 4 — Skills shared + learned | YES surface, no seed | SURFACE LIVE, SEED STILL EMPTY (PR #2407 OPEN) | stable — one merge away |
| 5 — Any-PC execution | PARTIAL (operator-driven) | NOT IN SPA SCOPE | stable |
| 6 — Multi-channel + multi-tenant + trust | YES, well | NOW DEMONSTRABLE (ChannelStripCard live via PR #2391) | 🟢 **+1** structural |

**Net shift**: 5 of 6 claims now fully demonstrable as SPA surfaces (1, 2-newly, 3, 4-conditional-on-PR-#2407, 6). Claim 5 is intentionally out of SPA scope. Orientation score moved 73 → 83 (whole-site doc's own reading) but that's a +10 jump driven by recognising PR #2345 + #2391 which already landed before T2152 — so this is more a **recognition delta** than a real-progress delta in the 22-min window.

## Distance to v1.0 ship-gate

**Subjective assessment: ship-recordable today CONDITIONAL on three operator-side gates clearing.**

The five P0 clusters this run names are all theatre-the-camera-sees, but four of them are *invisible without the right env vars set* — they only fire when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is plumbed and the dev server is the *current* worktree, not the stale one squatting port 3000. The single highest-leverage operator action is still:

1. **Kill the orphan pnpm dev process + relaunch from current worktree with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`** (F-NEW1) — this single action unblocks every demo-mode fix shipped this week and is a 30-second operation.
2. **Merge PR #2407 (seed demo marketplace skill catalog) + restart gateway** — closes claim-4 seed gap.
3. **Operator grants macOS Accessibility + Screen Recording perms on argo binary** — closes claim-5 runtime gap.

If all three clear, the demo MP4 is recordable end-to-end with all 6 mission claims visibly demonstrable. The 9 P0 briefs queued this run are post-record polish for the credibility second-tab — none of them block the recording itself, but all of them break credibility if a viewer drills into an audit row.

## Top 3 highest-leverage fix-briefs (this run)

1. **fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona** — single fix closes Scene 2's empty workflows + Scene 4/5's Run-now disabled + audit feed empty + delegation chain card has no data to render. Bundles 4 P0 findings into one P0 brief.
2. **fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance** — collapses the most credibility-destroying piece of theatre in the audit drill-in path. Replacement is 30 lines of "return empty decisionChain + null provenance instead of fabricating."
3. **fix-zombie-pnpm-dev-port-3000-from-stale-worktree** — meta-fix: an `argo demo predawn` script that kills any orphan dev process + restarts from the current worktree with the right env. Saves the operator from a 30-min "why isn't my fix visible?" debugging tangent on recording day.
