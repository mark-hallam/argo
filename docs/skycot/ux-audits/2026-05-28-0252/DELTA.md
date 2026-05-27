# UX e2e audit — T0252MEL delta vs prior (T2351MEL, ~3 hours prior)

Authored 2026-05-27T18:56Z (2026-05-28T04:56 Melbourne) by claude conductor
(scheduled-tick `skycot-ux-e2e-audit-4hourly`).

Prior whole-site report: `docs/skycot/ux-audits/2026-05-27-2351/whole-site-mvp-claims-coverage.md`.
This run's whole-site report: `docs/skycot/ux-audits/2026-05-28-0252/whole-site-mvp-claims-coverage.md`.

---

## Headline

**Distance to v1.0 ship-gate: marginally closer; the SPA-side surface gap
identified in the prior run has begun to close at the headline level.** The
whole-site rollup now reports **0 P0s** (was 4 prior), and onboarding-hire-flow
is also at 0 P0s (was 3 prior). The remaining 22 P0s are concentrated in the
two demo-recording scenes — `scene-1-and-2` (Claims 1+2) and `scene-4-and-5`
(Claims 4+5) — which are camera-on surfaces. Net motion: **−3 P0s**, but the
geography of the remaining P0s shifted from "onboarding + whole-site" (less
demo-critical) to "the demo scenes themselves" (recording-critical).

The synthesis pass clustered 86 findings → 24 net-new fix-briefs + 23
duplicates-skipped (mapped to in-flight or shipped briefs). The merge queue
is therefore meaningfully populated for the next worker cycle: 2 new P0s,
10 new P1s, 9 new P2s, 3 new P3s.

## MISSION-claim coverage — T2351 → T0252

| Claim | T2351 (prior) | T0252 (now) | Motion |
|---|---|---|---|
| 1 — Teammate, not tool | 🟢 works in MVP | 🟢 works in MVP | flat |
| 2 — Delegation is the unit | 🟡 partial (schema yes, manual-fire no) | 🟢 works in MVP (per whole-site rollup) | ↑ |
| 3 — Platform builds workers | 🟢 works in MVP | 🟢 works in MVP | flat |
| 4 — Skills shared + learned | 🟡 partial (marketplace renders; install missing) | 🟡 partial (skill detail still brochure, install still missing) | flat |
| 5 — Any-PC execution | 🟡 partial (surface yes, action doesn't fire) | 🟡 partial (surface yes; computer-use not visible in SPA scope) | flat |
| 6 — Multi-channel + multi-tenant | 🟡 partial (surface yes, data isn't) | 🟡 partial (surfaces in place; channel chip absent on /approvals rows, audit worker.name=UUID) | flat |

Backend layer (per CLAUDE.md Status block): all six claims remain 🟢 LIVE
with regression-test pins on origin/main — unchanged. The SPA gap is still
the load-bearing block, narrowing slowly.

## P0 delta

### Closed since T2351 (P0 → resolved or downgraded)

- **Whole-site F01** (audit-row drill-in "Action not found") — no longer the
  load-bearing whole-site P0; resurfaces as `scene-6 F2` this run, mapping
  to existing brief `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`
  + the net-new `fix-gateway-audit-detail-tenant-resolver-drift-vs-feed` (the
  upstream cause: list and detail handlers resolve tenant from different
  headers).
- **Whole-site F02** (workflow `customer_research_followup` execute → 403)
  — covered by existing brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`
  (BLOCKED). Status unchanged backend-side, but no longer a fresh whole-site
  P0.
- **Whole-site F03** (Mariah Schedules tab missing Run-now) — closed in
  flight by PR #2449 (`fix-worker-detail-run-now-action-and-endpoint`,
  IN_REVIEW).
- **Whole-site F04** (`/skills/<slug>` install affordance absent) — covered
  by existing brief `fix-skills-detail-install-button-and-flow` (BLOCKED).
- **Onboarding F3** (DEMO_SESSION hardcode) — MERGED (#2541).

### New P0s (open this run, 2 net-new clusters)

1. **`fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`** (codex) — the
   audit list and detail handlers resolve tenant from different headers
   with different defaults; every row the list returns 404s on detail from
   the SPA. Upstream cause for the recurring audit-drill-in P0. Closes
   `scene-1-and-2 F02` + `scene-6 F2`.
2. **`fix-approvals-row-channel-chip-claim-6-visual-proof`** (claude) —
   `/approvals` never displays the channel field on rows; Claim 6
   multi-channel signal invisible on the headline page despite the channel
   being canonical on the underlying data. Closes `scene-6 F3`.

### Persistent P0s (still open across runs)

- The Scene 4 install-flow trio (skills search · skills-marketplace seed ·
  skill-detail install button) — all already in queue (IN_REVIEW #2462,
  BLOCKED, BLOCKED). No regression, but no merge either.
- Scene 5 customer-research-followup execute 403 — still BLOCKED.
- Worker fixture vs gateway-seed UUID-space drift — net-new this run as
  `fix-onboarding-two-roster-uuid-space-fixtures-vs-gateway` (P1, owner
  claude); load-bearing pre-condition for several persistent visual P0s.

## P0/P1/P2/P3 totals

| Severity | T2351 (5 reports) | T0252 (5 reports) | Delta |
|---|---|---|---|
| P0 | 25 | 22 | −3 |
| P1 | 27 | 25 | −2 |
| P2 | 23 | 29 | +6 |
| P3 | 15 | 10 | −5 |
| Total | 90 | 86 | −4 |

Interpretation: P0+P1 down by 5 in aggregate. P2 rose by 6 — partly because
the synthesis re-categorised some "polish" findings that prior runs had
filed as P1. The total finding count is roughly flat, but the severity
mix is improving.

## Geography of the remaining P0s

| Report | P0 (this run) | P0 (prior run) |
|---|---|---|
| onboarding-hire-an-ai-teammate | 0 | 3 |
| scene-1-and-2 | 8 | 5 |
| scene-4-and-5 | 8 | 5 |
| scene-6 | 6 | 8 |
| whole-site | 0 | 4 |

The shift: P0s vacated onboarding + whole-site (both now zero) and
concentrated in the two recording scenes that the camera lingers on
(scenes 1+2 and 4+5). Two interpretations are consistent with this data:

1. **Positive**: the broad surface area is healing — marketing pages,
   onboarding wizard, settings, cohorts, audit-list all render and behave
   correctly; the remaining defects are deep in the demo-critical workflow
   surfaces.
2. **Negative**: the demo-critical surfaces aren't moving as fast as the
   broader site, and the synthesis is now finer-grained (better
   discrimination → more P0 detections within the same area).

Both effects are likely real. The merge queue should prioritise the 2 net-new
P0s plus the persistent skills + workflow execute P0s in queue.

## Top 3 highest-leverage fix-briefs (queued this batch)

1. **`fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`** (P0, codex)
   — fixes the root cause of the recurring audit-drill-in P0 that has now
   surfaced in 3 consecutive audit runs. ~4–8h. Backend-only, single-file
   scope.
2. **`fix-approvals-row-channel-chip-claim-6-visual-proof`** (P0, claude)
   — Claim 6 multi-channel proof needs to be visible on the headline
   surface for the investor narrative. Currently zero visual evidence on
   `/approvals` rows. ~2–4h web-only.
3. **`fix-gateway-audit-feed-serializer-worker-name-uuid-not-joined`**
   (P1, codex) — the audit feed serialises `worker.name = worker UUID`
   because the SQL doesn't join workers. Demo-live mode shows raw UUIDs
   on Scene 6's audit row pan. ~2–4h backend, one-query change.

## Distance to v1.0 ship-gate

Subjective: **2–3 merge cycles away from a recordable Scene 5 + Scene 6
chain**, conditional on the skills install + workflow execute P0s already
in queue landing. The 2 net-new P0s authored this tick give the merge train
clear targets and an upstream root cause for the audit drill-in issue.

The remaining operator-only item (item d: record the demo MP4 per
`docs/skycot/demos/v1-mission-mvp-runbook.md`) cannot be done until the
6 scenes are all visually solid; this run's net-new P0s narrow that gap
but do not close it.
