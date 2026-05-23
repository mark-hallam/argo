# web-redirect-bare-auth-paths

**Owner**: codex.
**Depends on**: nothing.
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-redirect-bare-auth-paths`.

🟢 **P3 — bare auth paths 404.** Source: evaluator round-25 SHARED.md handoff (Chrome MCP companion session, 2026-05-23).

> "🟡 P3 NEW: bare paths `/signup` `/signin` `/verify` 404 (canonical at `/auth/*`)."

Operators or marketing links may point at `/signup` etc., not the canonical `/auth/signup`. Today these return 404. Add Next.js redirects so bare paths 308-redirect to the canonical paths.

## Scope

Add three Next.js `next.config.js` (or `next.config.ts`) `redirects()` entries:

- `/signup` → `/auth/signup` (308 permanent)
- `/signin` → `/auth/signin` (308 permanent)
- `/verify` → `/auth/verify` (308 permanent)

If `next.config` doesn't already have a `redirects()` export, add one. If it does, extend it.

## Files you may touch

- `web/next.config.ts` (or `.js` — whichever the project uses).
- One Playwright spec `web/tests/auth-bare-paths-redirect.spec.ts` that asserts the three bare paths redirect to the canonical paths.

## Files you must NOT touch

- The auth pages themselves under `web/app/auth/`.
- Any middleware.
- Any other config file.

## Acceptance criteria

- [ ] `next.config` exports a `redirects()` (or extended one) with the three entries.
- [ ] Playwright spec loads `/signup` `/signin` `/verify` (with `redirect: 'manual'` in fetch terms) and asserts the redirect-target header is the canonical `/auth/*` path.
- [ ] `pnpm typecheck` + `pnpm lint` clean in `web/`.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Playwright spec is the regression contract.
- Manual: `curl -I http://localhost:3000/signup` returns 308 with Location: /auth/signup.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Adding new auth pages.
- Other bare-path redirects (just these 3).
- Marketing-page redirects.

~20-60 LOC. Fast-mode-eligible mechanical work — codex preferred.
