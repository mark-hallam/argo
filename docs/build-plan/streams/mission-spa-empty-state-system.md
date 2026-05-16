# mission-spa-empty-state-system — unified `<EmptyState/>` primitive + 8 applications (Wave 7 P2)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED (#1744).
**Authored against**: origin/main `111a6559` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-spa-empty-state-system`.
**Source**: Wave 7 Jony-Ive polish — today's SPA empty states are ad-hoc (some show "No data", some show nothing, some show a sad-face emoji). Unified primitive + brand voice across 8 surfaces.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — empty state is the user's first sight of a feature. "Helpful + calm + competent" voice across them all.
- **Repeatability** (#3) — one `<EmptyState/>` component, not 8 hand-rolled ones.

## Scope

P2. Build a single `<EmptyState/>` primitive consuming brand tokens, then apply to 8 SPA surfaces.

### Component shape

```tsx
<EmptyState
  illustration={WorkersIllustration}   // optional inline SVG
  headline="No workers yet"
  body="Hire your first AI teammate to start shipping work."
  cta={{ label: "Hire your first worker", href: "/hire" }}
  secondary={{ label: "Browse vertical packs", href: "/marketplace" }}
/>
```

Variants: `default` (full state) | `compact` (headline + body only) | `inline` (single-line, no illustration).

### 8 surfaces with copy

| Surface | Headline | Body | CTA |
|---|---|---|---|
| `/workers` (no workers) | "No teammates yet" | "Hire your first AI worker to start shipping." | "Hire a worker" → `/hire` |
| `/workers/:id/skills` | "No skills yet" | "Maya hasn't learned any skills yet. Skills emerge from successful actions or you can install from the marketplace." | "Browse marketplace" |
| `/workers/:id/memory` | "Memory is quiet" | "When Maya remembers facts or decisions, they appear here." | (no CTA) |
| `/workers/:id/evals` | "No evals run" | "Run an eval set to grade Maya's work against expected outcomes." | "Run an eval" |
| `/audit` | "Quiet day" | "Worker actions and approvals will appear here as they happen. Plenty of room for activity." | (no CTA) |
| `/team` | "Just you so far" | "Invite teammates to collaborate on workers and approvals." | "Invite" |
| `/cohorts` | "No cohorts" | "Group similar tasks into a cohort to share context across workers." | "Create cohort" |
| `/support/inbox` | "Inbox zero" | "All caught up. New customer requests will appear here." | (no CTA) |

**Discipline (per brand IDENTITY.md):**
- No exclamation points.
- Sentence case (not Title Case).
- No emojis.
- Voice: calm + competent + helpful. NOT cute, NOT corporate.
- Max body length: 2 lines at 480px viewport width.
- Illustration is single-color line-art motif (inherits `--foreground-muted`), inline SVG.

LOC: ~350.

## Files you may touch

- `web/components/ui/EmptyState.tsx` (NEW) — the primitive. ≤100 LOC.
- `web/components/ui/EmptyState.test.tsx` (NEW) — render + a11y. ≤60 LOC.
- `web/components/empty-state/illustrations/` (NEW directory) — 8 SVG components. ≤30 LOC each = ≤240 LOC.
- 8 page wrappers — apply `<EmptyState/>` conditionally:
  - `web/app/(app)/workers/page.tsx`
  - `web/app/(app)/workers/[workerId]/skills/page.tsx`
  - `web/app/(app)/workers/[workerId]/memory/page.tsx`
  - `web/app/(app)/workers/[workerId]/evals/page.tsx`
  - `web/app/(app)/audit/page.tsx`
  - `web/app/(app)/team/page.tsx`
  - `web/app/(app)/cohorts/page.tsx`
  - `web/app/(app)/support/inbox/page.tsx` (if from `claude-support-inbox-spa-page` — currently in queue)
  ≤30 modified LOC each, ≤240 total.

## Files you must NOT touch

- Brand tokens — consume only.
- API/data hooks — empty state is presentation; the conditional comes from `data?.length === 0` in existing hooks.
- Marketing site (`marketing/`) — separate empty-state philosophy.

## Acceptance criteria

- [ ] `<EmptyState/>` renders cleanly in all 3 variants (default/compact/inline).
- [ ] 8 surfaces each show their `<EmptyState/>` when underlying data is empty.
- [ ] Copy matches the table above verbatim.
- [ ] Accessibility: each empty state has semantic `<section aria-label="...">` and headline is `<h2>`.
- [ ] Lighthouse a11y ≥ 95 on each touched page (manual or via `pnpm exec @lhci/cli` if available).
- [ ] Bundle delta ≤ 6 KB gzipped (SVGs are small).
- [ ] No exclamation points in copy.
- [ ] `pnpm test && pnpm typecheck && pnpm lint` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-spa-empty-state-system`
- Title: `feat(web): unified <EmptyState/> primitive + 8 surfaces (mission-spa-empty-state-system)`
- Worker verification: pnpm checks + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Don't conflate empty with loading.** Loading state is a skeleton; empty state is "data fetched, result is zero rows". Wire only on `!isLoading && data?.length === 0`.
- **Don't auto-redirect from empty to CTA.** User must SEE the empty state and choose. The CTA is a button, not an auto-navigate.
- **Illustration motifs match the surface.** Workers gets a small workers icon; audit gets a timeline icon. Don't reuse one generic illustration.
- **`/support/inbox` brief is in TODO.** If it lands before this brief, wire it up here. If not, gate that single application behind `// TODO(post-support-inbox)` and ship the other 7.
- **Don't add new emojis.** Look — even single-emoji "✨" feels cheap on enterprise SaaS. Lucide icons are the visual language.
