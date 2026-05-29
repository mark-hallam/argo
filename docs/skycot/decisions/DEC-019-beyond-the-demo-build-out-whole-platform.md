---
id: DEC-019
date: 2026-05-29
status: active
supersedes: none
---

# DEC-019 — Beyond the demo: build out the whole platform, remove ALL theatre, drive to the mission

## Decision

The active program is no longer "prepare the demo." It is **build out the whole
platform**: ship every feature for real (voice first), **remove ALL theatre**
(every mock/fixture surface that the user can reach becomes real or is honestly
marked), and **drive the product to the mission**. The demo MP4 stops being the
goalpost; a real, working platform is.

This **reframes** (does not contradict) the prior demo-prep framing: the demo is
now an output of a working platform, not the thing we build toward.

## Why

- Operator directive (2026-05-29 night): "go beyond the demo … build out every
  feature, remove all the theatre, bring the platform to the mission."
- The MVP-reframe insight (2026-05-29) was that "v1.0 MVP / claims LIVE"
  overstated UI reality — roughly two dozen SPA surfaces still showed mock data
  even in demo-live. The honest response is to *finish the wiring*, not to keep
  dressing scaffold as features.
- NO THEATER ([[DEC-007]]) was a demo-scoped rule; this decision **generalizes**
  it to the whole product: theatre the user can reach is a bug everywhere, not
  just in the camera frame.

## How to apply

- The master tracker is [`docs/skycot/overnight-build-out.md`](../overnight-build-out.md)
  (queue + log + resume anchor). All build-out work threads through it.
- Prioritise: remaining theatre/mock surfaces → un-wired-but-shipped scaffolds →
  `docs/build-plan/FEATURE_MATRIX.md` 🟡 Partial / 🟧 Scaffold rows, highest value
  first. Always: **real + tested + committed**, NO THEATER, never break the
  running demo (gateway :7400 / SPA :3000).
- Honesty applies to our own status too: a faithful build-out of every feature is
  weeks of work, not one night — report DONE vs PARTIAL vs BLOCKED truthfully,
  never dress incomplete work as finished.
- Every shipped feature updates FEATURE_MATRIX and every operator-locked call gets
  a register ADR **in the same commit/cycle** (the discipline this decision makes
  load-bearing — see the build-out tracker's RESUME INSTRUCTIONS).

Related: [[DEC-007]] (no theater — now generalized), [[DEC-020]] (the autonomous
machine that executes this), [[DEC-022]] (Sprint-2 full-program scope),
[[project_mvp_reframe_working_core_not_shipped]].
