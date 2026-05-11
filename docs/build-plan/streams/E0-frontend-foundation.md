# Stream E0 — Frontend foundation: routes, auth bridge, AppShell, gateway client

**Owner**: claude.
**Depends on**: none — extends the Track C scaffold already at `web/`.
**Authored against**: origin/main `466a2457` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E0-frontend-foundation`.
**Tier tag**: PRODUCT_ROADMAP.md Track E foundation. **CONTRACT PR** — downstream E1–E5 briefs implement against the routing + auth + shell decisions this PR locks. Get the shape right.

## Why

The Track C scaffold ships a single page at `/` (the approvals
inbox demo). Track E adds five more surfaces — team, worker
profile, audit, hire flow, settings — each with multiple sub-
routes, each needing gateway-authenticated data fetching, each
re-using a shared chrome (sidebar + main, or chrome-free for
the hire flow). This brief lands that scaffold:

- Multi-route file structure under `app/`.
- Two route groups: `(app)` for sidebar-bearing surfaces,
  `(hire)` for the full-page hire flow.
- Auth bridge: NextAuth v5 reading the existing
  `ARGO_GATEWAY_BEARER_TOKEN` precedent.
- A typed `fetch` wrapper at `lib/api/client.ts` that all
  TanStack Query consumers route through.
- An `AppShell` composite that hosts the sidebar +
  breadcrumb + main, leaving the page-level content as a
  child slot.

Downstream slices (E1–E5) extend specific surfaces against this
scaffold; none re-litigate routing, auth, or shell layout.

## Scope

This is a CONTRACT PR. Decisions locked here:

- **Routing shape**: Next.js App Router route groups, named
  routes, layout inheritance.
- **Auth model**: NextAuth v5 session bridging to the gateway's
  bearer token. Public routes vs protected routes.
- **Layout shells**: `AppShell` for normal surfaces;
  `HireShell` for the chrome-free onboarding flow.
- **Gateway client conventions**: error envelope shape,
  retry semantics, auth header handling.
- **Query-key conventions**: how downstream hooks namespace
  their cache keys.
- **404 / error boundary shape**.

Downstream briefs reference this doc by section. If a brief
proposes deviating, escalate to operator BEFORE merge.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: `web/app/layout.tsx`

## Files you may touch

- `web/app/layout.tsx` — narrowly scoped: wrap children in
  `AuthProvider` and `<SessionProvider>` (added). Do NOT alter
  font loading or `<ThemeProvider>` mount.
- `web/app/(app)/layout.tsx` — NEW. Route-group layout that
  mounts `AppShell`. Children render as the main pane.
- `web/app/(app)/page.tsx` — MOVE the existing `app/page.tsx`
  here (the approvals inbox). The `/` route is preserved via
  the route-group rules (`(app)` groups don't add to URLs).
- `web/app/(hire)/layout.tsx` — NEW. Chrome-free shell for the
  hire flow. Renders `HireShell` (header bar + progress dots
  + escape button).
- `web/app/(hire)/hire/page.tsx` — NEW. Stub landing for the
  hire flow. Returns a placeholder `"Step 1 lands in E2-b"`
  pending E2.
- `web/app/auth/signin/page.tsx` — NEW. Minimal sign-in
  surface. Bearer-token input + "Sign in" button. Calls
  NextAuth's `signIn("gateway-bearer", { token })`.
- `web/app/not-found.tsx` — NEW. 404 page using AppShell.
- `web/middleware.ts` — NEW. Auth gate. Redirects unauth users
  hitting any non-`/auth/*` route to `/auth/signin?from=<path>`.
- `web/auth.config.ts` — NEW. NextAuth v5 config. Single
  credentials provider `"gateway-bearer"` that takes a token
  and verifies it by hitting the gateway's healthcheck with
  the token attached.
- `web/auth.ts` — NEW. NextAuth v5 instance export.
- `web/lib/api/client.ts` — NEW. Typed `fetch` wrapper. Reads
  the session's bearer token, builds the URL, throws a
  typed `ApiError` on non-2xx. ≤ 200 LOC.
- `web/lib/api/types.ts` — NEW. Shared API types:
  `ApiError`, `ApiEnvelope<T>` if used, `PaginatedResponse<T>`.
  ≤ 100 LOC.
- `web/components/skycot/AppShell.tsx` — NEW. Sidebar (the
  existing NavDrawer composition) + main pane + optional
  breadcrumb slot. ≤ 250 LOC.
- `web/components/skycot/HireShell.tsx` — NEW. Header bar with
  brand square + progress dots + escape button. ≤ 200 LOC.
- `web/components/skycot/AuthProvider.tsx` — NEW. Wraps NextAuth's
  `SessionProvider`. Mounted in `app/layout.tsx`. ≤ 50 LOC.
- `web/components/ui/Breadcrumb.tsx` — NEW. Generic breadcrumb
  primitive. `<Breadcrumb><BreadcrumbItem>...` shape, matching
  shadcn pattern. ≤ 120 LOC.
- `web/components/ui/ProgressDots.tsx` — NEW. Used by HireShell;
  reusable. ≤ 80 LOC.
- `web/lib/api/approvals.ts` — narrowly scoped: replace the
  `// TODO(track-e)` comment-out blocks with real fetcher
  calls via the new `lib/api/client.ts` wrapper. Same
  signatures; mock dataset preserved as fallback when
  gateway returns 404 (so the demo page still works in
  dev without a running gateway).
- `web/package.json` — narrowly scoped: add ONE dep:
  `next-auth@5.0.0-beta.x`. No other changes (deps already
  declared in scaffold).
- `web/.env.example` — NEW. Document env vars:
  `NEXT_PUBLIC_GATEWAY_URL`, `NEXTAUTH_SECRET`,
  `NEXTAUTH_URL`.
- `web/README.md` — narrowly scoped: add an "Authentication"
  section explaining the bearer-token flow.

Stories per primitive (for `Breadcrumb`, `ProgressDots`) are
deferred to a follow-up stories brief; not in scope here.

## Files you must NOT touch

- `web/app/page.tsx` — the existing demo page is being MOVED
  to `(app)/page.tsx`. Do not modify its contents in this PR.
- `web/components/ui/*` (other than the two new files):
  Button, Card, Avatar, StatusPill, NavDrawer, Toast,
  DataCell — locked from Track C ADR.
- `web/components/skycot/ApprovalCard.tsx`,
  `ThemeProvider.tsx`, `ThemePicker.tsx`,
  `QueryProvider.tsx` — locked.
- `web/lib/utils.ts`, `web/lib/theme/*` — locked.
- `web/lib/sample-approvals.ts`, `web/lib/hooks/use-
  approvals.ts` — keep as-is. The mock fallback path goes
  through `lib/api/approvals.ts`.
- `crates/**` — Rust workspace. No backend changes in this
  PR. A separate brief (`F-gateway-bearer-token-frontend-
  scope`) handles any gateway-side updates.

## Acceptance criteria

- [ ] `pnpm install` succeeds with next-auth added.
- [ ] All existing routes still work: `/` renders the approvals
      inbox identically to pre-PR; `/?b=<id>` URL pattern still
      works.
- [ ] `/auth/signin` renders a minimal form: bearer-token
      input + Sign in button. Strings: title `"Sign in to SKYCOT"`,
      placeholder `"Paste your gateway bearer token"`,
      CTA `"Sign in"`.
- [ ] Unauth user hitting `/` is redirected to
      `/auth/signin?from=/`. After successful auth, redirected
      back to `from`.
- [ ] Auth state is server-readable in layouts (via
      `auth()` helper). Client components can use
      `useSession()`.
- [ ] Auth bypass for dev: setting
      `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` in `.env.local` skips
      the middleware redirect (mirrors the gateway's
      `ARGO_GATEWAY_AUTH_BYPASS=1` precedent).
- [ ] `AppShell` renders sidebar + main; sidebar is the
      existing NavDrawer with the existing nav items.
- [ ] `HireShell` renders a header bar with brand + progress
      dots + escape (X) button. Stub `/hire` route works.
- [ ] `Breadcrumb` primitive renders. Empty Storybook story
      acceptable (full stories deferred).
- [ ] `ProgressDots` primitive renders. Used by `HireShell`.
- [ ] `lib/api/client.ts`: a typed `fetch` wrapper exported as
      `apiGet<T>`, `apiPost<T, B>`, `apiPatch<T, B>`,
      `apiDelete<T>`. Throws `ApiError` (subclass of `Error`
      with `status: number` + `code?: string` + `body?: unknown`).
- [ ] `lib/api/approvals.ts`: `fetchApprovals`, `submitDecision`,
      `submitUndo` route through `lib/api/client.ts`. Falls back
      to the mock dataset when the gateway returns 404 or a
      `NetworkError`.
- [ ] `app/not-found.tsx`: renders inside AppShell with title
      `"Page not found"` + body `"That route doesn't exist or
      you don't have access."` + CTA `"Back to approvals"`.
- [ ] All ApprovalCard unit tests still pass (`pnpm test`).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 25KB First Load
      JS vs pre-PR baseline (next-auth adds ~15KB).
- [ ] Every modified or new file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E0-frontend-foundation`
- PR title: `web: foundation — routes, auth bridge, AppShell, gateway client (E0)`

## Notes for the implementer

- **Route groups.** Next.js App Router treats `(app)` and
  `(hire)` as organizational folders that don't add URL
  segments. So `/` resolves to `(app)/page.tsx`; `/hire` to
  `(hire)/hire/page.tsx`. Layout inheritance: every
  surface inside `(app)/` automatically gets `AppShell`;
  every surface inside `(hire)/` gets `HireShell`.

- **AppShell composition.** Reuse the existing
  `<NavDrawer>` + `<NavItem>` components from
  `web/components/ui/NavDrawer.tsx`. The shell renders:
  ```tsx
  <div className="min-h-screen">
    <NavDrawer brand={<Brand />}>
      {navItems}  // imported list, see below
    </NavDrawer>
    <main className="md:pl-64">
      {breadcrumb && <BreadcrumbBar />}
      {children}
    </main>
  </div>
  ```
  Extract the nav items list into a shared module
  (`web/components/skycot/nav-items.tsx`) so each surface
  can mark its active item without duplicating the list.

- **NavItem active state.** Read `usePathname()` and
  compare against each item's `href`. Active when
  pathname starts with href (so `/workers/123` highlights
  `Workers`).

- **HireShell.** Full-bleed; no sidebar; brand square top-left
  with a click handler that returns to `/` (with a confirm
  dialog if `unsavedHire` is true in session storage).
  Progress dots fed via a prop or context — leave as a prop
  for now, downstream E2 briefs may switch to context if needed.

- **Auth provider choice.** NextAuth v5 (Auth.js) credentials
  provider. The token verification step inside `authorize()`
  calls the gateway's healthcheck (`GET /healthz`) with
  `Authorization: Bearer <token>` and treats 200 as valid.
  Persist token in the JWT session strategy (no DB needed).

- **Auth-bypass for dev.** Mirror the Rust gateway's pattern:
  reading `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` in middleware
  short-circuits the redirect. CI/staging never sets this;
  local dev does. Document in README.

- **Gateway client error envelope.** The Rust gateway returns
  errors as `{ "error": { "code": "...", "message": "..." } }`.
  Match it in `ApiError`:
  ```ts
  class ApiError extends Error {
    constructor(public status: number, public code?: string, public body?: unknown) { ... }
  }
  ```
  On 4xx, try to parse the body as the envelope; on 5xx or
  parse failure, set `code = undefined` and `body = undefined`.

- **Query-key conventions** (for E1–E5 briefs to follow):
  - Resource lists: `["resource"]` (e.g. `["approvals"]`,
    `["team"]`, `["workers"]`).
  - Single-resource: `["resource", id]` (e.g.
    `["worker", workerId]`).
  - Filtered list: `["resource", { ...filters }]`.
  - Sub-resource: `["resource", id, "sub-resource"]` (e.g.
    `["worker", workerId, "activity"]`).
  Document this in `lib/hooks/README.md` (NEW, ≤ 30 lines).

- **Don't add a route for `/team`, `/workers`, `/audit`,
  `/skills`, or `/settings` in this PR.** Each gets its own
  E1–E5 slice. AppShell will catch the 404 via
  `app/not-found.tsx` when a user clicks a nav item that
  doesn't have a route yet. That's intentional — surfaces
  light up as they ship.

- **Reuse the existing TanStack Query setup**
  (`QueryProvider.tsx`) without modification. The hooks
  layer (`lib/hooks/use-approvals.ts`) already uses it
  correctly.

- **Test plan.** Existing 6 ApprovalCard unit tests must
  still pass. Add at least:
  - `lib/api/client.test.ts`: 4xx returns typed ApiError,
    5xx returns typed ApiError, 2xx returns parsed body.
  - `middleware.test.ts`: unauth → redirect, auth →
    passthrough, bypass-flag → passthrough.
  No new Playwright e2e in this PR — those live in their own
  brief slice.
