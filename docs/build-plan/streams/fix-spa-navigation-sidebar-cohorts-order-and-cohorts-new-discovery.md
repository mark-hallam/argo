# fix-spa-navigation-sidebar-cohorts-order-and-cohorts-new-discovery

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-navigation-sidebar-cohorts-order-and-cohorts-new-discovery`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F19 [P2] + F23 [P3].

🟡 **P2 — Sidebar nav puts "Cohorts" last (after Workflows) even though the locked D3 design positions cohorts as a primary surface. `/cohorts/new` is only reachable from the cohorts empty-state, not from a top-of-page button when cohorts already exist.**

## Root cause

- `web/components/skycot/nav-items.tsx:22-30` orders Approvals / Team / Workers / Audit / Skills / Workflows / Cohorts. Cohorts is last.
- `grep -rn 'href="/cohorts/new"' web/components web/app` finds only the empty-state CTA.

## Findings this brief closes

- whole-site-mvp-claims-coverage F19 [P2] — /cohorts/new not discoverable when cohorts exist
- whole-site-mvp-claims-coverage F23 [P3] — sidebar Cohorts ordering

## Fix recipe

1. **Reorder sidebar** in `web/components/skycot/nav-items.tsx:22-30` to: Approvals / Team / Workers / Cohorts / Skills / Workflows / Audit. (Operator-confirm — alternative: Workers / Cohorts / Workflows clustering.)
2. **Add "+ New cohort" button in `/cohorts` page header** alongside the dept tabs, visible regardless of cohort count.
3. **Web test** asserts both: sidebar ordering matches; "+ New cohort" button present on /cohorts.

## Acceptance criteria

- [ ] Sidebar order matches operator-canonical sequence.
- [ ] `/cohorts` page header has visible "+ New cohort" button regardless of cohort count.
- [ ] Web test covers both.
- [ ] No regression on existing cohorts empty-state CTA.

## May-touch

- `web/components/skycot/nav-items.tsx:22-30` (reorder)
- `web/app/(app)/cohorts/page.tsx` (add header button)
- `web/tests/nav-sidebar-order.test.ts` (NEW)
- `web/tests/cohorts-page-new-button.test.ts` (NEW)

## Out of scope

- Cohorts detail rendering (separate)
- Dept registry alignment (`fix-cohorts-dept-registry-design-lock-drift-6-vs-8`)

## Effort

~25-40 LOC + 2 tests. ~45min.
