# fix-skills-try-chips-zero-results-rewrite

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-try-chips-zero-results-rewrite`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-4-and-5-claim-4-and-5.md` F2 [P0].

🔴 **P0 DEMO-BLOCKER — All six TRY chips below the `/skills` hero search bar return 0 results. Chip text is noun-phrase ("approval routing"), skill names are verb-stem ("approval.route"); substring match fails for 42/42 chip-skill pairs. If the operator falls back to clicking a chip during Scene 4, the count pill flips to "0 shown" and the empty-state panel reads "No skills match…" — telegraphing to the camera that search is broken.**

## Root cause

`web/app/(app)/skills/page.tsx:23-30` defines:

```ts
const TRY_CHIPS = [
  "approval routing",
  "invoice extraction",
  "candidate screening",
  "deal qualification",
  "support triage",
  "lead enrichment",
];
```

Each chip click sets the search query to the chip text via `setQuery(chip)`. The filter at `page.tsx:65-78` is a case-insensitive substring match across `name`, `description`, and `layer`.

The seven seed skill names (`web/lib/api/skills.ts:64-143`) are verb-stems with dot separators:

- `approval.route`
- `invoice.extract`
- `candidate.screen`
- `deal.qualify`
- `support.triage`
- `lead.enrich`
- `pricing.competitor_scrape`

The substring `"approval routing"` (with space) does NOT match `"approval.route"`. Verified by inlining the filter against all 6 chips × 7 seeds — 0 of 42 pairs match.

The hero search bar's intended affordance is "click a try-chip to populate the search and see results immediately". Currently the chips are anti-affordances that visibly degrade.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F2 [P0] — Every TRY chip below the search bar returns 0 results

## Fix recipe

Three options, operator-decisional:

### Option A — single-word chips that substring-match (cheapest, recommended)

Rewrite the 6 chip strings to single-word stems that ARE present in the seed skill names + descriptions:

```ts
const TRY_CHIPS = [
  "approval",
  "invoice",
  "candidate",
  "deal",
  "support",
  "lead",
];
```

Each is unambiguous, reads naturally, and hits ≥1 seed skill. Total change: 6 string edits.

### Option B — rewrite seed descriptions to lead with the noun-phrase

Prefix each `description` field at `web/lib/api/skills.ts:64-143` with the chip's noun-phrase wording:

```ts
{ name: "approval.route", description: "Approval routing — route approval requests…" }
```

Slightly more changes; preserves the existing noun-phrase chips that may be more discoverable for non-technical operators.

### Option C — extend the filter to tokenise + match any-token-match

`page.tsx:65-78` filter becomes token-aware: split the query on whitespace, match if every token appears anywhere in name/description/layer. Allows "approval routing" to hit "approval.route" (because both "approval" and "routing"→"route" stem-match). More complex; requires a stemmer or fuzzy match.

**Recommend Option A** — least code, reads naturally, and the TRY-chip affordance becomes self-evident: click "approval" → results filter to approval skills.

## Acceptance criteria

- [ ] All 6 TRY chips return ≥1 result when clicked.
- [ ] The chip text reads naturally to a non-technical operator.
- [ ] No regression on the typed-search beat (`fix-skills-marketplace-search-seed-competitor-pricing` covers the literal runbook query).
- [ ] Component test (`Skills.test.tsx` or extend) asserts each chip yields ≥1 result against the current seed set.

## May-touch

- `web/app/(app)/skills/page.tsx:23-30` (TRY_CHIPS array)
- `web/lib/api/skills.ts:64-143` (only under Option B)
- `web/components/skycot/skills/Skills.test.tsx` (assert chip → ≥1 result)

## Out of scope

- Seed wording for the runbook's literal `competitor pricing scrape` query (covered by `fix-skills-marketplace-search-seed-competitor-pricing`)
- Marketplace count pill misleading-state copy (covered by `fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage`)
- Install button on detail page (covered by `fix-skills-detail-install-button-and-flow`)

## Effort

Option A: ~6 LOC, ~10 min. Option B: ~30 LOC. Option C: ~50-80 LOC. P0 because every fallback path during Scene 4 currently shows zero results.
