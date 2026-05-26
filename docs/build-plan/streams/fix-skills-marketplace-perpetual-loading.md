# fix-skills-marketplace-perpetual-loading

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-marketplace-perpetual-loading`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F4 [P0] + whole-site-mvp-claims-coverage F12 (seed-side) + scene-4-and-5-claim-4-and-5 F2.

🔴 **P0 DEMO-BLOCKER — `/skills` shows "0 skills · 0 shown" in the hero count pill and "Loading skills from the marketplace..." in the FEATURED SKILLS panel indefinitely (still loading at 4s, still loading at 8s). Scene 4 (Claim 4 — skills shared + learned) cannot be demoed — the search → install → worker-uses-skill flow has nothing to search for.**

## Root cause

`/skills` calls the gateway at `/api/v1/skills`. Two coupled defects:

1. **The gateway returns `{"skills":[],"total_count":0,"tenant_id":"..."}`.** The demo tenant's skill catalogue is empty. PR #1682 wired the runtime side of skill-library loading, but no demo-seed step populates the marketplace listing for the demo tenant.
2. **The SPA never resolves the loading state when the response is `{ skills: [] }`.** The "Loading skills from the marketplace..." copy appears to be gated on `skills.length === 0` OR on `isLoading` — when the gateway returns an empty array, the FEATURED SKILLS panel never flips into the empty-state copy. The count pill correctly shows "0 skills · 0 shown" but the loading spinner stays.

The fix has both a seed-side and a SPA-side leg. Without skills seeded, even after the SPA properly handles the empty response, the page is a useless wall of "no skills available".

## Findings this brief closes

- chrome-live-walk-and-playwright F4 [P0] — Skills marketplace shows 0 skills perpetual loading
- scene-4-and-5-claim-4-and-5 F2 [P0] — "competitor pricing scrape" search returns 0 results (subset — needs seeded skills)
- whole-site-mvp-claims-coverage F12 (partial, skills side) — live seed has 0 skills

## Fix recipe

### Seed side (gateway)

1. **Add a demo-mode skill seed step** to the gateway boot (when `ARGO_DEMO_MODE=1`) that populates ≥6 marketplace skills with one matching the runbook's "competitor pricing scrape" beat. Candidates (some already in `web/lib/api/skills.ts:63-134` SEED_SKILLS as fallback):
   - `pricing.competitor_scrape` — Web research skill (matches Scene 4 narration)
   - `approval.route` — Approval routing
   - `invoice.extract` — Doc extraction
   - `candidate.screen` — Recruiting
   - `deal.qualify` — Sales qualification
   - `support.triage` — Support
   - `lead.enrich` — Sales enrichment
   - Plus 2-3 "learned this month" skills to populate the learned-skills strip

2. **Wire the seed into demo bootstrap.** Likely path: `crates/argo-demo-seed/src/lib.rs` or a new `skill.rs` sibling to the existing `workflow.rs`. When `ARGO_DEMO_MODE=1`, fire on gateway start.

### SPA side

3. **Fix the perpetual-loading state.** Locate `web/app/(app)/skills/page.tsx` (or the FEATURED panel component). The loading flag must:
   - Be `true` only while the fetch is in-flight (`isLoading`, not derived from `skills.length === 0`)
   - Flip to `false` when the response resolves regardless of array length
   - When `!isLoading && skills.length === 0`, render the empty-state CTA ("No skills installed yet — browse the marketplace") instead of the spinner

4. **Add a Playwright spec** `web/playwright/skills/marketplace-loads-and-paints.spec.ts`:
   - With demo mode on, navigate to `/skills`
   - Wait for `[data-testid="skills-loading"]` to disappear within 2s
   - Assert at least 1 skill card is visible
   - Assert count pill text matches `N skills · N shown` with N ≥ 6

## Acceptance criteria

- [ ] `/skills` paints ≥6 skill cards within 2 seconds in demo mode
- [ ] Hero count pill reads `6+ skills · 6+ shown`
- [ ] The "Loading skills from the marketplace..." copy is gone after the first paint
- [ ] Search "competitor pricing scrape" returns ≥1 result
- [ ] New Playwright spec passes
- [ ] Cold rerun of any claim-4 integration test still passes

## May-touch

- `crates/argo-demo-seed/src/skill.rs` (NEW or in existing lib.rs)
- `crates/argo-demo-seed/src/lib.rs` (wire the new seed step)
- `web/app/(app)/skills/page.tsx` (loading-state fix)
- `web/components/skycot/skills/FeaturedSkillsPanel.tsx` (or equivalent) — loading vs empty-state branch
- `web/playwright/skills/marketplace-loads-and-paints.spec.ts` (NEW)
- `web/lib/api/skills.ts` (SEED_SKILLS fallback — already exists, may need pricing.competitor_scrape entry)

## Out of scope

- `/skills/<id>` install affordance (covered by `fix-skills-detail-install-button-and-flow`)
- Hero count pill cosmetic "N of M" vs "M shown" wording (covered by `fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage`)
- Workflow seed (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)

## Effort

~200-400 LOC (seed crate + SPA fix + spec). ~6-10 hours. P0 because Scene 4 is unrecordable without it.
