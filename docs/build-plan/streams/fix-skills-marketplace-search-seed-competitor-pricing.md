# fix-skills-marketplace-search-seed-competitor-pricing

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-marketplace-search-seed-competitor-pricing`.
**Source**: e2e-ux-audit Agent 3 findings F2 + F11 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🔴 **P0 DEMO-GATE — Scene 4 hero search beat narrates "three results" for a query that returns zero.**

## Root cause

Scene 4 step 2 narration: "Operator searches **competitor pricing scrape**. Three results." Today:

- Live gateway: `GET /skills` returns `{count:0, skills:[]}` (so the SPA falls through to the seed).
- SPA seed (`web/lib/api/skills.ts:142`): 6 skills, none of which match "competitor", "pricing", or "scrape" in name, description, or layer:
  - approval.route, invoice.extract, candidate.screen, deal.qualify, support.triage, lead.enrich

Result: page shows "No skills match 'competitor pricing scrape'. Try a different search or clear the filter." The runbook's three-result fan-out doesn't exist.

Secondary defect (`web/app/(app)/skills/page.tsx` count pill area): empty-search state shows `6 skills · 0 shown` in the eyebrow + `No skills match "<q>"` in the body — two redundant phrasings of the same empty state.

## Findings this brief closes

- Agent 3 F2 [P0] — "competitor pricing scrape" search returns 0 results, not 3
- Agent 3 F11 [P2] — Skills page hero count says "6 skills · 0 shown" during empty search (redundant phrasing)

## Fix recipe

1. **Seed three skills that match "competitor pricing scrape"** into both `SAMPLE_SKILLS` (`web/lib/api/skills.ts:142`) AND the gateway skill library. Suggested set (final names + descriptions are dressing-side; align with the operator's narration):
   - `competitor.pricing.scrape` — "Scrape competitor pricing from public catalogs"
   - `market.intelligence.scan` — "Aggregate market-pricing signals weekly"
   - `pricing.benchmark` — "Compare your pricing to verified industry benchmarks"
   
   Provenance + risk fields per the existing seed-skill schema. Each must be tagged so the marketplace search hits any of "competitor", "pricing", or "scrape" tokens.

2. **Verify gateway-side seed.** The skills library load is wired per `crates/argo-agent-runner/src/turn_helpers.rs:65` per the memory `project_v1_ship_blocker_skill_library_load`. Make sure the demo-seed CLI populates the 3 new skill manifests into the per-tenant skill library so a live curl `GET /api/v1/skills?q=competitor` returns 3 results.

3. **Fix the redundant empty-state phrasing** at `web/app/(app)/skills/page.tsx` (count-pill JSX, around the eyebrow row):
   - When `filtered.length === 0 && query.length > 0`: show `0 of {total}` in the pill, drop the second clause from the body, OR
   - Keep the body message and remove the redundant count pill in this state.

## Acceptance criteria

- [ ] Typing "competitor pricing scrape" in the `/skills` hero search returns ≥3 matching results.
- [ ] Each of those results has a working `/skills/<id>` detail page (Install button is a separate brief).
- [ ] curl-test: `GET /api/v1/skills?q=competitor` returns ≥1 result on the live gateway (demo tenant).
- [ ] Empty-search state shows ONE consistent message, not "6 skills · 0 shown" + "No skills match".
- [ ] Component test (`Skills.test.tsx`) pinning the search beat.

## May-touch

- `web/lib/api/skills.ts` (seed catalog at `:142`)
- `web/app/(app)/skills/page.tsx` (count-pill empty-state phrasing)
- `crates/argo-cli/src/cli/demo_seed/skills*.rs` (gateway-side seed)
- `crates/argo-skill/manifests/competitor-pricing-scrape.yaml` (NEW — skill manifests)
- Sibling skill manifests for the other two seeded skills.

## Out of scope

- Install button + flow on `/skills/<id>` (separate brief: `fix-skills-detail-install-button-and-flow`)
- Gateway routing `/skills` vs `/api/v1/skills` + CORS preflight (separate brief: `fix-skills-gateway-route-prefix-and-cors-preflight`)
- Talk-track rewrite to search for an existing skill (rejected — the runbook's narration is locked)

## Effort

~50-100 LOC SPA + 3 skill manifests + seed CLI integration. ~1-2 hours.
