# Edge-state audit — wave 1

**Date:** 2026-05-20
**Auditor:** Wave-1 audit agent #5
**Scope:** Every in-product page under `web/app/(app)/` plus `(setup)/onboarding/` and `(hire)/hire/`. Three edge states per page: EMPTY (no data yet), LOADING (fetch in flight), ERROR (5xx / network).
**Method:** read-only static analysis of `page.tsx` / route files + the immediate child components that render the state branches.

Legend:
- HANDLED — explicit branch in code, screenshot-worthy (skeleton/empty/error component) — `✅`
- PARTIAL — handles state but is generic, missing retry, or missing illustration — `⚠️`
- MISSING — would render blank, undefined, or crash if the state hit — `❌`
- N/A — page uses static seed data, no fetch states applicable — `N/A`

## TL;DR

**21 ❌ MISSING cells out of 90 audited (3 states × 30 pages).** 5 ⚠️ PARTIAL. 39 ✅ HANDLED. 25 N/A (static seed pages — these have no live data and ship pre-populated).

The biggest gap is the **workflow surfaces** (`/workflows`, `/workflows/templates`, `/workflows/[v]/[name]`, `/workflows/[v]/[name]/build`): they were authored against static seed data with no plan for live fetch. When PR #1860's gateway routes are wired in, every workflow page will need empty/loading/error states added in the same PR or it will demo-fail.

Worker-tab leaf surfaces (overview / activity / memory / skills / trust / settings / evals) and the approvals inbox + audit timeline are the strongest — all three states explicitly handled with retry buttons.

## Matrix

| Page | Empty | Loading | Error |
|---|---|---|---|
| `/` (approvals inbox) | ✅ | ✅ | ✅ |
| `/team` | ✅ | ✅ | ❌ |
| `/workers` (zero-state) | ✅ | ✅ | ❌ |
| `/workers/[workerId]` (redirect to overview) | N/A | N/A | N/A |
| `/workers/[workerId]/overview` | ⚠️ | ✅ | ✅ |
| `/workers/[workerId]/activity` | ✅ | ✅ | ✅ |
| `/workers/[workerId]/memory` | ✅ | ✅ | ✅ |
| `/workers/[workerId]/skills` | ⚠️ | ✅ | ✅ |
| `/workers/[workerId]/trust` | ⚠️ | ✅ | ✅ |
| `/workers/[workerId]/settings` | ⚠️ | ✅ | ✅ |
| `/workers/[workerId]/evals` | ✅ | ✅ | ✅ |
| `/workers/[workerId]/[tab]` (invalid tab) | N/A (notFound) | N/A | N/A |
| `/workers/[workerId]/responsibilities/new` | N/A | ✅ | ✅ |
| `/workers/[workerId]/skills/new` (ghostwriter) | N/A | ❌ | ❌ |
| `/workers/[workerId]/skills/[skillId]/code` | ✅ (404 path) | ✅ | ✅ |
| `/workers/[workerId]/skills/[skillId]/canvas` | ✅ (404 path) | ✅ | ✅ |
| `/workers/[workerId]/skills/[skillId]/canvas/diff` | ⚠️ | ✅ | ✅ |
| `/cohorts` | ✅ | ✅ | ❌ |
| `/cohorts/[id]` | N/A — uses detail drawer not a route | N/A | N/A |
| `/workflows` (catalog) | N/A (static seed) | N/A | N/A |
| `/workflows/[v]/[name]` (viewer) | ⚠️ (not-found only) | N/A (no fetch) | N/A (no fetch) |
| `/workflows/[v]/[name]/build` (builder) | ⚠️ (not-found only) | N/A (no fetch) | N/A (no fetch) |
| `/workflows/templates` | ✅ (filtered-empty) | N/A (no fetch) | N/A (no fetch) |
| `/workflows/templates/[templateId]` | ⚠️ (not-found only) | N/A | N/A |
| `/verticals/[v]/workflows/[name]` (server) | ✅ | N/A (server-rendered, no client loading) | ✅ (via 404 + global error.tsx) |
| `/verticals/[v]/workflows/[name]/diff` (server) | N/A | N/A | ✅ (rethrows to error.tsx) |
| `/audit` | ✅ | ✅ | ✅ |
| `/audit/action/[actionId]` | ✅ (notFound variant) | ✅ | ✅ |
| `/audit/action/[actionId]/replay` | N/A (action-trigger) | ✅ | ✅ |
| `/skills` (marketplace) | ⚠️ | ✅ | ✅ |
| `/skills/[skillId]` | N/A (stub) | N/A | N/A |
| `/settings` (landing) | N/A (static tile grid) | N/A | N/A |
| `/settings/data-export` | N/A | N/A | ❌ |
| `/settings/members` | N/A (static seed) | N/A | N/A |
| `/settings/workspace` (not shipped) | — | — | — |
| `/settings/notifications` (not shipped) | — | — | — |
| `/settings/security` (not shipped) | — | — | — |
| `/settings/providers` (not shipped) | — | — | — |
| `/settings/connectors` (not shipped) | — | — | — |
| `/billing` | N/A | ✅ | ✅ |
| `/billing/subscribe` | N/A | ✅ | ✅ |
| `/hire` (wizard step 1) | N/A | ⚠️ (Suspense null) | ❌ |
| `/hire/step/[step]` | N/A | ⚠️ (invariant redirect) | ❌ |
| `/hire/done/[workerId]` | N/A | ⚠️ | ❌ |
| `/onboarding/welcome` | N/A | N/A | N/A (no fetch) |
| `/onboarding/workspace` | N/A | ✅ | ✅ (inline form error) |
| `/onboarding/pick-vertical` | N/A | ✅ | ✅ |
| `/onboarding/connect-provider` | N/A | ✅ | ✅ |
| `/onboarding/first-hire` | N/A | ✅ | ✅ |
| `/orchestration` | ❌ | ✅ | ✅ |
| `/retro` | ✅ | ✅ | ✅ |
| `/compare` | N/A (static) | N/A | N/A |
| `/brand` | N/A | N/A | N/A |
| `/brand/components` | N/A | N/A | N/A |
| `/support/inbox` | ✅ (per-column) | ✅ (per-column) | ✅ |
| `/support/inbox/[id]` | — | — | — |
| (Search results — no route exists) | — | — | — |

## Worst offenders (pages with ≥ 2 missing or partial edge states)

1. **`/workflows/[v]/[name]` (workflow viewer)** — `⚠️ N/A N/A`. Reads `SEED_WORKFLOWS[key]` from a const map. No client fetch, no skeleton, no error path. When PR #1860 wires the live gateway endpoint, all three states ship as ❌ unless the brief explicitly adds them.
2. **`/workflows/[v]/[name]/build` (workflow builder)** — same as viewer. The class-leading visual canvas has zero data-fetch states. A 500 from the proposed gateway PUT will land at the global `error.tsx` boundary, ejecting the user out of the canvas mid-edit — the worst possible UX moment.
3. **`/workflows/templates/[templateId]` (template detail)** — handles only "not found." When templates move off `WORKFLOW_TEMPLATES` const and onto a gateway endpoint, no loading/error states exist.
4. **`/hire` + `/hire/step/[step]` + `/hire/done/[workerId]`** — Suspense fallback is `null`, draft fetch fail throws to global error boundary. The wizard is the post-onboarding conversion surface and demo-critical. A draft hydration failure on first paint = white screen.
5. **`/workers/[workerId]/skills/new` (ghostwriter)** — no error handling visible in the page. If the L4 ghostwriter draft endpoint times out, the form just stays in submitting state forever with no recovery hint.
6. **`/orchestration`** — handles error + loading but no "no orchestrations yet" empty state — a brand-new tenant sees `!data && !error → LoadingState` forever.
7. **`/team`, `/workers`, `/cohorts`** — empty + loading both excellent but no error branch. If `/api/v1/team` returns 500, `teamQuery.data?.workers ?? []` falls through to the empty state — a brand-new tenant and a 500 outage look IDENTICAL to the operator. Demo risk.
8. **`/settings/data-export`** — single `downloading` boolean, no fetch error surfaced. If the audit bundle download fails, the button just re-enables silently.
9. **`/workers/[workerId]/overview`** (the personality card, recent changes panel) — the personality card has no isError branch at all; if `useWorkerPersonality(workerId)` rejects, the card silently does not paint while the rest of the page renders fine.

## Pages with all three states locked (gold standard, copy this pattern)

- `/` (approvals inbox) — `LoadingState`, `ErrorState` with retry, `EmptyState` with idle/filtered variants.
- `/audit` — same pattern, plus `RedactionBanner` overlay.
- `/audit/action/[actionId]` — `DetailFallback`, `DetailErrorState`, `AuditActionNotFound`.
- `/workers/[workerId]/activity` — pristine empty + filtered empty + retry-button error.
- `/workers/[workerId]/memory` — illustrated EmptyState + skeleton stack + retry error.
- `/support/inbox` — per-column loading + per-column "No tickets" empty + page-level error banner.

## Recommended additions — priority order

### P0 — pages on the v1 demo journey

1. **`/team` — add ErrorState.** File `web/app/(app)/team/page.tsx:168`. Pattern: copy `ErrorState` from `web/app/(app)/page.tsx:462`. Copy: "Couldn't reach the gateway — your teammates are still working. Refresh to try again."

2. **`/workers` — add ErrorState.** File `web/app/(app)/workers/page.tsx:29`. The current `if (teamQuery.isPending || hasWorkers)` branch silently treats network failure as "still loading" forever. Same retry pattern as `/team`.

3. **`/cohorts` — add ErrorState.** File `web/app/(app)/cohorts/page.tsx:56`. Insert `cohortsQuery.isError` branch before the empty/grid check. Reuse `<EmptyState>` styling with the destructive border treatment from `web/app/(app)/audit/page.tsx:360`.

4. **`/hire` + `/hire/step/[step]` + `/hire/done/[workerId]` — replace Suspense `null` with a wizard skeleton, and add a draft-load error fallback.** Files `web/app/(hire)/hire/page.tsx:25`, `step/[step]/page.tsx`, `done/[workerId]/page.tsx`. The hire wizard is the post-onboarding conversion surface — a white screen on draft hydration failure (e.g., `localStorage` corruption, gateway 500 on draft GET) is the worst demo-recording risk. Add: "We couldn't pick up where you left off — start fresh?" + button to clear draft and restart.

5. **`/workers/[workerId]/skills/new` (ghostwriter) — wrap `<GhostwriterForm>` in an error boundary** that catches stream timeouts and shows a "Couldn't reach the AI ghostwriter — try again or cancel" panel. File `web/app/(app)/workers/[workerId]/skills/new/page.tsx:39`.

### P1 — pages adjacent to the demo journey

6. **`/workflows` (catalog) — plan for live data.** Currently `SEED_WORKFLOWS` const. When PR #1860 wiring lands, add `useWorkflows()` hook with all three states. Until then, mark as N/A.

7. **`/workflows/[v]/[name]` (viewer) — wire `useWorkflow(vertical, name)` and add states.** File `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:431`. Today the lookup is `SEED_WORKFLOWS[key]` → "Workflow not found" if missing. When live, all three states needed. Especially the empty: "This workflow has no steps yet — open the builder to add one."

8. **`/workflows/[v]/[name]/build` — same as viewer, plus an autosave-state indicator** for the visual builder. Saving a 6-step workflow mid-edit and silently failing is unacceptable.

9. **`/workers/[workerId]/overview` (personality card)** — add `isError` and `isPending` branches to `<WorkerPersonalityCard>`. File `web/components/skycot/worker/WorkerPersonalityCard.tsx`. Pattern: silent-fail-card is the worst pattern. Either skeleton-then-render or skeleton-then-error-card.

10. **`/orchestration` — add a true empty state** for the "no orchestrations yet" case. File `web/app/(app)/orchestration/page.tsx:82`. The `!data && !error → LoadingState` branch never resolves for empty tenants.

11. **`/skills` (marketplace) — disambiguate the empty.** File `web/app/(app)/skills/page.tsx:379`. The current empty says "No skills match {query}" — but if the registry is genuinely empty (new tenant, no seed), this reads "No skills match" with no query. Add an `if (skills.length === 0 && query === "") return <PristineEmpty>` branch.

12. **`/settings/data-export` — surface download failure.** Today the button toggles back to enabled silently. Add a toast or inline error after the `setDownloading(false)` in the catch block.

### P2 — robustness for the long tail

13. **`/workers/[workerId]/trust` — empty state** for "no signals yet" (new tenant, worker hired this morning). Today the `<TrustGradeCard>` renders with mostly-empty data. Add a pristine variant: "Trust grade builds as I ship work — check back in a few hours."

14. **`/workers/[workerId]/settings` — `Approval policy section` empty handling.** When `policiesQuery.data?.rows` is empty array (new worker), the section may render with zero rows and no explanation. Confirm + add empty state if missing.

15. **`/workers/[workerId]/skills/[skillId]/canvas/diff` — empty state** when the two versions are identical (`leftIr === rightIr`). Today the diff just renders zero changes silently.

## Notes on the global error boundary

`web/app/error.tsx` catches anything that throws past a page boundary. It renders the ErrorShell at `web/components/error/ErrorShell.tsx` with code "500", a retry button, and links to status.skycot.com / support@skycot.com. This is the safety net — but it ejects the user out of context. The fixes above are about catching errors locally and offering a retry that stays on the page.

Server pages that throw `ApiError` (e.g. `/verticals/[v]/workflows/[name]` line 154-160) intentionally rethrow to the boundary. That's the right call for server-side fetches because the data is required for first paint — there is no skeleton-then-retry pattern for SSR.

## Demo-recording risk ranking (most likely to bite during `docs/skycot/demos/v1-mission-mvp.mp4`)

1. **`/team` 500 → looks identical to empty tenant.** If the gateway hiccups during the recording, the operator's "look at my 8 teammates" demo silently becomes "look at this empty state." Highest fix priority.
2. **`/hire` white screen on draft hydration failure.** Conversion path — must never fail mid-recording.
3. **`/workflows/[v]/[name]/build` error from gateway PUT.** Class-leading differentiator. If the operator hits "save" mid-recording and the gateway 500s, the user gets ejected to the global error page.
4. **`/orchestration` infinite loading on empty tenant.** If the demo tenant is freshly reset, the orchestration tab will spin forever.
5. **`/skills` "no skills match" on empty registry.** First-time-user reads "your search returned no results" — actually means "you have no skills yet, here's how to add one."

## Conclusion

The TanStack-Query-backed pages with explicit `isError` / `isPending` / `length === 0` branches are excellent. The static-seed workflow surfaces and the wizard flows are the gap. Add the 5 P0 fixes before the demo recording.
