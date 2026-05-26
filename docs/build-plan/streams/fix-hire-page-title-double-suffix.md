# fix-hire-page-title-double-suffix

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-hire-page-title-double-suffix`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F17 [P2].

🟡 **P2 — `/hire` page title double-suffixes: "Hire — SKYCOT · SKYCOT". Same shape as `/for/*` issue. The page-level metadata emits "Hire — SKYCOT", then the global template adds ` · SKYCOT`.**

## Root cause

`web/app/(hire)/hire/page.tsx` metadata sets `title: "Hire — SKYCOT"` instead of `title: "Hire"`. Global template at root layout appends ` · SKYCOT`. Result: "Hire — SKYCOT · SKYCOT".

## Findings this brief closes

- whole-site-mvp-claims-coverage F17 [P2] — `/hire` page title double-suffix

## Fix recipe

1. **Drop the ` — SKYCOT` suffix** in the page-level metadata at `web/app/(hire)/hire/page.tsx`:
   ```tsx
   // before
   export const metadata = { title: "Hire — SKYCOT" };
   // after
   export const metadata = { title: "Hire" };
   ```

2. **Verify**: `curl /hire | grep title` → expect `Hire · SKYCOT`.

3. **Audit related `/hire/step/{n}` pages** for the same pattern.

## Acceptance criteria

- [ ] `curl /hire | grep title` returns `Hire · SKYCOT` (no double-suffix)
- [ ] `/hire/step/{1..5}` follow the same pattern
- [ ] Playwright assertion passes

## May-touch

- `web/app/(hire)/hire/page.tsx`
- `web/app/(hire)/hire/step/[step]/page.tsx`

## Out of scope

- Other tab-title bugs (covered by sibling briefs)
- Hire wizard logic (separate concern)

## Effort

~5-10 LOC. ~15 min. P2 polish.
