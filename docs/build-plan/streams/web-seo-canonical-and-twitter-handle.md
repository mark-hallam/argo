# web-seo-canonical-and-twitter-handle — canonical URLs + twitter:site + root description trim

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seo-canonical-and-twitter-handle`.
**Priority**: P3 — wave-2 SEO audit: zero canonical URLs, no twitter:site handle, root description >155 chars (truncates in search).
**Sizing**: ~80-150 LOC (metadata adjustments + tests).

## Why this brief exists

From the wave-2 SEO audit:

> Other absences: zero canonical URLs, no `twitter:site` handle,
> root description is 261 chars (will be truncated at 155).

Crawlers use canonical URLs to deduplicate. Twitter unfurls credit
the brand via @handle. Search engines truncate descriptions.

## Goals

1. Add `metadata.alternates.canonical` to every public page (e.g. `https://skycot.com/landing`).
2. Add `metadata.twitter.site = "@skycot_ai"` (matches operator handle per `~/.claude/.../project_skycot_brand_identity.md`).
3. Trim root description in `web/app/layout.tsx` to ≤155 chars (currently 261).
4. Per-page descriptions also ≤155 chars (audit each).

## Steps

1. Update root `web/app/layout.tsx` metadata: trim description, add alternates.canonical, add twitter.site.
2. Per public page layout.tsx: add canonical URL.
3. Verify in dev: `curl http://localhost:3000/landing | grep -E 'canonical|twitter:site|description'`.
4. Test: snapshot the metadata for each public page.

## may-touch

- `web/app/layout.tsx` (root metadata trim + alternates + twitter)
- Each public page's layout.tsx (canonical)
- Tests

## must-NOT-touch

- Open Graph (handled by sibling brief)
- JSON-LD (handled by sibling brief)
- Title templates (already working)

## Acceptance criteria

- [ ] Every public page has `<link rel="canonical" href="https://skycot.com/<path>">` in rendered `<head>`.
- [ ] `<meta name="twitter:site" content="@skycot_ai">` in every public page's head.
- [ ] Root description ≤155 chars.
- [ ] Each per-page description ≤155 chars.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/seo/2026-05-20-wave-2-seo.md`
- Memory: `project_skycot_brand_identity.md` (twitter handle)
