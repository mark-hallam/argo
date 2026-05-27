# fix-mariah-memory-tab-facts-mismatch-marketplace-role

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-mariah-memory-tab-facts-mismatch-marketplace-role`
**Source:** UX e2e audit 2026-05-28-0252 (scene-1-and-2 F07)

🟠 **P1 — Mariah's role is "lists and re-prices items for sale" / Marketplace. Her Memory tab shows 3 of 5 facts about VENDOR PROCUREMENT ("Vendor onboarding requires Compliance review for any vendor over $50K/year spend", "Annual renewals route to Procurement + the business owner", "Use 'strategic partners' not 'vendors' for Tier 1 vendors per Operations branding"). Only 1 fact ("Photos required for every listing") matches her role. The camera lingering on her Memory tab during Scene 1 will read procurement-policy strings on a marketplace worker.**

## Root cause

`web/lib/sample-worker-memory.ts` carries Mariah's facts list. The fixture appears to have been copy-pasted from a procurement-persona seed (likely a Patrick / Peta cohort sketch) without re-writing the strings to match Mariah's role. The fact list is keyed by worker UUID, so the wrong content is bound to the right worker — not a routing bug, a content bug.

Audit observation (reading the 5 facts on `/workers/018f67d0-0000-7000-8000-000000000001/memory`):

- "Vendor onboarding requires Compliance review for any vendor over $50K/year spend" ← procurement
- "Annual renewals route to Procurement + the business owner" ← procurement
- "Use 'strategic partners' not 'vendors' for Tier 1 vendors per Operations branding" ← procurement
- "Floor price is the cost-basis plus 18% margin" ← listings (only role-matching)
- "Photos required for every listing — minimum 3, maximum 8" ← listings (only role-matching)

3 of 5 strings are procurement, 2 are listings. The operator narrating Scene 1 — "she remembers the photo standard, she remembers the floor-price rule" — has to skip past 3 unrelated facts to get to the on-message ones.

Sibling `fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface` (IN_REVIEW #2556) handles cross-surface persona drift but the audit indicates that brief does NOT touch the Memory tab facts string content — verify before claim.

## Findings this brief closes

- scene-1-and-2 F07 [P1] — Mariah's Memory tab facts mismatch her marketplace listing role

## Fix recipe

1. **Locate the Mariah fact list** in `web/lib/sample-worker-memory.ts`. Confirm by grep:
   ```
   rg "Vendor onboarding requires Compliance review" web/lib/
   ```
2. **Rewrite 3 of 5 facts to match the marketplace-listing role**. Suggested replacements (operator can adjust):
   - "Floor price is the cost-basis plus 18% margin" ← keep
   - "Photos required for every listing — minimum 3, maximum 8" ← keep
   - "Competitor scrape runs every Monday 06:30 against 3 anchor stores; re-price triggered when undercut by >5%" ← NEW
   - "Listing copy follows the 'short bullet, longer body, never markdown' pattern per Ops branding" ← NEW
   - "Returns over $200 require Compliance review; under $200 are auto-credited per the Trust policy" ← NEW
3. **Add 1 cross-policy fact** ("Returns over $200 …") so the operator narrating Scene 1 has a natural pivot to mention compliance + audit without needing to switch tabs. This is the right place to fold in compliance flavour without misaligning the role.
4. **Audit the other 4 workers** (Peta, Priya, Jamal, Nori) for similar role-fact drift. Out of scope to fix all here — open follow-up briefs only for ones that are camera-visible in the runbook scenes.
5. **Add a vitest assertion** at `web/lib/sample-worker-memory.test.ts` (NEW or extend) that pins the 5 Mariah facts as containing at least one of `["listing", "price", "photo", "scrape", "Ops"]` and zero of `["Vendor onboarding", "Procurement", "strategic partners", "Tier 1"]`.

## Acceptance criteria

- [ ] At least 4 of Mariah's 5 facts mention listings, pricing, photos, scraping, or the Ops branding — not procurement.
- [ ] No fact mentions "Vendor onboarding", "Procurement", "Tier 1 vendors", or "strategic partners".
- [ ] vitest pins the role-aligned-ness assertion.
- [ ] On `/workers/018f67d0-0000-7000-8000-000000000001/memory`, the camera reads 5 facts that all make sense for "she lists and re-prices items".
- [ ] PR body cites scene-1-and-2 F07.

## May-touch

- `web/lib/sample-worker-memory.ts` (~10 LOC string replacements)
- `web/lib/sample-worker-memory.test.ts` (NEW or extend, ~40 LOC)

## Out of scope

- Real-data memory facts (covered by future `fix-worker-memory-fetch-real-gateway`)
- Other workers' memory fact drift (open follow-up briefs only where camera-visible)
- Persona traits + skills cross-surface drift (covered by `fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface` IN_REVIEW #2556)

## Effort

XS — ~10 LOC string fix + ~40 LOC test. ~30-45 min including a Scene-1 re-watch to confirm the camera lingers.
