---
id: DEC-030
date: 2026-05-31
status: active
supersedes: none
---

# DEC-030 — Post-demo build order is locked: A → B → C → D (Voice first)

## Decision

Operator-locked 2026-05-31. With the v1.0 demo essentially **recordable** and the
three-lane build push **complete** —

- **Genesis** P0–P7 (the from-scratch teammate-synthesis safety stack, [[DEC-025]]),
- the **3 hero teammates** (Portia=pitch [[DEC-016]], Grant=GTM [[DEC-018]],
  Marketing [[DEC-028]]),
- the **eval + self-improvement loop** ([[DEC-017]]),
- frontend **de-theatre** + locked-design parity ([[DEC-007]], [[DEC-027]]),

— the **post-demo build order is now locked: A → B → C → D.** Work it top-down,
finish the higher band before dropping to the next (the [[DEC-023]] continuous-
top-down discipline; a blocker is marked PARTIAL/BLOCKED and the next item picked
up, never idle).

### A. Voice — **ACTIVE FRONTIER**

Live voice for the teammates. Vendor + wiring already locked in [[DEC-021]]:
OpenAI **discrete** STT (`POST /v1/audio/transcriptions`) + TTS
(`POST /v1/audio/speech`), `ARGO_VOICE_PROVIDER`-selectable (and user-selectable
via `workspace_settings`); the OpenAI **Realtime** streaming socket is a later
latency upgrade, not this slice.

- **Thin first slice = "talk to Portia by voice"** — speak a question, she answers
  in voice, the whole turn truth-bounded ([[DEC-029]]) over the already-shipped
  `pitch-ask` logic (voice inherits the moat with zero duplication).
- **Why first: it is the biggest differentiator / wow.** A live, truth-bounded
  voice teammate is the single most viral moment in the product and the clearest
  "no competitor does this" beat.

### B. Everything real

Build the backend endpoints the UI currently marks **honestly-unavailable**, and
close the remaining honest-gaps, so the *whole product* is real — not just the
demo path. Known camera-adjacent gaps to close first:

- `/api/v1/api-keys`
- `/api/v1/providers/keys`
- `/api/doctor/channels`

This is the natural continuation of the de-theatre work ([[DEC-007]] NO THEATER,
[[DEC-019]] build out the whole platform): the demo is real; B makes the rest of
the surface real too. Where an endpoint genuinely does not exist yet, the UI keeps
its **honest empty/unavailable state** until B fills it — never faked data.

### C. Marketplace depth

More **real** hero teammates plus a **richer hire-from-catalog** experience —
deepen the catalog and the hire flow beyond the three launch heroes. Each new
teammate is built to the same production-quality + safety bar as [[DEC-025]]
(does real work, can't ruin a business; draft-only / approval-gated where it
acts publicly, cf. [[DEC-028]]).

### D. Enterprise governance

The enterprise-sales table-stakes tail: **SSO** (SAML/OIDC), **SCIM**, **BYOK**
(bring-your-own-key), **data-residency**. Last because it closes enterprise
*sales*, not the funding-round *wow* (the [[DEC-023]] P5 rationale).

> **Build note (D):** the **SSO crypto chain can't build on Windows.** Plan D
> work on macOS/Linux; do not assume a Windows build of the SSO/crypto path.

## Why

- **Sequenced by leverage at this stage.** Voice is the highest-wow, lowest-new-
  surface lever (the vendor + trait seams already exist, [[DEC-021]]) — it ships a
  viral moment fastest. "Everything real" then removes the last honesty caveats
  from the product surface. Marketplace depth widens the value once the core is
  solid. Enterprise governance is real revenue but later, because it wins *sales*
  not the *round* (same logic that put it at P5 in [[DEC-023]]).
- **It extends, not replaces, the priority order.** DEC-023's P1–P5 governed the
  *pre-demo* build-out (kill theatre → wow → real delegation → self-improvement →
  enterprise tail). DEC-030 is the **post-demo** order: the demo and the three
  build lanes are done, so this is what comes next, with **Voice promoted to the
  active frontier** (it was the P2 wow pair; with delegation + self-improvement now
  landed, voice is the front of the queue).
- **Honesty is preserved throughout.** A/B/C/D all stay under NO THEATER
  ([[DEC-007]]) and truth-bounded reporting ([[DEC-029]]): real + tested +
  committed, honest empty states where a backend is absent, and no "done/live"
  claim without an artifact.

## How to apply

- Treat **A (Voice)** as the active frontier: the thin "talk to Portia by voice"
  slice is the next build target, on the [[DEC-021]] discrete-STT/TTS path.
- When A's thin slice is verified live, move to **B (everything real)**, starting
  with the three named honestly-unavailable endpoints above, then sweep remaining
  honest-gaps.
- Keep every item under the build-out discipline ([[DEC-019]], [[DEC-007]],
  [[DEC-029]]) and the `CLAUDE.md` "## Maintain" rule: update the FEATURE_MATRIX
  row in the same commit as the capability, and write any new operator lock as a
  DEC in the same turn.
- [`../../build-plan/PRODUCT_ROADMAP.md`](../../build-plan/PRODUCT_ROADMAP.md)
  carries the A→B→C→D order with Voice flagged ACTIVE as the post-demo product
  frontier.

Related: [[DEC-021]] (voice vendor = OpenAI, discrete STT+TTS, selectable —
the technical basis for frontier A), [[DEC-023]] (pre-demo P1–P5 order this
sequences *after*; same continuous-top-down operating discipline),
[[DEC-010]] / [[DEC-025]] (Genesis — the just-completed lane), [[DEC-007]]
(NO THEATER), [[DEC-029]] (truth-bounded reporting — applies to every "real"
claim B makes), [[DEC-019]] (build out the whole platform — B/C/D are its tail),
[[DEC-028]] (marketing teammate — pattern for C's new heroes).
