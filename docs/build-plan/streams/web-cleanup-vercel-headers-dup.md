# web-cleanup-vercel-headers-dup — delete duplicate security headers from vercel.json

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cleanup-vercel-headers-dup`.
**Priority**: P3 — wave-4 security-headers audit cosmetic finding: vercel.json:28-37 duplicates 3 of the 6 security headers we set via next.config.ts.
**Sizing**: ~20-50 LOC (delete + verify + test).

## Why this brief exists

From the wave-4 security-headers audit:

> Top 3 gaps: (1) none block the demo; (2) `vercel.json:28-37`
> duplicates 3 of 6 headers — cosmetic but confusing, recommend
> deleting; (3) error reporting proxy for when Sentry turns on
> later.

Single source of truth for security headers: `web/next.config.ts`
`async headers()`. The `vercel.json` rules duplicate them and could
diverge over time.

## Goals

1. Delete the duplicate header rules from `web/vercel.json`.
2. Verify next.config.ts headers still serve on dev + Vercel preview.
3. Document in a comment that headers live in next.config.ts, not vercel.json.

## Steps

1. Read `web/vercel.json` — identify the duplicate header block (lines ~28-37).
2. Delete the block.
3. Add a tiny comment near the top of vercel.json: `// Security headers live in next.config.ts — do not duplicate here.`
4. Verify: `curl -I http://localhost:3000/landing` still returns all 6 headers.
5. Optionally: deploy to a Vercel preview and verify.

## may-touch

- `web/vercel.json`

## must-NOT-touch

- `web/next.config.ts` (the canonical source — keep as-is)
- Other vercel.json fields (env, redirects, rewrites)

## Acceptance criteria

- [ ] vercel.json no longer duplicates security headers.
- [ ] `curl -I http://localhost:3000/landing` still returns all 6 security headers.
- [ ] Comment in vercel.json documents the canonical location.
- [ ] `pnpm build` succeeds.

## References

- Audit: `docs/skycot/audit/security-headers/2026-05-20-wave-4-security-headers.md`
- Canonical: `web/next.config.ts` `async headers()`
