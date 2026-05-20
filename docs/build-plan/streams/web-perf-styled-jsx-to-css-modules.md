# web-perf-styled-jsx-to-css-modules — migrate styled-jsx blocks to CSS modules

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-perf-styled-jsx-to-css-modules`.
**Priority**: P2 — wave-2 perf audit "single highest-impact fix" prerequisite. styled-jsx pins 6 marketing pages to client components (verified — see `feedback_styled_jsx_requires_use_client.md` in operator memory). Migration enables server rendering, SSR styling, no FOUC, smaller client bundle.
**Sizing**: ~800-1500 LOC (~6 pages × ~100-200 LOC of CSS-module rewrites + tests).

## Why this brief exists

From `~/.claude/projects/.../memory/feedback_styled_jsx_requires_use_client.md`:

> styled-jsx REQUIRES "use client" — Next.js will return HTTP 500
> if you remove the directive from a page that uses `<style jsx>`.
> The marketing pages are client components whether we like it or
> not until this migration lands. Per wave-2 perf:
>
> > Single highest-impact fix: Add Audiowide via next/font/google
> > in app/layout.tsx and delete the 19 duplicate Google-Fonts
> > <link> blocks across 13 page files — eliminates ~18
> > render-blocking roundtrips on the demo path.
>
> But the prerequisite is replacing styled-jsx with a server-safe
> CSS strategy: CSS modules or Tailwind. This brief is that prereq.

## Goals

1. Migrate all `<style jsx>` and `<style jsx global>` blocks to CSS modules (`*.module.css`).
2. After migration, remove `"use client"` from pages that no longer need it (the 6 marketing pages from the perf audit + sibling pages that are pure presentation).
3. Verify each migrated page still renders identical visuals (zero pixel-diff).

## Steps

1. Inventory `<style jsx>` callsites: `rg '<style jsx' web/`.
2. Per page, copy the CSS into a sibling `.module.css` file, replace styled-jsx tags with `import styles from './page.module.css'` + className references.
3. Convert dynamic style values (template-string CSS) into CSS custom-property props passed via `style={{ '--foo': bar }}` if necessary.
4. After migration, remove `"use client"` from pages with zero hook callsites.
5. Verify pages still render 200 + no FOUC + identical visuals.

## may-touch

- All page.tsx files currently using `<style jsx>`: /landing, /compare, /customers, /docs, /changelog, /pricing, /workflows, /workflows/templates, /skills, /privacy, /terms, /security
- Sibling new `.module.css` files
- `web/app/globals.css` (only if helper tokens need extracting)

## must-NOT-touch

- Tailwind config / utility-first classes (this is CSS modules in addition, not instead)
- BrandLoudHero (server-component-incompatible currently; out of scope)
- next.config.ts CSS handling (Next 15 ships CSS-module support by default)

## Acceptance criteria

- [ ] Zero `<style jsx>` blocks remain in `web/app/`.
- [ ] At least 4 marketing pages successfully converted to server components (no "use client" + no 500).
- [ ] Zero pixel-diff per page (manual screenshot + Playwright visual regression smoke).
- [ ] Bundle size: client JS for /landing reduces by ≥30% (measure via `pnpm build && du -sh .next/static`).
- [ ] `pnpm typecheck && pnpm lint && pnpm test && pnpm build` all clean.

## References

- Audit: `docs/skycot/audit/perf/2026-05-20-wave-2-perf.md`
- Memory: `~/.claude/.../feedback_styled_jsx_requires_use_client.md`
