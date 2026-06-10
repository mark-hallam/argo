# web-role-contract-review-access-needs-panel

**Owner**: claude or codex (web-only, UI panel + vitest — fast-mode suitable).
**Depends on**: none (#2798 + #2799 merged; this is the missing sibling panel).
**Authored against**: origin/main `02837c942` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-role-contract-review-access-needs-panel

## Goal

The Access needs the operator stated at intake ("Xero") must appear on the
pre-hire review card — today they vanish from the final screen, and the
adjacent Integrations card says "None bound", which reads as a contradiction
of what the operator just said.

## Why now (live e2e 2026-06-11)

During the interview the capture panel correctly shows **"Access needed
(draft-only): Xero"** (`ConversationCapturedPanel.tsx:156-168`). On the final
review card the value is gone: `RoleContractReview.tsx` (365 LOC) has ZERO
references to `access_needs` — while cadence/exceptions/success-criteria all
got panels via #2798 (lines 247-262 / 265-275 / 278-288). The data layer is
VERIFIED working end-to-end: `spec.access_integrations` →
`with_intake_slots` populates `RoleContract.access_needs`
(`role_contract.rs:234-248`, digest-EXCLUDED by design) → the
`vertical_ready` payload carries it (`routes_genesis_stream_payload.rs:53-61`)
→ the TS `RoleContract` type already has `access_needs?: string[]`
(`genesis.ts`). Pure UI gap; last DEC-048 capture slot without review-card
display.

## Fix (~40-60 LOC incl. test)

1. Add an `access_needs` panel to `RoleContractReview.tsx`, mirroring the
   #2798 panel pattern (rc-panel / rc-h4 / list; `Plug` icon is already
   imported at line 14). Place adjacent to the Integrations card (lines
   301-317). Title: "Access requested at intake (not yet connected)".
   Conditional on `contract.access_needs?.length`. `data-testid="genesis-access-needs"`.
2. When `access_needs` is non-empty, soften the Integrations card copy so it
   no longer reads as a contradiction — e.g. "None bound yet — Xero was
   requested at intake and stays draft-only until you connect it."
3. vitest: renders panel with `access_needs: ["Xero","Slack"]` (both items +
   testid present); absent/empty → panel not in DOM; existing #2798 panel
   tests stay green.

## May touch

- `web/components/skycot/genesis/RoleContractReview.tsx` (365 LOC → ~405-415;
  under 500 — do NOT exceed; if the Integrations rephrase pushes it, extract
  the panel into a sibling component file)
- its vitest sibling

## Acceptance / verifier

1. E2E-shaped: interview captures "Xero" → final review card shows the
   Access panel with Xero AND the Integrations card no longer contradicts it.
2. vitest cases above green; tsc clean; file ≤ 500 LOC.
3. Round-trip sanity (existing Rust tests already pin payload carriage —
   no Rust changes in this brief).

## Size / owner / priority

~40-60 LOC. **🟡 P3 — display completeness for a DEC-048 slot** (data layer
verified intact; review-card display is the last gap). No collisions:
`mission-genesis-access-controls-and-guardrails-spec-ui` (#2796, MERGED)
owns the BoundariesPanel policy WRITE path; this is the intake-stated READ
display. No open PRs touch `RoleContractReview` (verified 2026-06-11).
