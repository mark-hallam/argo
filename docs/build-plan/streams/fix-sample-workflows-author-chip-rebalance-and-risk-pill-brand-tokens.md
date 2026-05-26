# fix-sample-workflows-author-chip-rebalance-and-risk-pill-brand-tokens

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-sample-workflows-author-chip-rebalance-and-risk-pill-brand-tokens`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-4-and-5-claim-4-and-5.md` F10 + F11 + F13.

🟢 **P2 — Three dressing-side polish defects on `/workflows` that the camera sweeps during Scene 5: (1) Author chips credit Mariah on 3 of 8 cards (over-indexed; she's already the canonical Scene 1+2 worker); (2) Risk pill colours are hardcoded HEX literals (`#f59e0b` amber, `#14b8a6` teal) that ignore the violet→pink brand gradient; (3) TRY chips ("approval flows / data extraction / outbound qualification / escalation") are multi-word and half zero-result against the current 8 seeds — same pattern fixed for Skills via #2431.**

## Root cause

**(1) Author chip imbalance.** `web/lib/sample-workflows.ts:64-149` author counts:
- Mariah: 3 (`post_listing`, `morning_pipeline_digest`, `qualify_lead`)
- Jamal: 3 (variants)
- Priya: 1
- Nori: 1
- Peta: 1

Mariah is also the canonical Scene 1+2 worker (per the demo runbook). She's now on the workers page hero AND on 3 of 8 workflow author chips. Over-indexing reads as "small team" on screen. Patrick (per memory `project_pm_cohorts_peta_patrick`, the second PM worker after Peta) is absent. The roster of 6+ named personas is visible elsewhere in the product (cohorts, audit, /team); the workflows catalog only surfaces 5 of them.

**(2) Risk pill HEX literals.** `web/app/(app)/workflows/page.tsx:36-39`:
```ts
const RISK_COLOR = {
  "approval-required": "#f59e0b",
  autonomous: "#14b8a6"
};
```

The locked brand identity (memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses violet → pink gradient + Audiowide. Amber + teal pills read as off-brand and disconnect from the workers-page gradient cards. The risk taxonomy doesn't need to use the gradient itself — but the colours should at least be sourced from CSS variables tied to the brand token system (e.g. `var(--token-warning)` and `var(--token-success)`).

**(3) TRY chips zero-result.** `web/app/(app)/workflows/page.tsx:29`:
```ts
const TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"];
```

Substring-matched against the 8 seed workflows' `name + vertical + description`:
- `approval flows` → matches `approve_refund` description (kw: "approval") ✓
- `data extraction` → matches NOTHING ✗
- `outbound qualification` → matches NOTHING ✗
- `escalation` → matches `triage_ticket` description ✓

Two of four chips zero-result. Same pattern as Skills' chips before #2431 fixed them via single-word stems.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F10 [P2] — Workflow card author chip credits Mariah/Priya/Jamal/Nori/Peta — Mariah is over-indexed (3 of 8 cards)
- scene-4-and-5-claim-4-and-5 F11 [P2] — Workflow viewer's risk pill uses bg-color HEX literals that ignore brand tokens
- scene-4-and-5-claim-4-and-5 F13 [P3] — Workflow TRY chips ("approval flows / data extraction / outbound qualification / escalation") still multi-word — half zero-result against current seeds

## Fix recipe

**(1) Author chip rebalance.** In `web/lib/sample-workflows.ts:64-149`:
- Reassign Mariah's 3 → Mariah×2 (post_listing + one other).
- Reassign `morning_pipeline_digest` → Patrick (per memory; Patrick is a Status Reporter L3).
- Reassign `qualify_lead` → keep Mariah OR move to Peta (Project Coordinator L2). Suggested: move to Peta to balance Project & Ops persona footprint.
- Verify final counts: Mariah×2, Priya×2 (add one Priya assignment), Jamal×2, Nori×1, Peta×1, Patrick×1. Total = 9 across 8 cards (one card may show co-authors; otherwise drop one to land 8).

**(2) Risk pill brand tokens.** In `web/app/(app)/workflows/page.tsx:31-39`:
- Replace `RISK_COLOR` literals with CSS variable references:
  ```ts
  const RISK_COLOR = {
    "approval-required": "var(--token-risk-approval)",
    autonomous: "var(--token-risk-autonomous)"
  };
  ```
- Add the two tokens to the brand token system (likely `web/app/globals.css` or `web/styles/tokens.css` — verify via `rg '\-\-token\-' web/`). Default values: `--token-risk-approval: #f59e0b` (preserves current colour) + `--token-risk-autonomous: #14b8a6`. Future theming can override per-tenant.

**(3) TRY chips rewrite to single-word stems.** In `web/app/(app)/workflows/page.tsx:29`:
```ts
const TRY_CHIPS = ["approval", "qualification", "digest", "triage"];
```
- `approval` matches `approve_refund` ✓
- `qualification` matches `qualify_lead` (via `qualif` substring) ✓
- `digest` matches `morning_pipeline_digest` + `weekly_standup_digest` ✓
- `triage` matches `triage_ticket` ✓

All four chips now resolve to >0 cards.

## Acceptance criteria

- [ ] No persona is credited on more than 2 of 8 workflow cards.
- [ ] Patrick (and ideally Peta) appears at least once on the workflow author chips.
- [ ] Risk pill colours are sourced from CSS variables (no HEX literals in `RISK_COLOR`).
- [ ] All 4 TRY chips return >0 cards when clicked.
- [ ] Vitest pin: assert author-chip counts, brand-token resolution, and chip-filter results.

## May-touch

- `web/lib/sample-workflows.ts` (author chips)
- `web/app/(app)/workflows/page.tsx` (RISK_COLOR + TRY_CHIPS)
- `web/app/globals.css` OR `web/styles/tokens.css` (new brand tokens)
- `web/app/(app)/workflows/__tests__/page.test.tsx` (pin)

## Out of scope

- The customer_research_followup seed (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md`)
- The substrate chip fix (covered by `fix-workflow-substrate-chip-show-macos-browser-e2b.md` + `fix-workflow-substrate-pill-on-cu-workflows.md`)
- The Mariah-as-editor sample-workflows brief (covered by `fix-sample-workflows-add-peta-as-editor.md` — may overlap; coordinate)

## Effort

S — ~60-80 LOC across SPA + fixtures + tokens + pin. ~1-2 hours. P2 because the camera sweeps the workflows catalog during Scene 5.
