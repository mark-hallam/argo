# claude-marketing-site-blog-mdx — `/blog` route + MDX posts for marketing site (Wave 5 P2)

**Owner**: claude. **Depends on**: `claude-marketing-site-skycot-com` MERGED (#1754, 6318d8da).
**Authored against**: origin/main `2b984b94` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-marketing-site-blog-mdx`.
**Source**: Wave 5 launch content surface — SKYCOT needs to publish thought-leadership at launch (one of the only durable distribution mechanisms vs Anthropic/OpenAI marketing flywheel).

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — owned distribution channel; not platform-dependent (Twitter/HN).
- **Repeatability** (#3) — operator writes prose in MDX, single template renders consistently.

## Scope

P2. New `/blog` route on `skycot.com` marketing site with:

- Index page at `/blog` — chronological list of posts with title, summary, date, author, reading-time.
- Per-post route at `/blog/[slug]` — MDX content rendered with the marketing-site's typography.
- Three seed posts authored at launch:
  - `/blog/why-we-built-skycot` — founder narrative (operator approves content).
  - `/blog/computer-use-on-macos` — technical post on the macOS-CU executor.
  - `/blog/swarm-development` — meta-post on the 16-PR admin-merge-wave session (this very swarm pattern).
- RSS feed at `/blog/feed.xml`.
- OG image generation per post (Next.js dynamic OG via `app/blog/[slug]/opengraph-image.tsx`).
- JSON-LD `BlogPosting` structured data.

MDX implementation:
- Use `@next/mdx` + `remark-gfm` + `rehype-pretty-code` (with shiki for syntax highlighting).
- Posts under `marketing/content/blog/*.mdx`.
- Frontmatter: `title`, `summary`, `date` (ISO 8601), `author`, `cover_image` (optional path).

LOC: ~600.

## Files you may touch

- `marketing/package.json` — narrowly: add `@next/mdx`, `@mdx-js/react`, `remark-gfm`, `rehype-pretty-code`, `shiki`, `reading-time`. ≤8 modified LOC.
- `marketing/next.config.ts` — wire MDX page extensions. ≤15 modified LOC.
- `marketing/app/blog/page.tsx` (NEW) — index page. ≤120 LOC.
- `marketing/app/blog/[slug]/page.tsx` (NEW) — post renderer. ≤120 LOC.
- `marketing/app/blog/[slug]/opengraph-image.tsx` (NEW) — dynamic OG. ≤60 LOC.
- `marketing/app/blog/feed.xml/route.ts` (NEW) — RSS endpoint. ≤80 LOC.
- `marketing/components/blog/PostCard.tsx` (NEW). ≤60 LOC.
- `marketing/components/blog/MdxComponents.tsx` (NEW) — custom MDX renderers (Code, Callout, Figure). ≤80 LOC.
- `marketing/lib/blog/posts.ts` (NEW) — frontmatter parser + post-list loader. ≤80 LOC.
- `marketing/lib/blog/posts.test.ts` (NEW). ≤60 LOC.
- `marketing/content/blog/why-we-built-skycot.mdx` (NEW) — ~600 words. ≤25 LOC frontmatter + ~150 LOC prose.
- `marketing/content/blog/computer-use-on-macos.mdx` (NEW) — ~800 words technical. ≤180 LOC.
- `marketing/content/blog/swarm-development.mdx` (NEW) — ~700 words meta. ≤170 LOC.
- `marketing/app/sitemap.ts` — extend to include blog routes. ≤15 modified LOC.
- `marketing/components/Nav.tsx` — add Blog link to nav. ≤5 modified LOC.

## Files you must NOT touch

- `web/` SPA — blog is marketing-side only.
- The existing brand tokens — consume only.
- Operator-side content surfaces (changelog/roadmap from #1761 are separate concerns).

## Acceptance criteria

- [ ] `/blog` lists 3 posts in reverse-chronological order.
- [ ] `/blog/why-we-built-skycot` renders MDX cleanly with typography matching marketing brand.
- [ ] Code blocks have syntax highlighting (shiki).
- [ ] `/blog/feed.xml` returns valid RSS 2.0 (validates with `feedvalidator.org` rules).
- [ ] Each post has dynamic OG image with title + author + date overlaid on brand gradient.
- [ ] Sitemap.ts includes `/blog` + each post URL.
- [ ] Nav shows Blog link.
- [ ] `pnpm build` succeeds; all blog routes prerender static.
- [ ] `pnpm typecheck && pnpm lint` clean.
- [ ] Lighthouse score ≥ 95 on blog index + a sample post (verify in Vercel preview or via `npx @lhci/cli`).
- [ ] **Operator review required** for all 3 seed posts' content before merge.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-marketing-site-blog-mdx`
- Title: `marketing: /blog route + MDX posts + RSS + dynamic OG (claude-marketing-site-blog-mdx)`
- Worker verification: pnpm checks + Lighthouse + per SWARM_PROTOCOL.md.

## Hidden constraints

- **MDX != Markdown.** Imports inside `.mdx` files are supported; use them to inject Figure/Callout components.
- **`reading-time` is per-post.** Compute server-side at build time; render in PostCard + post header.
- **RSS dates are RFC 822.** Not ISO 8601. Use `new Date(post.date).toUTCString()`.
- **OG image route returns 5xx if frontmatter missing.** Fail gracefully — fallback to the global `/og.png`.
- **Don't add a comments system.** Out of scope; if launch traffic needs comments, file follow-up `claude-blog-comments-disqus-or-utterances`.
- **Author field is just a string for now.** Multi-author bios is a future brief.
- **Seed-post content: operator reviews.** Worker drafts; operator amends; PR doesn't merge until operator approves the prose. Add `// REVIEW-REQUIRED: prose` comment marker.
