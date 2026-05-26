# fix-workflows-page-try-chips-zero-results-rewrite

**Owner**: claude or codex.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-page-try-chips-zero-results-rewrite`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-4-and-5-claim-4-and-5.md` F13 [P3].

🟢 **P2/P3 — Half the TRY chips below the `/workflows` hero search return zero results when clicked. Chip text is multi-word noun-phrase (`approval flows`, `data extraction`, `outbound qualification`, `escalation`); workflow names + descriptions don't contain those exact word-order phrases. Same pattern as the Skills page chips that were fixed in #2431 — different page, identical fix shape. Lower severity than the Skills version (the Scene 5 runbook does not direct the operator to click these chips during the demo recording) but worth a follow-on cleanup so that no chip in the product is dead-on-click.**

## Root cause

`web/app/(app)/workflows/page.tsx:29`:

```ts
const TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"];
```

The filter at `web/app/(app)/workflows/page.tsx` does case-insensitive substring match against `name`, `vertical`, and `description` of the 8 seed workflows in `web/lib/sample-workflows.ts:64-149`. Testing each chip:

- `"approval flows"` — matches `approve_refund`'s description ("approval"), but only because "flows" doesn't strictly substring-need to match — actually verify: the literal substring "approval flows" with a space is NOT in any workflow's name + vertical + description. Mismatch.
- `"data extraction"` — substring miss against all 8 (no workflow description contains literal "data extraction").
- `"outbound qualification"` — substring miss (qualify_lead description doesn't have the literal phrase).
- `"escalation"` — matches `triage_ticket`'s description (contains the word "escalation"). PASS.

So 1 of 4 chips hits ≥1 result; 3 of 4 return zero. The Skills marketplace page had the identical bug pattern (6/6 chips dead) and was fixed by #2431 with single-word stems. Apply the same fix.

The runbook narration during Scene 5 does NOT direct the operator to click these chips, so this is lower severity than the Skills version (which was P0 because Scene 4 walks the search beat). But a recording dry-run could land on one of these chips and the camera reads "0 results" — small but real polish miss.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F13 [P3] — Workflow TRY chips not yet realigned to single-word stems like the Skills page

## Fix recipe

Three options, operator-decisional.

### Option A — single-word chips that substring-match (cheapest, recommended)

Rewrite the 4 chip strings to single words that ARE present in the seed workflow names/descriptions:

```ts
const TRY_CHIPS = [
  "approval",  // hits approve_refund
  "extraction", // → rewrite to a chip that hits: e.g. "extract" or "scrape"
  "qualification", // → rewrite to "qualify" (hits qualify_lead)
  "escalation", // already hits triage_ticket
];
```

Suggested final set:
```ts
const TRY_CHIPS = ["approval", "qualify", "triage", "digest"];
```
Each hits ≥1 seed workflow:
- `approval` → approve_refund
- `qualify` → qualify_lead
- `triage` → triage_ticket
- `digest` → morning_pipeline_digest + weekly_standup_digest

### Option B — rewrite chips to canonical vertical names

```ts
const TRY_CHIPS = ["sales", "support", "finance", "gtm"];
```
Each is a vertical that ≥1 seed workflow is tagged under. Reads naturally and is more semantically meaningful than verb stems.

### Option C — extend the filter to be token-aware

Same as the Skills option C — split query on whitespace, match any token. More complex; rejected for #2431 in favor of simpler chip rewrite.

**Recommend Option B** — chips as vertical names give the operator a discovery affordance ("show me sales workflows") that's more naturally meaningful than verb stems. Skills used Option A because the seed names were verb-stems; Workflows have a vertical taxonomy that's more useful.

## Acceptance criteria

- [ ] All 4 TRY chips return ≥1 result when clicked.
- [ ] Chip text reads naturally to a non-technical operator.
- [ ] No regression on the typed-search beat.
- [ ] Vitest pin (extend or new) asserts each chip yields ≥1 result against the current seed set.

## May-touch

- `web/app/(app)/workflows/page.tsx:29` (TRY_CHIPS array)
- `web/app/(app)/workflows/__tests__/page.test.tsx` (assert chip → ≥1 result)

## Out of scope

- Workflow filter logic (case-insensitive substring; unchanged).
- Workflow card author chip rebalance (covered by `fix-workflow-card-author-chip-mariah-over-indexed`).
- Substrate chip wiring (covered by `fix-workflow-substrate-chip-show-macos-browser-e2b`).
- `customer_research_followup` workflow seed (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`).

## Effort

Option A or B: ~4 LOC, ~5 min. Option C: ~40-80 LOC. P2/P3 because the runbook does not direct the operator to click these chips during recording; lower than the Skills P0.
