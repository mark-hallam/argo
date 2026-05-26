# fix-team-page-duplicate-h1-and-settings-shell-h1-hierarchy

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-duplicate-h1-and-settings-shell-h1-hierarchy`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/whole-site-mvp-claims-coverage.md` F11 + F16.

🟡 **P1 — Two related heading-hierarchy bugs that fail WCAG semantic-HTML guidance. (1) `/team` renders 2 `<h1>` elements on one page (line 55 + line 250). (2) `/settings/*` shell renders a "Settings" `<h1>` AND each sub-page (api-keys, data-export, members, security) renders its own page-level `<h1>` — 2 H1s per render. The 7 sub-pages WITHOUT their own H1 (billing, connectors, knowledge, notifications, notify-channels, providers, workspace) have only the shell H1 — no clear top-level heading for their content. Screen-reader heading-nav misbehaves on both surfaces.**

## Root cause

**(1) `/team` 2 H1s.** Per audit `grep -n '<h1' web/app/(app)/team/page.tsx`: 2 H1s — line 55 `Team` + line 250 (same class). The second H1 is presumably a section heading that should be `<h2>` (or `<h3>` depending on hierarchy).

**(2) Settings shell + sub-page double-H1.** `web/app/(app)/settings/layout.tsx` (the settings shell) renders a `<h1>Settings</h1>` element AND each settings sub-page renders its own `<h1>` for the sub-page name. Result on `/settings/api-keys`: two H1s ("Settings" + "API Keys"). Result on `/settings/connectors`: one H1 ("Settings") because the sub-page has no own H1 — but no clear sub-page heading.

Both bugs share the same fix shape: enforce a single H1 per page that names the most-specific surface, demote shell H1s to H2 (or a non-heading `role="banner"` element).

## Findings this brief closes

- whole-site-mvp-claims-coverage F11 [P1] — `/team` has 2 `<h1>` elements on one page
- whole-site-mvp-claims-coverage F16 [P2] — `/settings/*` sub-pages render duplicate "Settings" `<h1>` from shell + per-page H1

## Fix recipe

**Part 1 — `/team`:**

1. In `web/app/(app)/team/page.tsx`, locate the 2 H1s at lines 55 and 250.
2. Determine which H1 is the page-level heading (presumably line 55 — "Team" as the visible title). Keep that one.
3. Convert the secondary H1 (line 250) to `<h2>` or `<h3>` depending on the visual hierarchy. The class can stay the same if Tailwind classes are typographic, not semantic.

**Part 2 — Settings shell:**

4. In `web/app/(app)/settings/layout.tsx`, replace the shell `<h1>Settings</h1>` with a `<header role="banner">` element that renders the same visual treatment but is no longer a heading element. OR demote to `<h2>` (depends on whether the settings tab strip is considered the primary navigation).
5. Audit each `/settings/*` sub-page. Ensure each one has exactly one `<h1>` naming the sub-page (e.g. "API Keys", "Connectors"). The 7 sub-pages currently lacking an H1 need one added.
6. Verify the rendered DOM via Playwright pin: `await expect(page.locator('h1')).toHaveCount(1)` on each settings sub-page.

**Part 3 — Sweep other in-app pages** for sibling violations. `rg -c '<h1' web/app/(app)/**/page.tsx` and inspect anything with count >1.

## Acceptance criteria

- [ ] `/team` renders exactly one `<h1>` element.
- [ ] Every `/settings/*` sub-page renders exactly one `<h1>` element. The shell-level "Settings" label is either `<h2>` or `<header role="banner">`.
- [ ] The 7 currently-H1-less settings sub-pages (billing, connectors, knowledge, notifications, notify-channels, providers, workspace) have a meaningful page-level `<h1>` added.
- [ ] Playwright pin walks all relevant pages and asserts H1 count is 1.
- [ ] No regression on the locked settings shell visual treatment (still looks like the locked D top-tabs design).

## May-touch

- `web/app/(app)/team/page.tsx` (demote 2nd H1)
- `web/app/(app)/settings/layout.tsx` (shell H1 → H2 or banner)
- `web/app/(app)/settings/billing/page.tsx` (add H1)
- `web/app/(app)/settings/connectors/page.tsx` (add H1)
- `web/app/(app)/settings/knowledge/page.tsx` (add H1)
- `web/app/(app)/settings/notifications/page.tsx` (add H1)
- `web/app/(app)/settings/notify-channels/page.tsx` (add H1)
- `web/app/(app)/settings/providers/page.tsx` (add H1)
- `web/app/(app)/settings/workspace/page.tsx` (add H1)
- `web/__tests__/heading-hierarchy.spec.ts` (new Playwright pin)

## Out of scope

- The settings sub-page metadata gaps (covered by `fix-settings-sub-pages-add-missing-metadata`)
- The `/billing/subscribe` H1 (covered by `fix-billing-subscribe-missing-h1-and-metadata.md`)
- Marketing-page H1 hierarchy (separate scope)

## Effort

S — ~50-80 LOC across 9 files + 1 pin. ~1-2 hours. P1 because screen-reader nav is broken on the settings and team surfaces.
