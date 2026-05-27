# fix-workflow-viewer-builder-skeleton-only-ssr-no-generatemetadata

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-viewer-builder-skeleton-only-ssr-no-generatemetadata`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-4-and-5-claim-4-and-5.md` F8 [P1].

🟠 **P1 — Workflow viewer + builder canvas SSR (Server-Side Render — the HTML Next.js returns before React client takes over) return skeleton-only HTML. `<title>` renders "Workflows · SKYCOT" not the workflow name. The body contains only `workflow-viewer-skeleton-step-0..4` testids. The skill detail OG (Open Graph — metadata format social platforms read for link previews) title leaks the parent route's metadata ("Skill marketplace — SKYCOT") instead of the specific skill name. SEO + share-link previews are broken; cold-load shows skeleton flash for 1-2s on camera.**

## Root cause

`/workflows/sales/qualify_lead` and `/workflows/sales/qualify_lead/build` are client-rendered (probably `"use client"` at the top). They have no `generateMetadata` export — so SSR stamps the title from the parent route's metadata ("Workflows · SKYCOT" for the viewer; presumably "Workflow builder · SKYCOT" for the builder).

The skill detail page (`/skills/[skillId]`) does render the skill name in `<title>` (correct), but `og:title` still says "Skill marketplace — SKYCOT" (parent metadata override missing).

Why it matters:
- **Cold-load skeleton flash on camera**: 1-2s of skeleton-only HTML on Scene 5's first paint, before the client renders the actual workflow viewer.
- **Share-link previews**: any operator pasting the URL into Slack / Teams / email shows a generic SKYCOT card instead of the specific workflow.
- **SEO**: search-engine results show the parent route's title instead of the specific page.

Evidence:
- `curl -s http://127.0.0.1:3000/workflows/sales/qualify_lead | grep -oE '<title>[^<]+</title>'` → `<title>Workflows · SKYCOT</title>`
- `curl -s http://127.0.0.1:3000/skills/pricing.competitor_scrape | grep -oE 'og:title[^>]*content="[^"]*"'` → `og:title" content="Skill marketplace — SKYCOT"`

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F8 [P1] — Workflow viewer + builder canvas SSR return skeleton-only HTML; page title is the section name not the workflow name; skill detail OG title leaks parent route

## Fix recipe

1. **Add `generateMetadata` to `/workflows/[verticalId]/[name]/page.tsx`.** This is a Next.js App-Router pattern: export an async function that fetches the workflow name (from gateway or seed fallback) and returns `{ title, description, openGraph: { title, description } }`. Server-rendered before the client component takes over.
2. **Repeat for `/workflows/[verticalId]/[name]/build/page.tsx`** — same pattern, title is "Editing {name} · SKYCOT" or similar.
3. **Add `generateMetadata` to `/skills/[skillId]/page.tsx`.** Resolve the skill name + description; stamp both `title` and `openGraph.title` + `openGraph.description`. Default the OG image to the locked brand asset.
4. **Coordinate with `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`** — that brief covers the (app)/layout.tsx default title leak; this brief covers per-route generateMetadata. They are independent and additive.
5. **Pre-warm**: update `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist to include `/workflows/sales/qualify_lead` so the camera doesn't catch the 1-2s skeleton flash on Scene 5 cold-load.
6. **Vitest pin** — assert the metadata function returns the workflow name + description for a known seed workflow.

## Acceptance criteria

- [ ] `curl -s http://localhost:3000/workflows/sales/qualify_lead | grep -oE '<title>[^<]+</title>'` returns a title containing "Qualify lead" (or the workflow name), not "Workflows · SKYCOT".
- [ ] Same page's `og:title` contains the workflow name.
- [ ] `curl -s http://localhost:3000/skills/pricing.competitor_scrape | grep og:title` returns the skill name, not "Skill marketplace".
- [ ] Cold-load of the workflow viewer shows the title in browser tab from first paint.
- [ ] Pre-record runbook includes `/workflows/sales/qualify_lead` (or the chosen Scene-5 anchor) as a pre-warm route.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (add `generateMetadata`)
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (add `generateMetadata`)
- `web/app/(app)/skills/[skillId]/page.tsx` (add `generateMetadata`)
- `web/lib/api/workflows.ts` or `web/lib/api/skills.ts` (helper to resolve name without full body fetch)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-warm list)

## Out of scope

- The (app) layout default-title leak (covered by `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`)
- Workflow viewer's body-side rendering (out of scope; the page is intentionally client-rendered for the canvas)
- Skill detail Install button (covered by `fix-skills-detail-install-button-and-flow`)

## Effort

S — ~80-150 LOC across 3 routes + 1 helper + 1 runbook update + tests. ~1-2 hours. P1 because the v1.0 demo MP4 records the workflow viewer cold; the skeleton flash + wrong tab title are visible polish gaps.
