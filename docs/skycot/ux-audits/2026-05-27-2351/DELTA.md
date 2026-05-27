# UX e2e audit — T2351MEL delta vs prior (T0819MEL, ~15h prior)

Authored 2026-05-28T01:51Z (2026-05-27T23:51 Melbourne) by claude conductor
(scheduled-tick `skycot-ux-e2e-audit-4hourly`).

Prior whole-site report: `docs/skycot/ux-audits/2026-05-27-0819/whole-site-mvp-claims-coverage.md`.
This run's whole-site report: `docs/skycot/ux-audits/2026-05-27-2351/whole-site-mvp-claims-coverage.md`.

---

## Headline

**Distance to v1.0 ship-gate: closer.** Four of six MISSION claims moved toward 🟢
since the morning audit; none regressed. The remaining gap is no longer a backend
liveness problem (CLAUDE.md confirms all six are 🟢 LIVE at the regression-test
layer) — it is purely the SPA failing to *surface* that liveness for two claims
(Skills install affordance, Workflow execute 403). Two of the four net-new P0s
hand the merge train clear targets.

## MISSION-claim coverage — T0819 → T2351

| Claim | T0819 (morning) | T2351 (now) | Motion |
|---|---|---|---|
| 1 — Teammate, not tool | 🟡 LIKELY-LIVE | 🟢 works in MVP | ↑ |
| 2 — Delegation is the unit | ❓ NOT-OBSERVABLE | 🟡 partial (schema yes, manual-fire no) | ↑ |
| 3 — Platform builds workers | 🟡 LIKELY-LIVE | 🟢 works in MVP | ↑ |
| 4 — Skills shared + learned | 🔴 BROKEN-IN-SPA | 🟡 partial (marketplace renders; install missing) | ↑ |
| 5 — Any-PC execution | 🟡 LIKELY-LIVE-via-fixtures | 🟡 partial (surface yes, action doesn't fire) | flat |
| 6 — Multi-channel + multi-tenant | 🟡 LIKELY-LIVE-degraded | 🟡 partial (surface yes, data isn't) | flat |

Backend layer (per CLAUDE.md Status block, refreshed 2026-05-27): all six claims
🟢 LIVE with regression-test pins on origin/main. The gap is entirely SPA-side.

## P0 delta

### Closed since T0819 (P0 → resolved or downgraded)
- `/api/v1/workers` collection 404 — no longer surfaced as a P0 (now P1 F11
  in this run, scoped to single-worker fetch only).
- Mariah's worker-detail Schedules tab structure — schedules render; what's
  missing is the per-row "Run now" button (rescoped to a P0 of its own this
  run, F03).
- Worker activity row href IDs (T0739 P0 worker-overview-activity-row-href-id)
  — this run does not resurface; the in-flight fix in #2464 appears to be
  landing.

### New P0s (open this run)
1. **F01 audit drill-in "Action not found"** — list and detail API IDs don't
   reconcile; demo-Scene-6 drill-in dead-ends. Maps to existing fix-brief
   `fix-audit-list-detail-endpoint-id-mismatch` (already in flight, IN_REVIEW
   per T0819 batch).
2. **F02 workflow execute → 403** — capability grant missing for demo
   principal. Maps to existing brief `fix-workflow-execute-demo-principal-capability-grant`.
3. **F03 Schedules tab missing Run-now per-duty button** — there's a worker-level
   Run-now button (#2449 IN_REVIEW), but no row-level button on the Schedules
   tab. Net-new follow-up — covered by existing brief
   `fix-worker-detail-run-now-action-and-endpoint` extension.
4. **F04 /skills/<slug> install affordance absent** — marketplace card renders
   but detail page has no install CTA. Maps to existing brief
   `fix-skills-detail-install-button-and-flow`.
5. **NET-NEW P0 from onboarding F3** — `DEMO_SESSION` constant hardcodes
   "Acmeco" workspace name, ignoring any value typed into onboarding step 2.
   Sidebar pill reads "Acmeco · Owner" on every in-app page in demo mode,
   regardless of operator input. NEW brief authored this run:
   `fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs`.

### Persistent P0s (open at both T0819 and T2351)
- Skills detail install action (T0819 F02 → T2351 F04) — same root cause.
- Workflow execute 403 (T0819 F02 → T2351 F02) — same root cause.

## Brief-batch impact

- **15 net-new fix-briefs** authored under this batch (header
  `▼ ▼ ▼ UX-AUDIT-T2351MEL BATCH (15 briefs)` in BUILD_STATUS.md).
- **~45 findings** mapped to existing fix-* briefs (queued, in-review, or
  shipped) — no duplicate briefs authored. Mapping shipped in the synthesis
  manifest at the bottom of the synthesis-agent transcript.
- **1 P0, 4 P1, 8 P2, 2 P3** in the net-new batch — the P0 is the Acmeco
  workspace-name hardcode (sidebar pill defies the wizard input on camera).

## Top 3 highest-leverage fix-briefs to unblock next tick

1. **fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs** (P0,
   net-new) — fixing this lifts an obvious "this is a demo, not a product"
   tell off every in-app page during the recorded walkthrough.
2. **fix-skills-detail-install-button-and-flow** (P0, existing,
   pre-T2351) — flips Claim 4 from 🟡 → 🟢 in the SPA; currently the only
   thing blocking that claim's surface from working end-to-end on camera.
3. **fix-workflow-execute-demo-principal-capability-grant** (P0, existing,
   pre-T2351) — unblocks the click-and-watch demonstration of Claim 5; the
   underlying runtime PASSes the cold-cargo regression but the SPA can't
   trigger it because of the cap-gate.

## Direction

Backend: stable + green on all 6 claims. SPA surface: 4 of 6 visibly working
in MVP scope, 2 of 6 (Skills install, Workflow execute) waiting on
capability-gate + install-button fixes that are already authored as briefs.
Once those two land, the 6-MISSION-claim coverage in the SPA will be ≥🟡 for
all six, with 4 of 6 at 🟢 — recordable per the v1-mission-mvp runbook.
