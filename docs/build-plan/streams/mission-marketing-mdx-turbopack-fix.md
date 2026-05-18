# mission-marketing-mdx-turbopack-fix — marketing dev server starts cleanly (P1)

**Owner**: claude. **Sizing hint**: build-tooling investigation, narrow fix.
**Depends on**: none.
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketing-mdx-turbopack-fix`.
**Source**: 2026-05-18 browser UX audit — `cd marketing && pnpm dev` (which defaults to Turbopack via `package.json:scripts.dev`) errors on the `@next/mdx` loader: `loader does not have serializable options`. Marketing dev server never finishes booting. Workaround today: `pnpm exec next dev --port 3001` (no Turbopack).

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — `pnpm dev` as documented should just work.
- **Commercial usability** (#5) — marketing is the first thing prospective customers see; the marketing dev experience must be solid.

## Scope

P1. Two paths; worker picks whichever is simpler after investigating.

### Path A — Fix MDX + Turbopack compatibility

Upgrade `@next/mdx`, `@mdx-js/loader`, `@mdx-js/react` to versions that work with Turbopack 15.5.x. Make sure the `mdx` config in `next.config.ts` (or `mdx.config.ts`) uses serializable options only — no function references, no class instances, only plain JSON-shaped values.

### Path B — Disable Turbopack for marketing (documented)

If Path A is blocked by upstream library compatibility, change `marketing/package.json:scripts.dev` from `next dev --turbopack -p 3001` to `next dev -p 3001` (Webpack). Document the trade-off in `marketing/README.md`.

Worker picks. Either path is acceptable for v1.0.

LOC: ~50.

## Files you may touch

- `marketing/package.json` — `scripts.dev`, possibly `dependencies` versions. ≤10 modified LOC.
- `marketing/next.config.ts` — mdx config tweaks if Path A. ≤30 modified LOC.
- `marketing/mdx.config.ts` (if it exists). ≤20 modified LOC.
- `marketing/README.md` — document the dev-mode choice + any caveat. ≤30 modified LOC.

## Files you must NOT touch

- Marketing content (`marketing/app/**`, `marketing/content/**`) — pure build-config fix.
- The web SPA's MDX config — not in scope.

## Acceptance criteria

- [ ] `cd marketing && pnpm dev` starts cleanly and serves `http://localhost:3001/` within 30s on a cold start.
- [ ] No MDX-loader exceptions in the dev log.
- [ ] All existing marketing pages render the same as before the fix.
- [ ] `pnpm build` works (production build still passes).
- [ ] README documents the dev-mode setup.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-marketing-mdx-turbopack-fix`
- Title: `fix(marketing): pnpm dev starts cleanly without MDX/Turbopack incompatibility (mission-marketing-mdx-turbopack-fix)`
- Worker verification: cold pnpm install + pnpm dev startup + manual /  page render.

## Hidden constraints

- **The fix is local — don't pull marketing into the web/ workspace.** They're separate Next.js apps intentionally.
- **If Path A requires a major version bump on upstream MDX libraries**, prefer Path B (less risk). Document why.
