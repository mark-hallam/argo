# fix-changelog-this-week-date-rail-auto-refresh

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-changelog-this-week-date-rail-auto-refresh`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F13 (P2).

🟡 **P2 — `/changelog` "this week" header reads `May 18 – May 20, 2026` even though today is May 27. The page-leading rail says "What we shipped this week" with a date range that is 7 days stale. Investors and SOC2 reviewers tend to read changelog freshness as a proxy for product velocity — stale ranges are a credibility hit.**

## Root cause

`/changelog` hard-codes the "this week" date range, likely in either `web/app/changelog/page.tsx` or a feeder JSON / MDX inventory file under `web/content/changelog/`. The author intended manual maintenance ("update the range when a new entry ships"); the maintenance has slipped 7 days. A diligent operator might catch this on every weekly release; in practice, nobody does.

Two ways to fix:
1. **Auto-derive** — pull the most-recent changelog entry's date dynamically and label the rail "Latest" or "This sprint" (better-loaded language that doesn't promise "this calendar week").
2. **Static refresh with a CI check** — author a `scripts/check-changelog-freshness.sh` that fails if the date in the rail is >14 days behind the most-recent entry, and add it to the brief-lint pre-commit.

Option 1 is more durable; Option 2 catches it at commit time. Combine for v1.1.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F13 [P2] — `/changelog` "this week" header reads May 18 – May 20, 2026 even though today is May 27

## Fix recipe

1. Find the date-range hardcoding: `rg "May 18 – May 20" web/`.
2. Replace with a dynamic derivation:
   ```tsx
   const latestEntryDate = changelogEntries[0].date;
   const label = "Latest"; // or "Recent" — better-loaded than "This week" which implies fixed cadence
   ```
3. Render: `Latest · ${formatDate(latestEntryDate)}`.
4. Update the visual: keep the rail style; the text content goes from "What we shipped this week · May 18 – May 20" to "What we shipped recently · May 25, 2026" (or whatever the latest entry's actual date is).
5. (Optional) Add `scripts/check-changelog-freshness.sh` that asserts the rail text matches `changelogEntries[0].date` and pre-commit-hooks it.

## Acceptance criteria

- [ ] `/changelog` rail date matches the most-recent entry's date dynamically.
- [ ] No hardcoded date ranges in `web/app/changelog/page.tsx` or feeder data.
- [ ] On the next changelog entry ship, the rail updates without manual edit.

## May-touch

- `web/app/changelog/page.tsx` (~20 LOC dynamic derivation)
- (potentially) `web/content/changelog/` feeder JSON / MDX (~5 LOC)
- (optional) `scripts/check-changelog-freshness.sh` (NEW — ~30 LOC)

## Out of scope

- Pagination for the 96-entry list (covered by `fix-changelog-pagination-for-96-entries`).
- Mobile breakpoints (covered by `fix-marketing-pages-compare-changelog-mobile-breakpoints`).
- The changelog content itself.

## Effort

XS — ~30 LOC + optional CI check. ~30 minutes. P2 because the page is marketing-surface (camera-adjacent but the demo doesn't navigate here) and the staleness signals a maintenance gap to anyone who notices.
