# fix-support-and-for-bare-paths-404-need-index-redirects

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-support-and-for-bare-paths-404-need-index-redirects`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F20 [P2].

🟡 **P2 — Bare paths `/support` and `/for` return 404. Sub-paths `/support/inbox` and `/for/<vertical>` work. Any link or user-typed URL guess to `/support` or `/for` lands on 404.**

## Root cause

`web/app/(app)/support/` and `web/app/for/` exist as routing trees with no top-level `page.tsx`. Bare `/support` and `/for` therefore 404. Intended landing pages are `/support/inbox` and `/for/<vertical>`.

## Findings this brief closes

- whole-site-mvp-claims-coverage F20 [P2] — bare /support and /for return 404

## Fix recipe

1. **Add `web/app/(app)/support/page.tsx`** that redirects to `/support/inbox` (server-side redirect using Next.js `redirect()` from `next/navigation`).
2. **Add `web/app/for/page.tsx`** that redirects to a chosen default vertical (probably `/for/sales` or `/for/customer-support` per memory operator priority).
3. **Web smoke** asserts both bare paths return 200 (after redirect) or 30x with the right Location header.

## Acceptance criteria

- [ ] `curl /support` returns 307/308 with Location: `/support/inbox`.
- [ ] `curl /for` returns 307/308 with Location: `/for/<default>`.
- [ ] Browser navigation to bare path lands on the redirect target.
- [ ] Smoke test covers both.

## May-touch

- `web/app/(app)/support/page.tsx` (NEW)
- `web/app/for/page.tsx` (NEW)
- `web/tests/bare-paths-redirect.test.ts` (NEW)

## Out of scope

- Sub-page content (already exists)
- Marketing nav linkage (separate)

## Effort

~20-30 LOC + 1 test. ~30min.
