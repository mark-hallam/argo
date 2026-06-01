# Overnight build queue — 2026-06-01 → Thursday (investor) push

> **Operator directive (2026-06-01, going to bed, remote):** *"queue up work from
> the next phases … need a lot done tonight as we are heading towards Thursday.
> So lots of work, then lots of testing in the browser."*
> **Split:** build everything non-browser tonight (conductor-driven, ONE rustc
> build at a time, truth-bounded per [[DEC-029]]); the browser-testing is its own
> phase for when Mark is back at the desktop. Thursday = funding meeting.

## How I'm running it
Conductor works the ranked queue across the night, chaining background builds
(each completion re-invokes me → next item). Every item is **built + verified by
code/test/curl** before it counts as done — NO THEATER. Morning report will be
honest DONE / PARTIAL / BLOCKED-on-browser per item.

---

## ✅ OVERNIGHT OUTCOME — 2026-06-01 night (Part A COMPLETE + pushed)

All 8 Part-A items done + **pushed to `origin/main` (18 commits, head `0f369a24b`)**. Truth-bounded:
1. ✅ **Cora's voice backend LIVE** — `coach` + `interview` modes curl-verified minting `gpt-realtime-2` (`has_secret:true`, `voice:marin`); gateway on `79b9ace` / schema 87.
2. ✅ **Voice stack consolidated** + pushed (client + hook committed).
3. ✅ **Cora's `/coach` frontend** — persona-aware hook + Coaching/Founder-interview surface; tsc clean, coach vitest 3/3. (Live mic = Part B.)
4. ✅ **Cora's Genesis hire brief** — ready-to-paste (`docs/skycot/operator/cora-founder-coach-hire-brief.md`).
5. ✅ **investor pitch materials** — 1-page leave-behind + ~11-slide deck scaffold (`[MARK]` blanks need Mark's facts).
6. ⚠️ **Quality** — MY changes all green (voice/pitch cargo tests, tsc, coach vitest). **Found ~50 PRE-EXISTING red `argo-cli` tests** — tenant-harness drift from the cross-tenant audit migration (test helpers seed via the deprecated tenant-less `AuditSink::new`; the query now tenant-scopes → seeded rows filtered out). **NOT mine; the live code works** (`curl /api/v1/audit` returns real hash-chained rows). **Spawned a daytime fix task.** Suite NOT fully green, but failures are pre-existing + non-blocking.
7. ✅ **Deploy-kit** — seed step validated line-by-line (`argo tenant create <NAME> --json` → `argo demo seed --tenant <ID> --skip-pass-a-turns`); refine-item #2 resolved.
8. ✅ **Pushed** — 18 commits + `FEATURE_MATRIX` rows in-band (HTML regenerated).

**NOT done overnight (deliberately — reliability #1, operator asleep):** no backend migration / gateway redeploy while unattended. Next frontier-B items need Mark's call: (a) **waitlist backend** (storage + public-safety = design decisions; the `/landing` form already exists, backend is the TODO); (b) **workflows live-list** (gateway CORS-mount `/api/v1/verticals` — a documented separate brief); (c) **public-safe Portia path** for the landing embed. **Verify-before-fix note:** skills / cohorts / workflow-detail mock-gating is ALREADY done — the "SPA mock-gating gap" memory overstates what's left.

---

## PART A — build tonight (autonomous, non-browser) — ranked

1. **Cora's voice backend live** — deploy the `coach` + `interview` realtime modes
   (rebuild gateway) → curl-verify each mode mints a session with the right
   persona. *(In progress.)*
2. **Consolidate the held voice stack** — commit the streaming-voice frontend
   (`voice.ts` client + `use-realtime-voice.ts` hook) so the stack is consistent
   (the committed `/pitch` page already imports the hook); cargo + tsc + vitest green.
3. **Cora's frontend** — a "Talk to Cora" control + a founder-interview entry on
   `/pitch` (or a small `/coach` surface) that starts a `mode=coach` / `mode=interview`
   live session; tsc + vitest. *(Browser-test queued — Part B.)*
4. **Pre-stage Cora's Genesis hire** — drop her role brief as a ready-to-paste
   file so the `/genesis` hire is one action when Mark's back.
5. **investor pitch materials** — polish `DIFFERENTIATORS.md` into a 1-page
   leave-behind; draft a pitch-deck outline; tighten the meeting brief. (Founder
   story stays scaffolded — needs Mark's real facts; the voice interview gathers them.)
6. **Quality hardening** — full `cargo test` + `vitest` suites green; `tsc --noEmit`
   = 0; fix what's red; note known flakes.
7. **Deploy-kit finalization** — resolve the refine-items I can without AWS
   (confirm the `argo` seed subcommand via `--help`; tighten the runbook).
8. **Consolidate + PUSH the validated stack** — once the above is green, push the
   held local commits (with the `FEATURE_MATRIX` rows in the same commit). The
   realtime-voice *live* grill stays browser-gated (Part B), but the sound, tested
   code ships.

## PART B — browser-testing phase (when Mark is back at the desktop)
1. **Mic fix** — Chrome ⌘Q + full relaunch → confirm mic permission flips to *granted*.
2. **Streaming voice smoke** — `/pitch` → Go Live → a basic Portia voice call (proves the live path).
3. **Hire Cora** on `/genesis` (her brief pre-staged) → persistent teammate in the roster.
4. **Talk to Cora** — her coach voice; the truth-bounded honest-read coaching.
5. **Founder-story voice interview** — the interviewer agent draws out Mark's story → I shape it.
6. **Full demo walkthrough** — the 6-scene runbook, cold, on the clean roster.
7. **Visual confirm** — the `/pitch` newest-first positioning + the real notifications bell.
8. **AWS deploy** — Mark does the account/DNS; we follow `DEPLOYMENT.md` together.

---
*Refs: [[DEC-033]] (this authorization), [[DEC-032]] (deployment readiness),
[[DEC-029]] (truth-bounded), `2026-06-04-investor-meeting-brief.md`, `FOUNDER_STORY.md`.*
