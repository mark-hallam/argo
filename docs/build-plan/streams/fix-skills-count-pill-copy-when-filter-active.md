# fix-skills-count-pill-copy-when-filter-active

**Owner**: claude.
**Authored against**: origin/main `ea161df3a` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-count-pill-copy-when-filter-active`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-4-and-5-claim-4-and-5.md` F9 [P2].

🟡 **P2 — The skills page count pill at `web/app/(app)/skills/page.tsx:113-114` renders `{skills.length} skills · {filtered.length} shown`. When a filter is active and yields 0 matches, this reads as `"24 skills · 0 shown"` — which the audit notes parses awkwardly (the reader sees "24 skills" and wonders where they went, instead of "filter matched 0 of 24").**

The fix is to reverse the copy direction when filtered < total: lead with the filtered count, qualify with the total, and surface the active filter clearly.

## Findings this brief closes

- scene-4-and-5 F9 [P2] — Skills page empty-state copy still says "6 skills · 0 shown" instead of "0 of 6"

## Fix recipe

1. **Locate** `web/app/(app)/skills/page.tsx:113-114` (the `<span className="count-pill" data-testid="skills-count">` block).

2. **Reverse the copy direction conditionally**:
   ```tsx
   <span className="count-pill" data-testid="skills-count">
     {filtered.length === skills.length
       ? <>{skills.length} skills</>
       : <>{filtered.length} of {skills.length} skills</>
     }
   </span>
   ```
   - When no filter is active: `"24 skills"` (clean, no redundant "shown")
   - When filter is active with matches: `"5 of 24 skills"`
   - When filter is active with 0 matches: `"0 of 24 skills"` (clearly communicates the filter excluded everything)

3. **If a filter chip / search bar is visible above**, no additional copy needed — the user can see the filter and the `"0 of 24"` count together tells the story.

4. **Add / update vitest spec** at `web/app/(app)/skills/__tests__/page.test.tsx`:
   - With no filter applied: assert count pill text matches `/^\d+ skills$/`
   - With a filter that excludes all rows: assert count pill text matches `/^0 of \d+ skills$/`
   - With a partial-match filter: assert count pill text matches `/^\d+ of \d+ skills$/`

## Acceptance criteria

- [ ] No-filter case: `"24 skills"` (or whatever the total is)
- [ ] Partial-filter case: `"N of M skills"` where N < M
- [ ] Zero-match case: `"0 of M skills"` (instead of `"M skills · 0 shown"`)
- [ ] vitest spec covers all 3 cases
- [ ] `pnpm --dir web exec vitest run app/\(app\)/skills` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/app/(app)/skills/page.tsx` (~10 LOC)
- `web/app/(app)/skills/__tests__/page.test.tsx` (NEW or extended, ~30 LOC) — verify whether a `__tests__` dir exists under skills first; if not, place at `web/__tests__/skills/page.test.tsx` matching existing convention

## Out of scope

- The skill seed count itself (covered by `fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage` #2373, padding to 24)
- Search/filter chip styling (locked design)
- The skills-marketplace perpetual-loading bug (covered by `fix-skills-marketplace-perpetual-loading`)

## Effort

~15 LOC change + ~30 LOC test. ~30 minutes. P2 polish; small copy fix with clear acceptance.
