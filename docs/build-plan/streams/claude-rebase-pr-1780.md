# claude-rebase-pr-1780 — recover #1780 claude-marketing-site-blog-mdx (P2)

**Owner**: claude. **Recovery target**: PR [#1780](https://github.com/mark-hallam/Argo-Agent/pull/1780). **Original brief**: `streams/claude-marketing-site-blog-mdx.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1780`.

## Why this brief exists

PR #1780 (`/blog` route + 3 MDX seed posts + RSS + dynamic OG) opened 2026-05-16T14:18. Operator authorized admin-merge but the PR was DIRTY mid-attempt (#1783 cookie-banner-and-legal-pages merged a sibling change touching `marketing/`). Stayed DIRTY ≥20h.

## Acceptance: pick ONE recovery path

### Path A — rebase

1. `git fetch origin && git checkout claude-claude-marketing-site-blog-mdx`
2. `git rebase origin/main`. Conflicts likely in:
   - `marketing/app/layout.tsx` (cookie-banner mounted Toaster-like primitives here)
   - `marketing/components/Footer.tsx` (touched by discord-community-setup + cookie-banner)
   - `marketing/package.json` (mdx + sonner + cookie-banner deps may need merge)
3. Verification: `cd marketing && pnpm install && pnpm typecheck && pnpm lint && pnpm build`.
4. `git push --force-with-lease origin claude-claude-marketing-site-blog-mdx`.

### Path B — fresh start

1. Re-read `streams/claude-marketing-site-blog-mdx.md`.
2. Implement on `claude-marketing-site-blog-mdx-v2`.
3. Close #1780.

**Path A is preferred** — this PR is pure-additive (new `/blog` route, new MDX files); conflicts are likely cosmetic (Footer import list, layout.tsx mount order).

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY.
- [ ] `/blog`, `/blog/[slug]`, `/blog/feed.xml` routes serve.
- [ ] 3 seed MDX posts present (founder narrative + macOS-CU + swarm-development).
- [ ] `pnpm build` succeeds; all routes prerender static.
- [ ] Operator-approved prose stays as-is (don't rewrite the 3 posts).
- [ ] BUILD_STATUS.md row flipped TODO → IN_REVIEW.

## Hidden constraints

- **Operator already approved the prose** in the prior IN_REVIEW state — don't re-litigate the content; preserve verbatim.
- **MDX bundler dep**: prior worker may have added `@next/mdx`; check `marketing/package.json` diff after rebase.
- **Conductor's note in prior demotion**: "PR ships clean (typecheck/lint/build all green)". Only the merge-conflict is broken; the code is fine.
