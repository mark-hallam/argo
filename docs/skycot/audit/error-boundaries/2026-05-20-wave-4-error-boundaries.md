# Error Boundaries Audit — Wave 4 / Audit #2

**Date:** 2026-05-20
**Auditor:** Wave-4 audit agent #2 (read-only)
**Scope:** `~/code/argo/web/app/` — Next.js 15 App Router error-boundary coverage
**Demo recording:** tomorrow (2026-05-21). A white-screen on an unexpected throw is the worst on-camera state.

---

## Executive summary

The web app has exactly **one route-level `error.tsx`** in the entire tree — the **root `web/app/error.tsx`** — and exactly **one `not-found.tsx`** (root). There is **NO `global-error.tsx`** anywhere, and **NO client-side `ErrorBoundary` / `componentDidCatch` / `react-error-boundary`** usage anywhere in `web/`. That means:

- Every authed-product page, marketing page, auth page, onboarding page, and hire-flow page falls through to the root `error.tsx` when something throws. The boundary works (it's a clean `<ErrorShell />` + retry), but it replaces **the entire shell**, including the chrome users were navigating in. Mid-demo this is a hard hand-off out of context.
- The React Flow canvas at `/(app)/workflows/[v]/[name]/build` has no canvas-local boundary. A throw inside `<ReactFlow>` or any of the seed-data lookups will blow up the whole route and unmount the `<AppShell>` chrome.
- API route try/catch coverage is **fully clean** — every one of the seven `route.ts` handlers has a JSON-error envelope on every parse + fetch path. Audit passes.
- Window-level `error` + `unhandledrejection` listeners ARE wired (`web/lib/error-reporting/init.ts`) and forward to Sentry/OTLP when configured. This is a good safety net but does NOT replace route-scoped `error.tsx` — listeners report, they don't render a fallback.

**Demo blast radius:** the highest-risk route for tomorrow's recording is `/(app)/workflows/[verticalId]/[name]/build` — React Flow canvas, drag-pan-zoom, seed data lookups, AI prompt bar. A bad data path or a runtime exception inside an `@xyflow/react` callback will white-screen the surface. Recommend a single canvas-scoped client `<ErrorBoundary>` before recording.

---

## 1. Route-level `error.tsx` coverage

### Counted

| Status | Count |
|---|---:|
| Route segments with `page.tsx` | **58** |
| Route segments WITH `error.tsx` | **1** (root only) |
| Route segments WITHOUT `error.tsx` | **57** |

### Existing boundary

- `web/app/error.tsx` — root last-resort boundary. Renders `<ErrorShell code="500" variant="server-error">` with retry / view-status / contact-support buttons. Copy is professional and on-brand. **Quality: GOOD**, scope: too wide (catches every throw across the whole app).

### Top-priority route segments missing `error.tsx`

Listed by demo risk (most-likely-to-throw-on-camera first):

| # | Route segment | Why high-risk | Demo impact |
|---|---|---|---|
| 1 | `web/app/(app)/workflows/[verticalId]/[name]/build/` | React Flow canvas (`@xyflow/react`), drag-pan-zoom + seed-data dictionary lookup keyed on URL params, AI prompt bar with toast side-effects | Whole `<AppShell>` unmounts on canvas throw; operator loses left-nav mid-demo |
| 2 | `web/app/(app)/workers/[workerId]/` (and all 9 sub-tabs) | TanStack Query fetches against gateway, redirect → overview, freeze banner on layout, `WorkerPersonalityCard` + `WorkerOverviewContent` + `RecentChangesPanel` all fetch independently | Worker profile is the demo's hero surface; a stale `[workerId]` URL during recording would blow up the page |
| 3 | `web/app/(app)/audit/` + `web/app/(app)/audit/action/[actionId]/` + `.../replay/` | `useAuditFeed` + `useAuditDetail` + `useReplay` — three independent gateway streams; redaction policy + viewer policy hooks; deep-link from filters | "Audit" is THE differentiator surface. A throw here breaks the trust pitch |
| 4 | `web/app/(setup)/onboarding/*` (5 steps) | Onboarding wizard — write-side gateway calls for workspace, provider, vertical, first-hire | Onboarding fail during demo = abandoned signup story |
| 5 | `web/app/auth/verify/` + `web/app/auth/signin/` + `web/app/auth/signup/` | Magic-link verification, token parsing, sessionToken handoff. Any parsing error throws | A throw here white-screens the auth flow — operator can't even get logged in on camera |

### Full list of unprotected route segments

**`(app)` group (authed product surfaces) — 27 missing:**
- `(app)/` (root index)
- `(app)/audit/`
- `(app)/audit/action/[actionId]/`
- `(app)/audit/action/[actionId]/replay/`
- `(app)/billing/`
- `(app)/billing/subscribe/`
- `(app)/brand/`
- `(app)/brand/components/`
- `(app)/cohorts/`
- `(app)/orchestration/`
- `(app)/retro/`
- `(app)/settings/`
- `(app)/settings/data-export/`
- `(app)/settings/members/`
- `(app)/skills/`
- `(app)/skills/[skillId]/`
- `(app)/support/inbox/`
- `(app)/support/inbox/[id]/`
- `(app)/team/`
- `(app)/verticals/[verticalId]/workflows/[name]/`
- `(app)/verticals/[verticalId]/workflows/[name]/diff/`
- `(app)/workers/` (cohort list)
- `(app)/workers/[workerId]/` (and 9 sub-tabs: activity, evals, memory, overview, responsibilities, responsibilities/new, settings, skills, skills/[skillId]/code, skills/[skillId]/canvas, skills/[skillId]/canvas/diff, skills/new, trust, [tab])
- `(app)/workflows/`
- `(app)/workflows/templates/`
- `(app)/workflows/templates/[templateId]/`
- `(app)/workflows/[verticalId]/[name]/`
- `(app)/workflows/[verticalId]/[name]/build/` **← React Flow, P1 risk**

**`auth/` (login flow) — 4 missing:**
- `auth/check-email/`
- `auth/signin/`
- `auth/signup/`
- `auth/verify/`

**`(setup)` (onboarding) — 6 missing:**
- `(setup)/setup/`
- `(setup)/onboarding/welcome/`
- `(setup)/onboarding/workspace/`
- `(setup)/onboarding/connect-provider/`
- `(setup)/onboarding/pick-vertical/`
- `(setup)/onboarding/first-hire/`

**`(hire)` flow — 3 missing:**
- `(hire)/hire/`
- `(hire)/hire/step/[step]/`
- `(hire)/hire/done/[workerId]/`

**Public marketing — 7 missing:**
- `landing/`
- `compare/`
- `pricing/`
- `customers/`
- `changelog/`
- `docs/`
- `maintenance/`

---

## 2. `not-found.tsx` coverage

### Counted

| Status | Count |
|---|---:|
| `not-found.tsx` files in `web/app/` | **1** (root only) |

### Existing

- `web/app/not-found.tsx` — root 404 fallback. Renders `<ErrorShell code="404" variant="not-found">` with Approvals / Workers / Docs suggestions. Quality: **GOOD**.

### Notes

The Next.js convention is to scope `not-found.tsx` per route segment so the surrounding chrome stays in place. Currently every unmatched URL across `(app)`, `auth`, marketing, and onboarding lands on the same shell-less page. **For tomorrow's demo this is acceptable** — the existing root 404 is professional and on-brand. Scoped 404s are a v1.x polish item.

Worth noting: `/(app)/audit/action/[actionId]/page.tsx` already implements an **in-shell 404** via `<AuditActionNotFound>` (component-level fallback when the action ID is missing). Good pattern — uses Next.js' framework not-found is intentionally skipped so the audit-page chrome stays visible.

---

## 3. `global-error.tsx`

**Status: MISSING.**

Search: `find web/app -name 'global-error.tsx'` returns nothing.

`global-error.tsx` is the outermost boundary — it catches throws in the root `app/layout.tsx` itself (e.g. inside `<AuthProvider>`, `<QueryProvider>`, `<ThemeProvider>`, `<CommandPaletteProvider>`, `<ErrorReportingInit>`). When the root layout itself throws, `app/error.tsx` cannot catch it because `error.tsx` only wraps **children** of the layout. The result is the Next.js default error screen (white background + monospace stack trace). Mid-demo this is the catastrophic state.

The root layout in `web/app/layout.tsx` mounts **five providers** plus calls Google Fonts + `ErrorReportingInit`. Any throw in those (provider config error, font loader hiccup, fetch in `init.ts` blocking render) shows the bare Next.js error screen on camera.

---

## 4. Client-side error boundaries

**Status: ZERO usage.**

Searches:
- `rg 'ErrorBoundary' web/` → 0 hits
- `rg 'componentDidCatch' web/` → 0 hits
- `rg 'getDerivedStateFromError' web/` → 0 hits
- `rg 'react-error-boundary' web/` → 0 hits
- `package.json` does NOT include `react-error-boundary`

**What this means in practice:**
- Any client-side throw inside any component bubbles up to the nearest `error.tsx` (only root exists today)
- React Flow canvas at `/(app)/workflows/[v]/[n]/build` has no canvas-local boundary
- TanStack Query failures bubble through `useAuditFeed`, `useAuditDetail`, `useReplay`, `useViewerPolicy`, all the worker hooks (no boundary, error state must be handled inline)
- Window-level `error` + `unhandledrejection` listeners in `web/lib/error-reporting/init.ts` DO catch + report unhandled async/window-level throws to Sentry/OTLP — but they do not render a fallback UI

**Highest-leverage single addition:** wrap `<ReactFlow>` in `/(app)/workflows/[v]/[n]/build/page.tsx` with a client-side `<ErrorBoundary>` that falls back to a "Canvas couldn't load — view as timeline" panel pointing to the timeline route. This single boundary protects the highest-risk surface in the demo without touching framework conventions.

---

## 5. API route error handling

**Status: CLEAN.**

| Route | `try` blocks | Coverage |
|---|---:|---|
| `app/api/auth/verify/route.ts` | 2 | body parse + fetch upstream — both protected |
| `app/api/auth/signup/route.ts` | 3 | body parse + fetch upstream + JSON parse — all protected |
| `app/api/auth/[...nextauth]/route.ts` | 0 | **Delegates to `@/auth.handlers`** — NextAuth owns error handling. Acceptable. |
| `app/api/workflows/propose/route.ts` | 2 | body parse + streamFromGateway — both protected, fall through to mock in dev |
| `app/api/support/ticket/route.ts` | 4 | body parse + persistTicket + webhook fetch + JSON re-parse — all protected; degraded-mode 202 paths covered |
| `app/api/workers/[id]/freeze/route.ts` | 1 | upstream fetch protected, returns 502 on gateway unreachable |
| `app/api/workers/[id]/unfreeze/route.ts` | 1 | mirror of freeze, same coverage |

Every route either has a try/catch around the gateway fetch OR delegates to a library (NextAuth) that owns the error path. Every error path returns a structured `{ error: { code, message } }` JSON envelope. **No action required.**

---

## Recommendations (prioritized for tomorrow's demo)

### P0 — must-do before recording (≤ 30 minutes of work)

1. **Add `web/app/global-error.tsx`** — the outermost boundary. Without it, any root-layout provider throw shows the Next.js default error screen on camera. Copy: same `<ErrorShell code="500">` as `error.tsx`, with `<html>` + `<body>` tags (required for global-error). One file, ~30 LOC.

2. **Add `web/app/(app)/workflows/[verticalId]/[name]/build/error.tsx`** — React Flow canvas is the highest-throw surface in the demo. Fallback copy: "We couldn't render the canvas. View this workflow as a timeline → [link to `/workflows/<v>/<n>`]." This keeps the operator's flow intact (the timeline page works without React Flow).

### P1 — nice-to-have before recording

3. **Add `web/app/(app)/error.tsx`** — scoped boundary for the whole authed product. Catches throws anywhere under `(app)/` and keeps the marketing surfaces' separate error path clean.

4. **Add `web/app/auth/error.tsx`** — catches throws in signup/verify/signin. Critical because if magic-link verify throws, the operator can't log in on camera. Fallback should offer a "Resend magic link" button.

### P2 — v1.x polish (post-demo)

5. **Per-route `not-found.tsx`** in `(app)/`, `(setup)/`, and `auth/` — scoped 404s with surrounding chrome intact.

6. **Per-segment `error.tsx`** for the remaining 53 route segments — small repeatable copy. Could be a single brief: "add `error.tsx` to every authed product route."

7. **Install `react-error-boundary`** + introduce a project convention for component-level boundaries around risky children (any `@xyflow/react`, `recharts`, dynamic-imported chart). Treat fault-isolated UI islands as a v1.x pattern.

### One-line recommendation

> **Add `web/app/global-error.tsx` and `web/app/(app)/workflows/[v]/[n]/build/error.tsx` before tomorrow's recording. Both use the existing `<ErrorShell />` component. Total cost: 30 minutes. Eliminates the two most-likely-on-camera white-screen paths.**

---

## Appendix — file paths referenced

- `~/code/argo/web/app/error.tsx` (existing root error boundary, GOOD)
- `~/code/argo/web/app/not-found.tsx` (existing root 404, GOOD)
- `~/code/argo/web/components/error/ErrorShell.tsx` (reusable error shell)
- `~/code/argo/web/components/error/ErrorIllustration.tsx`
- `~/code/argo/web/lib/error-reporting/init.ts` (Sentry/OTLP window-level listener, GOOD)
- `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (P0 React Flow canvas, no boundary)
- `~/code/argo/web/app/layout.tsx` (root layout — five providers, no global-error catches throws here)
- `~/code/argo/web/app/api/*/route.ts` (all 7 API routes — try/catch CLEAN)
