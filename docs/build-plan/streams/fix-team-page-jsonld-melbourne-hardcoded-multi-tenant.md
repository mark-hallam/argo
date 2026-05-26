# fix-team-page-jsonld-melbourne-hardcoded-multi-tenant

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-jsonld-melbourne-hardcoded-multi-tenant`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F18 [P2].

🟡 **P2 — `/team` JSON-LD embeds "Melbourne, Australia" hardcoded. Correct for SKYCOT brand metadata, but on a multi-tenant SaaS every tenant inherits Melbourne. Not v1.0 blocker; flag for white-label work.**

## Root cause

`/team` HTML contains JSON-LD `{"@type":"Place","name":"Melbourne, Australia"}` baked into the schema. Per `project_skycot_melbourne_australia` memory this is correct for SKYCOT, but when SKYCOT enters white-label mode every tenant inherits the address.

## Findings this brief closes

- whole-site-mvp-claims-coverage F18 [P2] — Melbourne hardcoded in /team JSON-LD

## Fix recipe

1. **Move SKYCOT brand JSON-LD into a separate brand-metadata component** (`web/components/skycot/marketing/BrandJsonLd.tsx`) that renders on marketing pages only.
2. **In `/team`**, render only tenant-specific schema (or omit Place metadata entirely if tenant has no configured location).
3. **Add a tenant.location field** to the tenant settings model (deferred to white-label work, just document).
4. **Web test** asserts /team JSON-LD does not contain hardcoded "Melbourne" string.

## Acceptance criteria

- [ ] `/team` rendered HTML does not contain hardcoded "Melbourne, Australia".
- [ ] Marketing pages (`/landing`, `/pricing`) still carry SKYCOT brand metadata.
- [ ] Web test asserts both.
- [ ] No regression on SEO meta tags on marketing pages.

## May-touch

- `web/app/(app)/team/page.tsx` (strip Place metadata)
- `web/components/skycot/marketing/BrandJsonLd.tsx` (NEW)
- `web/app/landing/page.tsx` (include BrandJsonLd)
- `web/tests/team-page-no-melbourne-hardcode.test.ts` (NEW)

## Out of scope

- Per-tenant location config (deferred to white-label)
- Marketing-side schema cleanup beyond this single Place type

## Effort

~40-60 LOC + 1 test. ~1 hour.
