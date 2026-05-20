# Wave-8 Audit — Error Recovery Flows

Date: 2026-05-21
Auditor: Wave-8 audit agent #2 (re-spawn #2)
Scope: `web/` — read-only audit of error recovery UX
Surfaces audited: `/`, `/workers/[id]` mutations, `/workflows/.../build` Save+publish + Test run, `/hire` mid-genesis, `/onboarding` 4 endpoints, `/audit` drill-in for deleted action

---

## Executive verdict

The error-handling story is **two-tier**:

- **Page-level (route segment errors)**: Strong. 9 Next.js `error.tsx`
  boundaries + a `global-error.tsx` outer crash boundary + a polished
  `<ErrorShell>` component with "Try again / Back to X / Contact
  support" actions. The 5xx-server-error / not-found / maintenance /
  offline shells are brand-loud, accessible, and consistent.
- **Mutation-level (action error recovery)**: Weak in load-bearing
  places. Several mutations silently succeed when the network is dropped
  (return mock results in production), silently roll back optimistic
  updates without any toast (approve / refuse / undo on `/`), and
  silently swallow PATCH failures ("non-blocking" on hire Step 5
  channel-prefs). 401 token expiry has no global re-auth handler —
  the user sees an obscure "HTTP 401" string and the middleware does
  not detect expired tokens, only missing cookies.

---

## Counts

| Metric                                                          | Count |
| --------------------------------------------------------------- | ----- |
| Route-segment error.tsx boundaries                              | 12    |
| Product pages (excl. marketing)                                 | 58    |
| Coverage = error.tsx parent : pages                             | ~21%  |
| `error.tsx` boundaries that use `<ErrorShell>` + retry          | 9     |
| Files using `ApiError` (typed gateway error envelope)           | 23    |
| Mutation hooks with `onError` toast                             | ~3 (Step4 hire commit, TeammateCard, Connector flows) |
| Mutation hooks with **silent** rollback (cache revert, no toast) | 4 (approve, refuse, undo, batch-approve) |
| `lib/api/*` functions that fall through to mock-offline on TypeError | 14 |
| API functions that silently mock-fallback **even in production** | 3 (`submitDecision`, `submitUndo`, `fetchApprovals` mutations path) |
| 401 / token-expiry → re-auth prompt                             | 0 |
| Routes that auto-redirect to `/offline` when navigator.onLine = false | 0 |
| Routes that listen to `online`/`offline` browser events         | 0 |

**Silent-fail paths: 5–7** (depending on how strictly counted).
**Paths with explicit recovery action (toast + retry, or page-level
"Try again" CTA): ~14** route-segment recoveries + ~3 mutation-level
retries.

---

## Infrastructure inventory (the good)

### 1. Typed gateway error envelope — `lib/api/client.ts` + `lib/api/types.ts`

`ApiError` carries `status`, `code`, `message`, `body`. The wrapper
`request<T>()` parses the gateway's `{ error: { code, message } }`
envelope on 4xx and surfaces it to callers. 23 files import `ApiError`
and branch on `.status === 404` — primarily to fall through to
demo/sample data when the gateway isn't running. Pattern is clean and
well-tested (`lib/api/client.test.ts`).

### 2. `<ErrorShell>` component — `components/error/ErrorShell.tsx`

Single shared layout: big code, title, description, action row,
inline-SVG illustration via `<ErrorIllustration variant>`. Used by:

- `app/error.tsx` (App Router boundary, 500 + Try again / View status /
  Contact support)
- `app/global-error.tsx` (outermost crash, renders own `<html>` +
  inline styles since root layout failed — minimal, just Reload)
- `app/not-found.tsx`
- `app/maintenance/page.tsx`
- `app/auth/error.tsx`
- 7 segment-scoped boundaries below

### 3. Segment error boundaries

```
app/error.tsx                                                  (root)
app/auth/error.tsx                                             (auth)
app/(setup)/onboarding/error.tsx                               (onboarding wizard)
app/(hire)/hire/error.tsx                                      (hire wizard)
app/(app)/cohorts/error.tsx
app/(app)/skills/error.tsx
app/(app)/settings/error.tsx
app/(app)/audit/error.tsx
app/(app)/audit/action/[actionId]/error.tsx
app/(app)/audit/action/[actionId]/replay/error.tsx
app/(app)/workers/[workerId]/error.tsx
app/(app)/workflows/[verticalId]/[name]/build/error.tsx
```

Each has the same pattern: `useEffect(()=>console.error(...))` for
logging, an `<ErrorShell>` with a Try-again button + a back-link, and
brand-friendly copy ("Onboarding hit a snag", "We couldn't load this
worker", "The builder couldn't open this workflow").

**Gaps**: no `error.tsx` for `/workflows` list, `/workflows/templates`,
`/team`, `/customers`, `/orchestration`, `/retro`, `/support/inbox`, `/replay`,
`/workers/[workerId]/[tab]` sub-routes, `/onboarding/welcome`,
`/onboarding/workspace`, `/onboarding/connect-provider`,
`/onboarding/pick-vertical`, `/onboarding/first-hire` (one shared
`(setup)/onboarding/error.tsx` exists at the segment level, good).
A page-level error in those routes falls through to the root
`app/error.tsx` — generic 500.

### 4. Global crash boundary — `app/global-error.tsx`

Inline-styled, no Tailwind dependency, "We hit a wall" Reload-only.
Production-grade. Properly handles the case where the root layout
itself failed.

### 5. WebSocket / live updates — `lib/api/websocket.ts`

`ManagedWebSocket<T>` has heartbeat (30s ping, 10s pong deadline),
exponential backoff with ±25% jitter capped at 30s, and
fallback-to-polling after 3 failures within 60s. Status surface
(`"connecting" | "open" | "polling" | "closed"`) is wired to a visual
`<LiveIndicator>` on `/`. The inbox WS path is the only place that
properly tells the user "we are reconnecting" via that pill.

### 6. Error reporting — `lib/error-reporting/init.ts`

`window.addEventListener("error" / "unhandledrejection")` installed on
boot, scrubbed for email/JWT/sk-/phone PII before posting to Sentry or
OTLP. Good operator-side visibility, no user-side UI.

### 7. Toast surface — `lib/ui/use-toast.ts`

Typed Sonner wrapper: `{ success, error, info, warning, loading,
promise, dismiss }` with sane durations (error = 8s, warning = 6s).
Available everywhere — but **underused** in mutation onError paths.

### 8. Offline shell — `app/offline/page.tsx`

Audiowide-hero "You're offline" page with brand blobs + Try again
reload button. **NOT WIRED** — no code routes the user here. There is
no `navigator.onLine` listener, no `online`/`offline` event handler,
no service worker. The page exists as a future hook for a Track-H
hardening brief.

---

## Dimension-by-dimension findings

### Dimension 1 — Network drop mid-action

**Verdict: silent fail in most mutation paths**

**a. Approvals decide / refuse / undo (`/`)** — 4 silent paths.
`lib/api/approvals.ts:144-162` (`submitDecision`), `:164-179`
(`submitUndo`), `:119-135` (`fetchApprovals` decision mutations
path) all fall through to a synthesised `{ ok: true, source:
"mock-offline" }` on `TypeError` (network drop) — **and this
fallback is NOT guarded by `isStrictProduction()`**. In prod, the
operator clicks Approve, sees the card flip to approved, gets no
toast, no error indicator, and the gateway never recorded the
decision. The audit trail is silently missing.

The other mock-offline fallbacks (team, cohorts, audit, skills, etc.)
DO honour `isStrictProduction()` and re-throw in prod — only
approvals' decision-mutations don't. Compare `lib/api/team.ts:62-66`
(prod-safe) to `lib/api/approvals.ts:156-159` (always-mock). This is
the highest-stakes silent failure on the platform — a customer
demoing a buy could click 5 approvals, get optimistic green ticks,
and have zero of them landed on the gateway.

**b. `useSubmitDecision` onError rollback is silent** —
`lib/hooks/use-approvals.ts:62-64`:

```ts
onError: (_err, _vars, ctx) => {
  if (ctx?.prev) qc.setQueryData(APPROVALS_KEY, ctx.prev);
},
```

The cache rolls back, the card visually un-approves, no toast fires.
The operator just sees their click "undo itself" with no
explanation. Same in `useUndoDecision` (line 178-180) and the batch
`useBatchSubmitDecisions` (failures are counted in `state.failed` but
no toast — the `BatchGroupBlock` is supposed to render "Approved 4 —
1 failed" inline but that rendering is in `BatchUiState` not a toast,
and the operator may have already navigated away).

**c. Hire Step 5 channel PATCH is "non-blocking"** —
`components/skycot/hire/Step5Connect.tsx:141-150`:

```ts
try {
  await apiPatch(`/api/v1/workers/${workerId}`, payload);
} catch {
  // Save failure is non-blocking — the worker is already hired.
}
```

User picks primary/fallback channels, clicks Continue, the PATCH
fails, the user is forwarded to `/hire/done` thinking the channels
are saved, and nothing tells them they need to re-configure on the
worker-profile page. The comment justifies it but the user has no
breadcrumb.

**d. Onboarding workspace/provider/first-hire mutations** — clean.
`lib/onboarding/use-provision.ts`, `use-provider-key.ts`,
`use-first-hire.ts` throw `ApiError`; the calling pages
(`workspace/page.tsx:36-42`, `connect-provider/page.tsx:35-43`,
`first-hire/page.tsx:42-59`) show an inline `<p
data-testid="onboarding-*-error">` with the message + "Please try
again". Good — inline + retryable.

**e. WS drops** — `<LiveIndicator>` shows the status pill ("Live" /
"Reconnecting" / "Polling") on the inbox header. The polling
fallback masks most drops; no toast is needed.

### Dimension 2 — Token expiry / 401

**Verdict: silent 401 — no global re-auth handler**

- `lib/api/client.ts:45-63` parses the 4xx envelope but treats 401
  exactly like any other 4xx. `ApiError.status === 401` propagates to
  whichever caller; only `lib/auth/use-verify.ts:29` distinguishes
  401 (and only for magic-link verification).
- `middleware.ts:40-44` only checks for cookie **presence**, not
  validity. An expired gateway-bearer JWT in the cookie still lets the
  user past middleware; the first API call fails with 401; the page
  shows a generic ErrorState ("Couldn't load worker — HTTP 401") with
  a Try-again button that will 401 forever.
- No `useEffect` anywhere triggers `signIn()` or `router.push("/auth/signin")` on 401.
- The `auth.config.ts` callback returns the gateway token verbatim in
  the JWT — there is no refresh-token flow; once the gateway token
  expires the user has to manually log out and log back in, and the
  UI gives them no indication that's what they need to do.
- `lib/sample-notifications.ts:60-65` has an `oauth-needs-reauth`
  notification type for **connector** OAuth expiry (Gmail, etc.) —
  but no equivalent for the user's own session.

### Dimension 3 — Validation error display

**Verdict: solid on auth + onboarding forms, inconsistent elsewhere**

- **Inline + aria-invalid + aria-describedby + role="alert"**: signup,
  signin, onboarding workspace, onboarding connect-provider,
  onboarding first-hire, hire StructuredAdjustDrawer, hire DutyEditorRow
  (cron), worker BudgetSection (daily/monthly), Step1Capture (cron),
  GhostwriterForm, canvas/nodes/shared inputs. Pattern: a `<p
  role="alert" id="*-error">` directly below the field, message text
  in destructive red. ~16 fields confirmed.
- **Toast-only or no validation**: many list filter pickers, search
  inputs (which don't need validation), and the workflow builder
  (which has no field-level inputs in the canvas demo state).
- **Validation messages are user-friendly**:
  "Anthropic API keys start with sk-ant-.",
  "Workspace name and company name are required.",
  "Tell us a bit about who you'd like to hire.",
  "Pick a template first." — all plain-language. Good.

### Dimension 4 — 5xx from gateway

**Verdict: friendly per-page error.tsx where present, raw error string
where absent**

- Where `error.tsx` exists: `<ErrorShell>` with "Something went wrong
  on our side. We've been notified." + Try again / View status /
  Contact support. Status URL is a hardcoded
  `https://status.skycot.com` (does that page exist?), support email
  is `support@skycot.com`. Good UX.
- Where it doesn't (37+ pages, see infrastructure section): the
  parent root `app/error.tsx` catches and shows "Something went wrong
  on our side" — still ErrorShell, just less context-specific
  ("Worker not found" vs "Something went wrong"). Acceptable but the
  "back to" link goes to root.
- Query-level isError states: most TanStack-Query callers render a
  small inline ErrorState card (`AlertCircle + Couldn't load X +
  message + Try again link`) — see `WorkerLayoutShell.tsx:128-147`,
  `app/(app)/audit/page.tsx:360-377`, `app/(app)/page.tsx:472-487`.
  These don't escalate to a page boundary because the error came
  from the data layer, not from React rendering.

### Dimension 5 — Recovery action coverage

**Verdict: every page-level error has Try again + Back; mutation
errors are spotty**

| Surface | Try again | Back / Reset | Contact support |
| ------- | --------- | ------------ | --------------- |
| `app/error.tsx`                                            | Yes | View status | Yes |
| `app/global-error.tsx`                                     | Reload | — | Yes |
| `app/not-found.tsx`                                        | — | Yes | Yes |
| `app/auth/error.tsx`                                       | Yes | Yes | — |
| `app/(setup)/onboarding/error.tsx`                         | Yes (this step) | Restart | — |
| `app/(hire)/hire/error.tsx`                                | Yes | Back to team | — |
| `app/(app)/audit/error.tsx`                                | Yes | Yes (assumed) | — |
| `app/(app)/audit/action/[actionId]/error.tsx`              | Yes | Back to audit | — |
| `app/(app)/audit/action/[actionId]/replay/error.tsx`       | Yes | — | — |
| `app/(app)/workers/[workerId]/error.tsx`                   | Yes | Back to team | — |
| `app/(app)/workflows/[verticalId]/[name]/build/error.tsx`  | Reload canvas | Back to all workflows | — |
| Approve / Refuse / Undo silent rollback                    | — (no toast) | — | — |
| Hire Step 4 commit                                         | Yes (in-step retry) | — | — |
| Hire Step 5 channels PATCH                                 | — (silently swallowed) | — | — |
| Onboarding inline form errors                              | Yes (resubmit) | — | — |
| Worker freeze / unfreeze                                   | Yes (resubmit; inline error in dialog) | Cancel | — |

---

## Top 5 missing-recovery flows

### 1. CRITICAL: Approvals decision silent success in production

`lib/api/approvals.ts:144-179` (`submitDecision` + `submitUndo`) fall
through to `source: "mock-offline"` on any `TypeError` (network drop)
without checking `isStrictProduction()`. The optimistic-update path
in `lib/hooks/use-approvals.ts` flips the card; the user sees a green
tick; the gateway never received the decision. No audit trail. **This
is the highest-stakes silent fail** — directly contradicts the
"reliability" and "observability" priorities in CLAUDE.md.

Fix shape: same as `lib/api/team.ts:62-66` — guard with
`!isStrictProduction() && isMissingOrUnreachable(err)` and re-throw
in prod. Pair with an `onError` toast in `useSubmitDecision` so even
when the rollback fires the user is told why.

### 2. HIGH: 401 token expiry has no re-auth path

No middleware refresh, no client-side 401 → `signIn()` interceptor,
no toast offering "Sign in again". The user gets a generic ErrorState
with a Try-again button that re-fires the 401. They have to manually
navigate to `/auth/signin`. Common case: laptop wakes from sleep
after the gateway JWT expired (24h default).

Fix shape: wrap the `lib/api/client.ts:request()` to detect 401, fire
NextAuth's `signOut({ redirect: false })`, surface a toast with a
"Sign in again" action linking to `/auth/signin?from=<current>`.
Optionally pair with a refresh-token flow in `auth.config.ts`.

### 3. HIGH: Optimistic mutation rollback is silent

`useSubmitDecision` / `useUndoDecision` / `useBatchSubmitDecisions`
all rollback on `onError` without firing a toast
(`lib/hooks/use-approvals.ts:62-64, 178-180`). The card "undoes
itself" visually with zero explanation. Same in `TeammateCard` action
toasts — but those at least toast on error.

Fix shape: add `onError: (err, vars, ctx) => { qc.setQueryData(...);
toast.error("Couldn't save your decision", { description:
err.message, action: { label: "Try again", onClick: () =>
mutate(vars) } }); }`.

### 4. MEDIUM: Hire Step 5 channel-prefs PATCH silently swallowed

`components/skycot/hire/Step5Connect.tsx:141-150` calls `apiPatch`
and catches+ignores the error in a comment that says "non-blocking".
The user thinks their channel preferences are saved. They aren't.

Fix shape: surface a non-blocking warning toast — "We saved your hire
but couldn't save your channel preferences. You can configure them
on the worker profile." — and link to `/workers/<id>/settings`.

### 5. MEDIUM: No `online` / `offline` event handling

`/offline` page exists at `app/offline/page.tsx` but no code routes
to it. No `navigator.onLine` listener, no service worker, no banner
that says "You appear to be offline". If a user loses WiFi mid-task
they see TanStack-Query errors with no contextual "you're offline"
hint. The WS layer's polling-fallback partially compensates but
mutations still fail with raw TypeErrors.

Fix shape: in `app/layout.tsx` or a top-level provider, listen for
`window.addEventListener("offline", ...)` and render a sticky
non-blocking banner ("Working offline — your last changes will sync
when you're back online"). Don't navigate away — the user keeps the
loaded page. Pair with a connectivity check in `request()` to defer
mutations until back online.

---

## Other observations (not in the top 5)

- **No `/workflows` list error boundary**: `/workflows/page.tsx` has
  no sibling `error.tsx`. A render error falls back to root.
- **No `/team` list error boundary**: same.
- **No `/orchestration` error boundary**: page has its own inline
  error UI but no boundary for render-time errors.
- **`/customers/[id]` and `/replay/[id]`**: no boundaries.
- **`handleHeroAction` in `WorkerLayoutShell.tsx:111-117`** is a
  console.warn stub — the menu items don't actually do anything yet.
  Flagged for E3-b / E4-f follow-up briefs; user-visible if a customer
  uses the hero action menu.
- **Worker freeze/unfreeze** — solid pattern: inline error message
  inside the dialog (`FreezeButton.tsx:98-103, 174-178`), Cancel button,
  retry by clicking Freeze again. GitHub-style type-the-id confirm
  for the destructive direction.
- **Hire Step 3 (Genesis drafting)** — best-in-class recovery:
  cosmetic progress timer, "Still working…" hint at 8s, retry panel
  at 30s with "Try again" + "Pick a template instead", 3-retry
  exhaustion drops to a template-picker grid
  (`Step3Drafting.tsx:131-218`). This is the gold-standard pattern
  the rest of the platform should adopt.
- **Audit drill-in for deleted action**: handled cleanly. 404 →
  `<AuditActionNotFound>` (specialised empty state), other errors →
  inline retry card with the gateway message
  (`audit/action/[actionId]/page.tsx:91-101, 176-194`).
- **Workflows build Save+publish + Test run** are intentional
  honest-stub toasts ("Real run streams from gateway in v1.x").
  Not a recovery gap — a wiring gap. The user is correctly told
  nothing happened. (`build/page.tsx:354-372`.)
- **Status URL `https://status.skycot.com` is hardcoded** in
  `app/error.tsx:7`. Does it resolve? If not, the support CTA is
  worse than no link. Verify before launch.

---

## Recommended brief shape

Single brief, ~250 LOC, mostly `web/lib/api/approvals.ts` +
`web/lib/hooks/use-approvals.ts` + new `web/lib/api/client-401.ts`
interceptor + an `online/offline` banner in `web/app/layout.tsx`:

1. Add `isStrictProduction()` guard to `submitDecision` /
   `submitUndo` / `fetchApprovals` mock-fallback paths (15 LOC).
2. Add `onError` toast handlers to the 4 approval mutations (25 LOC).
3. Wrap `lib/api/client.ts:request()` with a 401 → re-auth handler
   that fires `signOut` + redirects to `/auth/signin?from=<path>`
   (40 LOC).
4. Add a top-level `online/offline` listener + sticky banner in the
   app shell (60 LOC).
5. Add `try { … } catch { toast.warning(…) }` to Step 5 channel PATCH
   (10 LOC).
6. (Stretch) Add `error.tsx` to `/workflows`, `/team`,
   `/orchestration`, `/customers/[id]`, `/replay/[id]` — each ~40
   LOC, fully boilerplated. (~200 LOC across 5 files.)

Reliability + observability priorities (CLAUDE.md #1, #2) demand the
first three. The rest are polish for v1.x.
