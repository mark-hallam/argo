---
id: DEC-033
date: 2026-06-01
status: active
supersedes: none
---

# DEC-033 — Overnight autonomous build toward the investor meeting (Thursday)

## Decision

The operator (2026-06-01, remote, going to bed) put the conductor in charge of an
**autonomous overnight build toward Thursday's funding meeting** — *"queue
up work from the next phases … need a lot done tonight as we are heading towards
Thursday. So lots of work, then lots of testing in the browser."* The work splits
in two: **build everything non-browser tonight** (conductor-driven), and **the
browser-testing is its own phase** for when Mark is back at the desktop.

Executes the [[DEC-026]]/[[DEC-031]] overnight-autonomous pattern under [[DEC-029]]
truth-bounding. The live queue is `docs/skycot/overnight-build-queue-2026-06-01.md`.

## Why

Thursday is the funding meeting (brief at
`docs/skycot/fundraising/2026-06-04-investor-meeting-brief.md`). With ~3 days and 0
traction, the asset is the *working product + the founder*: a live Cora (founder
coach with voice), the streaming-voice headline, the pitch materials, and a
deployable gated URL. Most of the *building* is non-browser and can run tonight;
the *validation* (mic, live voice, the demo walkthrough, the deploy) needs Mark +
his browser, so it's queued separately.

## How (held)

- **Truth-bounded ([[DEC-029]]):** every item built + verified by code/test/curl
  before it counts; morning report is honest DONE / PARTIAL / BLOCKED-on-browser.
- **ONE rustc build at a time** (near-OOM Mac; sccache + capped jobs + real-disk
  target dir). Conductor-driven, NOT the worker swarm (kept halted) — controlled +
  verified over unattended churn.
- **NO THEATER ([[DEC-007]]):** browser-gated validation is *not* claimed done from
  code alone — it's listed for Mark's browser phase.
- **Push** the held validated stack with `FEATURE_MATRIX` rows in-band; the
  realtime-voice *live* grill remains browser-gated.

## Outcome
Tracked live in `docs/skycot/overnight-build-queue-2026-06-01.md` + the morning report.

Related: [[DEC-031]]/[[DEC-026]] (overnight pattern), [[DEC-029]] (truth-bounded),
[[DEC-032]] (deployment readiness), [[DEC-021]] (voice).
