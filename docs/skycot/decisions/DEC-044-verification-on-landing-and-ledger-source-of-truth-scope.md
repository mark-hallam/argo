# DEC-044 — Browser-verification-on-landing quality gate + ledger source-of-truth scope (reaffirmed)

- **Date:** 2026-06-09
- **Status:** **active** — operator-directed (2026-06-09): *"can you spawn agents to ensure features are all available in the browser, wiring and UX and UI is all correct when stuff lands?"* and *"is keeping the ADR and feature roadmap updated as the source of truth in your scope? … that should be captured."*
- **Owner:** conductor (Claude).
- **Relates to:** [DEC-029](DEC-029-truth-bounded-status-reporting.md) (NO THEATER), [DEC-042](DEC-042-continuous-self-improvement-loop.md) (every loop ends in a checkable verifier), [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) (conductor-driven run-mode + twice-daily live signup gate), [DEC-007](DEC-007-no-theater-in-demo.md) (no theater), [DEC-043](DEC-043-stop-committing-generated-feature-matrix-html.md) (the .md roadmap is the source of truth).

---

## 1. Why
"Done" has meant **unit tests pass**. But the user-visible truth — does the feature actually render, with REAL (not mock) data, with working wiring (clicks/forms/nav), matching the locked design — lives in the **browser**, which the unit-test gate never exercises. That is the same NO-THEATER hole DEC-029/DEC-007 close for status/demo, now closed for the frontend. Separately, the operator asked to confirm that keeping the decision register + feature roadmap live as source-of-truth is in scope.

## 2. Decision — two standing scopes, now explicit

### 2a. Browser-verification-on-landing quality gate (NEW)
When a **web-affecting change lands** (or a batch lands), the conductor runs a **real browser verification** — load the affected SPA surface (Claude Preview tooling: dev server + navigate/click/fill/snapshot), confirm it (i) renders, (ii) shows REAL backend data not mock/fixture, (iii) the wiring works (interactions, forms, navigation, no console errors), (iv) the UX/UI matches the locked design system ([DEC-027](DEC-027-brand-and-design-system-locked.md)) — and **captures a screenshot / snapshot as the checkable verifier** (the [DEC-042](DEC-042-continuous-self-improvement-loop.md) verifier spine applied to the frontend; never the model's say-so). Anything wrong becomes a fix-brief queued into the swarm.

Two cadences:
- **Per web-landing:** the affected surface, when its PR merges.
- **Periodic full-loop walkthrough:** the whole stranger path (signup → own empty workspace → Genesis hire → real work → approval → hash-chained audit → can pay) — the DEC-034 twice-daily live signup gate.

### 2b. Ledger source-of-truth maintenance is in scope (REAFFIRMED — already a CLAUDE.md rule)
The decision register (`docs/skycot/decisions/`) and the feature roadmap (`docs/build-plan/FEATURE_MATRIX.md`) are kept **live in-band**: every operator-locked call gets a `DEC-NNN` ADR + README index line the same turn; every shipped/changed capability updates its `FEATURE_MATRIX.md` row the same commit; the conductor keeps the `claude` TODO queue fed (replenish when at/below threshold) via the mission-grounded **discovery → curate → author → verify** fan-out. (DEC-043 clarified the `.md` is THE roadmap; the generated `.html` view is gitignored, not source.)

## 3. Honesty / mechanism (NO THEATER about what this is)
- Browser-verify needs the SPA + gateway running and is heavier than unit tests, so it runs **per-web-landing (affected surface) + periodic full-loop**, not on every commit.
- It is a **real verifier** (screenshot/interaction observed this turn), not a self-report.
- The conductor (an active session, DEC-034) drives it; when **no session is active it does not run** (the cron conductor stays disabled). Fully-autonomous browser verification is DEC-042 self-improvement-loop territory (not built yet) — stated plainly so "we browser-verify" is never read as "it happens unattended."

## 4. Deploy timing (RECOMMENDATION — becomes its own DEC when the operator triggers it)
First-principles staged posture (do not big-bang a public launch):
1. **Gated private preview — ready now-ish.** The deploy kit is built (DEC-032/WS5: loopback gateway behind nginx TLS + basic-auth; prod SPA build), and the platform loop is verified working for a fresh stranger (the 2026-06-09 discovery sweep confirmed WS1/WS2 functional). A basic-auth-gated preview lets real people test **without a public abuse surface** or live billing. Operator-owed: AWS account + DNS + cert + `.htpasswd` + secrets.
2. **Fully-public self-serve — gated on four checks:**
   - perimeter hardening (anonymous rate-limits + LLM-cost caps + the public rate-limited pitch endpoint — queued 2026-06-09);
   - billing live (operator creates the $49 Stripe price + provides keys — DEC-035);
   - a **browser-verified clean loop** (per 2a);
   - **adversarial review on the 3 risky surfaces** (tenant isolation / billing / public perimeter — DEC-034).

The "go public" moment is locked in its own DEC when those four are green.

## 5. Verifiable
- Browser-verify gate is working when a web-landing produces a committed screenshot/snapshot artifact + (if broken) an auto-queued fix-brief, within one cycle.
- Ledger scope is working when every session's DECs + FEATURE_MATRIX rows are in-band (zero later-sweep reconciliations) and the queue does not starve while a session drives.
