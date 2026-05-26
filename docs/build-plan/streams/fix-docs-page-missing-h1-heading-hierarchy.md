# fix-docs-page-missing-h1-heading-hierarchy

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-docs-page-missing-h1-heading-hierarchy`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F14 [P1].

🟡 **P3 — `/docs` has no `<h1>` — only `<h3>` card titles ("Getting Started", etc.). SEO + accessibility regression; screen readers can't find the document title.**

## Root cause

`/usr/bin/curl -s http://127.0.0.1:3000/docs | grep -oE '<h1[^>]*>'` returns empty. The "Docs" wordmark in `<head>` doesn't appear in the body.

## Findings this brief closes

- whole-site-mvp-claims-coverage F14 [P1] (downgraded to P3 — small SEO regression, no demo impact) — /docs missing h1

## Fix recipe

1. **Add `<h1>SKYCOT Documentation</h1>` above the card grid** in `web/app/docs/page.tsx` (or wherever the page renders).
2. **Style** to match the F1 editorial-center register if applicable, or simply use the standard page-title type style.
3. **Web test** asserts /docs returns HTML with exactly one h1.

## Acceptance criteria

- [ ] `curl /docs | grep '<h1'` returns one match.
- [ ] Screen reader and lighthouse pass.
- [ ] No visual regression.

## May-touch

- `web/app/docs/page.tsx` (add h1)
- `web/tests/docs-h1.test.ts` (NEW)

## Out of scope

- Full docs SEO sweep
- Docs content authoring

## Effort

~5-15 LOC + 1 test. ~15min.
