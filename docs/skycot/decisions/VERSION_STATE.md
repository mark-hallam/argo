# SKYCOT Version State — reconciliation (2026-05-29; updated 2026-05-30)

The honest answer to "where are we?" — reconciling the originally-planned
v1.0 against the MVP scope as it actually grew. Companion to the decision
register; read alongside `README.md`.

## TL;DR

**As of 2026-05-30 we are in the Sprint-2 platform build-out** ([[DEC-019]] +
[[DEC-022]]): the program has moved **beyond demo-prep** to building out the
whole platform for real, removing ALL theatre, and driving to the mission —
running autonomously ([[DEC-020]]) with real voice on OpenAI ([[DEC-021]]). The
live queue is in [`docs/skycot/overnight-build-out.md`](../overnight-build-out.md).

All 6 mission claims are LIVE on the demo path, but the **MVP scope grew
materially** beyond the original v1.0 definition (Pitch Agent, Genesis,
computer-use-as-must-have folded in mid-stream — DEC-008/009/010), and the
2026-05-29 MVP-reframe found ~24 SPA surfaces still showing mock in demo-live.
Sprint 2 is the catch-up: finish the wiring rather than dress scaffold as
features. This document reconciles the originally-planned v1.0 against where we
actually are.

## What "v1.0" originally meant vs. what it means now

| Aspect | Original v1.0 plan | Actual MVP (as scoped by operator decisions) |
|--------|--------------------|----------------------------------------------|
| 6 mission claims LIVE | ✅ required | ✅ required (all LIVE) |
| Demo MP4 | ✅ ship gate | ✅ ship gate (not yet recorded — operator-only) |
| Pitch Agent | beyond-v1.0 headline | **IN MVP** (DEC-008) — now the demo headline |
| Genesis onboarding studio | post-MVP capability | **IN MVP** (DEC-010) — Phase-1 shipped, Phase-2 partial |
| Computer-use | v1.x stretch | **MVP-must-have** (DEC-009) — macOS path, Scene 5 |

## Current claim status (per MISSION.md, operator-locked round 19 criteria)

All 6 🟢 **LIVE**:
1. Teammate not tool — LIVE
2. Delegation is the unit — LIVE
3. Platform builds workers (incl. Genesis) — LIVE
4. Skills shared + learned — LIVE
5. Any-PC execution (macOS computer-use) — LIVE
6. Multi-channel + multi-tenant — LIVE

## Cargo / semantic version

`Cargo.toml version = "0.0.1"` — never bumped. The "v1.0" framing is the
**product/mission milestone**, not the crate semver. Recommendation: when the
demo MP4 records + the MVP ships, bump the workspace to a real version
(e.g. `1.0.0`) so semver tracks the product milestone. Decision deferred to
operator.

## Outstanding for the v1.0 ship gate

- **Demo MP4 recording** — operator-only artifact (per runbook). Blocked only
  by operator availability + the two known live-SPA issues below.
- **Genesis "Try again" 503** — live-SPA idempotency retry still 503s despite
  #2641; demo workaround = clear `verticals/.genesis/op-*` before each take.
  Follow-up fix outstanding.
- **Scene 5 capability gate** — demo principal can't execute workflows; the
  demo must record signed-in as a real account (correct behavior, operator-side).

## What grew the timeline (the documented misalignment)

The MVP-scope expansion (DEC-008 + DEC-009 + DEC-010) added the Pitch Agent
tree (P-01..P-08), the Genesis pipeline + onboarding studio, and computer-use
hardening — none of which were in the original v1.0 line. This is net-positive
(more shipped, stronger demo) but it moved the goalposts without updating
BUILD_PLAN_V11.md / PRODUCT_ROADMAP.md at the time. Those docs should now be
reconciled to reflect that Pitch + Genesis + computer-use are v1.0, not v1.x.

## Beyond v1.0 (still genuinely deferred)

Per BUILD_PLAN_V11 phased plan, still post-v1.0:
- Linux / Windows / E2B computer-use (DEC-002 keeps these v1.x)
- Genesis Phase-2 multi-modal onboarding (voice + doc upload + comprehension
  verification) — the fuller vision beyond shipped Phase-1
- Pitch Agent web-embed channel (P-06 dropped) + Zoom/live-meeting channel
  (P-07 blocked — multi-day macOS audio routing)
- Cross-tenant marketplace install, canary, A/B testing of skills (claim-4
  end-state)
- The post-MVP product-surface program (Tracks A–G)

## Recommended doc reconciliation (follow-up)

1. Update `BUILD_PLAN_V11.md` + `PRODUCT_ROADMAP.md` to move Pitch / Genesis /
   computer-use from v1.x into the v1.0 line (cite DEC-008/009/010).
2. Decide the Cargo semver bump policy at MVP ship.
3. Keep this VERSION_STATE.md current as the single "where are we" answer.
