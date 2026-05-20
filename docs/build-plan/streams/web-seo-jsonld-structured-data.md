# web-seo-jsonld-structured-data — JSON-LD structured data on public pages

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seo-jsonld-structured-data`.
**Priority**: P3 — wave-2 SEO audit: zero JSON-LD anywhere. Google/Bing reward structured data with rich snippets.
**Sizing**: ~150-250 LOC (per-page Schema.org JSON + tests).

## Why this brief exists

From the wave-2 SEO audit:

> Other absences: zero JSON-LD anywhere, zero canonical URLs, no
> twitter:site handle, root description is 261 chars (will be
> truncated at 155).

JSON-LD structured data per public surface lets crawlers
understand SKYCOT's product, pricing, FAQs, articles.

## Goals

1. Root `<Organization>` Schema.org JSON-LD on every page.
2. `<Product>` JSON-LD on /landing + /pricing (product name, description, brand, offers).
3. `<FAQPage>` JSON-LD on /pricing FAQ section.
4. `<Article>` JSON-LD on /changelog entries.
5. Use Next.js metadata API + a small `<JsonLd>` component that renders `<script type="application/ld+json">`.

## Steps

1. Build `<JsonLd>` component that takes a typed Schema.org object and renders the script tag.
2. Per public page, attach the relevant JSON-LD via the metadata API or inline component.
3. Validate via Google's Rich Results Test (manual one-off + a test harness).
4. Lint-only: ensure JSON is valid.

## may-touch

- New: `web/components/seo/JsonLd.tsx`
- Each public page's layout.tsx or page.tsx (add JSON-LD)
- Test files

## must-NOT-touch

- Other metadata fields (handled by sibling SEO briefs)

## Acceptance criteria

- [ ] Every public page has at least Organization JSON-LD.
- [ ] /landing + /pricing have Product JSON-LD.
- [ ] /pricing FAQ has FAQPage JSON-LD.
- [ ] /changelog has Article JSON-LD (per entry).
- [ ] Google Rich Results Test passes on each surface (manual sign-off OK for v1).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/seo/2026-05-20-wave-2-seo.md`
- Schema.org/Product, Schema.org/FAQPage, Schema.org/Article, Schema.org/Organization
