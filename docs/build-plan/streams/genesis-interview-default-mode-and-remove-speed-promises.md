# genesis-interview-default-mode-and-remove-speed-promises

**Owner**: claude or codex (web-heavy; copy + 1-line default flip + one edit seam).
**Depends on**: `fix-genesis-intake-fidelity-captured-slots-must-land` (the review-DISPLAY panels + payload fields land there; this brief makes them editable). Coordinate with `mission-genesis-access-controls-and-guardrails-spec-ui` (CLAIMED — owns BoundariesPanel on the same review surface).
**Authored against**: origin/main `aac402865` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh genesis-interview-default-mode-and-remove-speed-promises

## Goal ([DEC-048] rules 1 + 3)

Make the interview the default Genesis intake, remove every speed promise
from user-facing copy, and let the operator EDIT the captured
cadence/exceptions/success-criteria on the pre-hire review (DEC-025 #7).

## Why now (operator-locked 2026-06-10, DEC-048)

Operator: "the 4 minutes promise can be deleted, its more important that we
take the time to build the AI teammate right the first time… instead of
creating unused or useless agents." Today the one-shot textarea is the
default tab, the product promises "four minutes" / "about ten seconds" at the
exact moment depth matters, and the review screen is read-only — a wrong
capture can't be corrected pre-hire.

## Fix

**1. Default mode flip (1 line + test).**
`web/app/(app)/genesis/page.tsx:95` — `useState<GenesisMode>('describe')` →
`'talk'`. The `GenesisModeToggle` stays exactly as-is (DEC-039 additive
honored: describe-it remains one click away; toggle reversible until
streaming starts). page.tsx is at **481/500 LOC** — this brief may add at
most a couple of lines there; all new UI goes in separate component files.

**2. Speed-promise copy sweep (16 verified hits; delete, don't soften).**
Canonical replacement (use VERBATIM where a sentence is needed):
**"A guided interview so your teammate starts right the first time."**
- `web/app/landing/page.tsx:368` — "four minutes" hero line → canonical.
- `web/app/auth/signup/page.tsx:258` — "In four minutes, you'll…" → canonical.
- `web/app/(hire)/hire/page.tsx:121` — "four minutes" → canonical.
- `web/app/(hire)/hire/step/[step]/page.tsx:69` — "about ten seconds" → drop
  the timing clause.
- `web/app/(setup)/onboarding/welcome/page.tsx` — step cards "STEPS 2-3 ·
  90 SEC" → "STEPS 2-3 · SETUP"; "STEPS 4-5 · 2.5 MIN" → "STEPS 4-5 · HIRE";
  the "In four minutes you'll onboard…" hero → canonical.
- `web/components/skycot/genesis/HeroInput.tsx:180` — "in about ten
  seconds" → "We'll draft a real worker — persona, charter, and skills —
  and walk you through reviewing it."
- `web/app/for/_data/verticals.ts` (~:114/:167/:224/:277) — 4 "in minutes"
  sub-lines → outcome language ("configured, reviewed, and ready").
- Sweep check: `rg -i "four minutes|4 minutes|ten seconds|90 sec|2\.5 min"`
  across `web/` AND `marketing/` must come back EMPTY for user-facing copy
  when done (tests/comments exempt).

**3. Review-and-edit seam (DEC-025 #7, scoped).**
New separate component `CadenceExceptionsEditor.tsx` (~80-120 LOC, own file —
NOT inline in page.tsx): on the pre-hire review, "Adjust" affordance that
pre-fills from the `vertical_ready` payload (server echo — single source of
truth, no client-side divergence) and lets the operator correct cadence /
exceptions / success-criteria before confirming. **Reuse**
`CadenceExceptionsCapture.tsx` as the form internals — do not fork a second
capture form. Edits are advisory, never gate the Confirm button (empty/
skipped values fall back to server defaults). Panel ordering with the
access-controls brief's BoundariesPanel: boundaries/policy first, then
cadence/success — both feed the single hire confirmation.

## May touch

- `web/app/(app)/genesis/page.tsx` (481 LOC — ≤ ~5 LOC delta: default flip +
  editor mount)
- the 8 copy files listed above (≈20-30 LOC of rewrites total)
- `web/components/skycot/genesis/CadenceExceptionsEditor.tsx` (NEW) +
  `CadenceExceptionsCapture.tsx` (export reuse only)
- `web/lib/api/genesis.ts` types ONLY if `role_contract` payload fields are
  missing after Brief A (extend in a sibling `genesis-*.ts`, file is 690 LOC
  grandfathered)
- vitest siblings (`GenesisModeToggle.test.tsx`, genesis page specs)

## Acceptance / verifier

1. /genesis fresh load opens in "Talk it through"; toggle still switches to
   "Describe it" and back (existing toggle tests extended with the
   initial-state assertion).
2. `rg -i "four minutes|ten seconds|90 sec|2\.5 min" web/ marketing/` →
   zero user-facing hits; vitest regression asserts the strings are ABSENT
   from landing/signup/hire/onboarding-welcome page text.
3. Review screen: operator edits cadence on a generated teammate pre-hire →
   confirm → the hire draft carries the EDITED value (vitest round-trip on
   the editor + buildHireDraftFromGenesis); Confirm stays enabled with the
   editor untouched.
4. `genesis/page.tsx` ≤ 500 LOC after change (assert in PR body);
   tsc + vitest green.

## Size / owner / priority

~150-220 LOC incl. tests (mostly the editor + copy). **🟠 P2 — DEC-048
rules 1+3** (P1 sibling `fix-genesis-intake-fidelity-captured-slots-must-land`
ships first — the editor pre-fills from fields that brief surfaces).
DEC-039's additive contract is explicitly honored: nothing removed, default
flipped per DEC-048.
