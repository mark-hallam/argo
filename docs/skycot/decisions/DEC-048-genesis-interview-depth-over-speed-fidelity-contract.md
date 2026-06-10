# DEC-048 — Genesis interview: depth over speed + captured-equals-built fidelity contract (speed promises removed)

- **Date:** 2026-06-10
- **Status:** **active** — operator-locked 2026-06-10 in the interactive e2e
  session: *"the 4 minutes promise can be deleted, its more important that we
  take the time to build the AI teammate right the first time by getting all
  the relevant information from the user. so get everything right from the get
  go instead of creating unused or useless agents."*
- **Owner:** conductor + any agent touching Genesis intake, onboarding copy,
  or the hire path.
- **Amends:** [DEC-039](DEC-039-genesis-conversational-mode-additive.md)
  (conversational mode stays additive — but the DEFAULT flips to the
  interview; one-shot "Describe it" remains as the shortcut).
  Extends [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) (#7
  operator review-and-edit is promoted to near-term; the v1 modality cap —
  voice / Slack-import deferred — is unchanged).

## 1. Why

The 2026-06-10 operator-requested live e2e walk + 4-agent audit of the Genesis
intake found:

1. **The interview skeleton is real and good** ("Talk it through": 6-slot
   state machine, completeness gate, live capture panel, typed success
   criteria — `crates/argo-genesis/src/intake_machine.rs`,
   `routes_genesis_intake_conversation.rs`). It is NOT the default, asks one
   question per slot, and accepts the first non-empty answer ("Weekly" passes
   as a cadence).
2. **Captured ≠ built.** Cadence is captured then DISCARDED — the hire draft
   hardcodes `0 9 * * 1-5` + canned copy (`web/lib/api/genesis.ts:613`);
   exceptions are display-only (never reach the minted pack's prompts or
   policies); the pre-hire review card doesn't even render
   cadence/exceptions/success-criteria, so the loss is silent.
3. The product promises speed ("In four minutes…", "about ten seconds") at
   exactly the moment the operator wants depth.

Asking and then ignoring is worse than not asking — it manufactures the
"useless teammate" the user discovers a week later. The intake moment is the
single highest-leverage point for teammate quality.

## 2. The rule

1. **Depth over speed.** Intake completeness + fidelity outrank onboarding
   speed. All speed promises ("four minutes", "about ten seconds", per-step
   second counts) are REMOVED from user-facing copy (web + marketing).
   Replacement framing: a short interview so the teammate starts right the
   first time.
2. **Captured-equals-built (the fidelity contract).** Every slot the intake
   captures MUST verifiably land in the minted teammate — cadence → the real
   schedule, exceptions → enforced carve-outs (not just display), success
   criteria → the manifest KPIs — and the pre-hire review must DISPLAY all of
   it. Enforced by a round-trip verifier test (DEC-042 spine: checkable
   verifier, not self-report; NO THEATER applied to intake). A captured field
   that silently drops is a P1 bug class from this date.
3. **The interview is the default intake.** "Talk it through" becomes the
   default Genesis mode; "Describe it" stays available (DEC-039's additive
   contract honored — nothing is removed, the default flips).
4. **Quality bar, adaptively.** Slots fill on QUALITY, not non-empty: vague
   answers get a bounded follow-up (cap 1 re-ask per slot, graceful
   acceptance on provider failure — never block intake on an LLM error). Rich
   answers keep collapsing multiple slots in one turn (already-working
   mechanism), so depth comes from probing vagueness, not from more
   questions. Two question families are added: access/integration needs and
   escalation thresholds.

## 3. Implementation (briefs queued 2026-06-10)

- `fix-genesis-intake-fidelity-captured-slots-must-land` (P1 — rule 2)
- `genesis-interview-quality-bar-and-access-escalation-slots` (P2 — rule 4)
- `genesis-interview-default-mode-and-remove-speed-promises` (P2 — rules 1+3)

## 4. Explicitly NOT in scope

- Voice intake, Slack/Notion/Drive import, full SOP-comprehension moat —
  remain deferred per DEC-025 (#5, Phase 6). This DEC deepens the TEXT
  interview; it does not pull deferred modalities forward.
- Removing the one-shot path (DEC-039 additive stands).
