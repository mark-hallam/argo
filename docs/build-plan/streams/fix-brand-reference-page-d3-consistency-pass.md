# fix-brand-reference-page-d3-consistency-pass

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-brand-reference-page-d3-consistency-pass`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F5 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — `/brand` reference page is pre-D3-consistency-pass: lowercase blue Inter wordmark, plain blue `--brand-primary` chip, no Audiowide, no gradient. Actively misleads the next designer who lands on it.**

## Root cause

Memory `project_brand_consistency_pass` (locked 2026-05-19) mandates Audiowide hero + violet→pink gradient + warm radial-blob background + `#1e1b4b` ink across all in-product files.

The `/brand` reference at `web/app/(app)/brand/page.tsx` shows:
- Wordmark rendered in **lowercase blue Inter** ("skycot"), not `SKYCOT` in white Audiowide on a gradient surface.
- `--brand-primary` token displayed as a flat `#3B82F6`-ish blue (no gradient).
- No Audiowide on the page.
- No gradient anywhere.

Per the function-vs-dressing rule this is an internal designer doc; the demo never navigates to `/brand`. So NOT a demo blocker. But it actively misleads the next designer who lands on it — they'll think the brand is blue Inter when the canon is violet→pink Audiowide.

## Findings this brief closes

- whole-site-mvp-claims-coverage F5 [P1] — `/brand` reference page is pre-D3-consistency-pass

## Fix recipe

Two options; pick (a) for the longer-term value, (b) if the page genuinely has no consumers:

**(a) Rebuild `/brand` against the D3 brand-consistency recipe**:
1. Touch site: `web/app/(app)/brand/page.tsx`.
2. Wordmark panel: render `SKYCOT` in white Audiowide on a violet→pink gradient surface (recipe per memory + `web/public/brand/`).
3. Color token grid: replace `--brand-primary` flat blue with the actual gradient + per-token chips matching the locked palette (`#8B5CF6` → `#0891B2` etc.).
4. Add the warm radial-blob bg, `#1e1b4b` ink.
5. Source from `docs/skycot/brand/DESIGN_TOKENS.md` if it exists; otherwise reference the source-of-truth memory.

**(b) Delete the page entirely**:
1. Remove `web/app/(app)/brand/page.tsx` and any related styles.
2. Remove the `/brand` link from any nav or sitemap.
3. The canonical brand reference becomes `docs/skycot/brand/DESIGN_TOKENS.md` + memory `project_skycot_brand_identity`.

Recommend (a) because designers DO need a visual reference and a working page is more useful than a markdown doc.

## Acceptance criteria

- [ ] `/brand` renders the SKYCOT wordmark in white Audiowide on a violet→pink gradient.
- [ ] Color token chips show actual locked-brand colors, not flat blue.
- [ ] Warm radial-blob bg + `#1e1b4b` ink + Inter weight 200 sub-copy per the D3 recipe.
- [ ] OR the page is deleted and the `/brand` route 404s into the global brand-themed not-found page.

## May-touch

- `web/app/(app)/brand/page.tsx`
- `web/app/(app)/brand/page.css` (or styles file)
- Any nav component that links to `/brand`
- `docs/skycot/brand/DESIGN_TOKENS.md` (verify it matches the rebuilt page)

## Out of scope

- Wider brand-drift cleanup in components (separate brief `web-brand-drift-cleanup` if it exists)
- Marketing page theme drift (separate brief `fix-marketing-pages-theme-drift-landing-vs-compare`)
- `/brand` becoming a customer-facing surface (it's internal)

## Effort

~150-300 LOC for option (a); ~20 LOC for option (b). ~2-4 hours for (a); ~15 min for (b).
