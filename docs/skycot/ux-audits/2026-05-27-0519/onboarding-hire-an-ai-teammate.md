# Onboarding hire-flow UX audit — "Hire your first AI teammate"

**Auditor**: SKYCOT e2e UX journey testing agent
**Date**: 2026-05-27 05:19 AEST
**Scope**: `/landing` → `/auth/signup` (demo-mint) → `/onboarding/welcome` → `workspace` → `connect-provider` → `pick-vertical` → `first-hire` → `/workers/{id}` → `/team`
**Stack under test**: `web/` Next.js 15.5 SPA (`pnpm dev --turbopack`), gateway `127.0.0.1:7400` (schema v83), demo-mode flags `NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`.
**Classification rule applied**: function-vs-dressing from `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

---

## TL;DR

Visually the 5-step wizard renders the locked F1 editorial-center design crisply and the copy ("Hire your first AI teammate", "Onboard them like a human", "In four minutes…") is strong investor-narrative material. But the **last beat of the narrative is broken**: after clicking "Hire them" the demo wizard sends the user to `/workers/demo-worker-<vertical>-<timestamp>` — a synthetic id the gateway rejects with HTTP 400 (it expects a UUID v7). The detail page spins on **"Loading worker…" forever**, never falls through to either the friendly `WorkerNotFound` shell or the `WorkerHero` / `WorkerPersonalityCard`. The newly-hired Maya/Mason/etc. never appears in `/team` or `/workers` either — both surfaces still show only the 5 hardcoded sample workers (Mariah, Peta, Priya, Jamal, Nori).

**Headline counts**: 4 P0 demo-blockers · 7 P1 visible cracks · 3 P2 polish · 1 P3 nit. **Total: 15 findings.**

**Biggest blocker**: F1 (post-hire `/workers/{id}` infinite-spinner). For an investor watching the wizard's `Hire them → AI teammate profile` moment, the camera lands on a loading spinner instead of "here's Maya, ready to work."

---

## Headline blockers

| # | Sev | Finding | Scene impact |
|---|---|---|---|
| F1 | **P0** | Post-hire `/workers/{synthetic-id}` hangs on "Loading worker…" forever — gateway 400s on non-UUID worker id, fetchWorker has no 400 branch | Onboarding Scene 3 climax — the moment that proves "hire is real" |
| F2 | **P0** | Demo wizard creates zero real backend state — workspace / provider / vertical / first-hire all short-circuit client-side (synthetic ids, no gateway round-trip) | Investor pitch — answers the camera question "does anything actually happen?" with no |
| F3 | **P0** | Demo banner reads "Demo mode — showing sample data. Sign in to see your own workspace." while the operator is sitting INSIDE the post-signup app shell that they just hired Maya from | Onboarding Scene 3 → 1 transition — directly contradicts the narrative |
| F4 | **P0** | `/workers` (sidebar entry) redirects to `/team`, which shows only the 5 hardcoded sample workers. The freshly "hired" worker never appears in the roster | Onboarding Scene 3 → Scene 1 transition — "find your new hire" |

---

## All findings

### F1 [P0] — Post-hire detail page hangs on "Loading worker…" forever

The wizard's `Hire them` button (`/onboarding/first-hire`) calls `useFirstHire`, which in demo mode synthesises `workerId = "demo-worker-<verticalId>-<base36-stamp>"` (e.g. `demo-worker-marketing-intern-mpn1b10z`) and pushes the user to `/workers/<id>`. The detail layout's `fetchWorker` then calls `GET /api/v1/workers/<id>`, which the gateway rejects with **HTTP 400 `bad_request` "invalid worker_id 'demo-worker-marketing-intern-mpn1b10z': invalid character: found 'm' at 3"** — the gateway parses workerId as a UUID v7 (`crates/argo-cli/src/handlers/gateway/...`). `fetchWorker` only converts 404 → `WorkerNotFoundError`; a 400 bubbles up as a raw `ApiError`, but the React Query default retries indefinitely and the layout stays in `isPending=true`. The camera-visible state is a spinner that never resolves — the investor sees the wizard succeed, then sees infinity.

**Reproduction**:
- `pnpm --dir web dev` with the demo env vars from `.claude/launch.json`
- Visit `/onboarding/first-hire` (e.g. via the wizard); fill in any prompt; click "Hire them"
- The URL becomes `/workers/demo-worker-marketing-intern-<stamp>` and the body shows "Loading worker…" forever
- Network tab: repeated `GET /api/v1/workers/demo-worker-...` returning 400

**File**: `web/lib/onboarding/use-first-hire.ts:102-112` (synth id format); `web/lib/api/workers.ts:119-152` (no 400 branch); `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:48` (`isPending` traps the spinner).

**Suggested fix**: Either (a) in demo mode redirect to `/workers/018f67d0-0000-7000-8000-000000000007` (a known sample-worker UUID) so the operator lands on Peta/etc. as a stand-in for the "new hire", or (b) extend `fetchWorker` to treat 400 the same as 404 and surface the friendly `WorkerNotFound` shell, then add a demo-mode synthetic `Worker` overlay to `findSampleWorker` keyed on the `demo-worker-` prefix. Option (a) gives the better camera moment.

---

### F2 [P0] — Demo wizard creates zero real backend state

Every step of the 5-step wizard short-circuits in demo mode and never hits the gateway:

| Step | File | Demo-mode behaviour |
|---|---|---|
| Workspace | `web/lib/onboarding/use-provision.ts:77-87` | Synthetic `tenantId = "demo-tenant-<slug>"`, `workspaceId = "demo-workspace-<slug>"` |
| Provider | `web/lib/onboarding/use-provider-key.ts:33-42` | Returns `{validated: true, testCost: 0.001}` for ANY key matching `sk-ant-*` prefix — no upstream call |
| Vertical | `web/app/(setup)/onboarding/pick-vertical/page.tsx:225-229` | `if (isDemoMode()) skips /onboarding/install-vertical entirely` |
| First hire | `web/lib/onboarding/use-first-hire.ts:114-122` | Returns synthetic `workerId` without calling gateway; no charter, persona, memory schema, or worker row is created |

This is more theatrical than the `feedback_no_theater_in_demo` rule sanctions. The reframe doc says it's OK for "dressing" (workspace names, customer logos, KPI numbers) to be fake, but the **function — what the agents DO** must be real. The wizard's whole pitch ("In four minutes you'll onboard your first AI teammate") is undermined when the four minutes produce nothing — no real tenant in the SQLite store, no vertical pack installed, no worker row, no charter, nothing for the camera to drill into.

**Reproduction**: After completing the wizard, query the gateway:
- `curl -s http://127.0.0.1:7400/api/v1/workers/<id>` → 400 (id format) or 404 (real format unknown)
- `curl -s http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}` (zero installed)

**File**: 4 short-circuit blocks listed above.

**Suggested fix**: Per `feedback_no_theater_in_demo`, rewrite the demo path from "Option A (mock bypass)" to "Option B (real backend seed)" — the wizard should call a single seed endpoint that provisions a tenant + installs the chosen vertical + creates a worker row with a real UUID, then redirect to that real worker's detail page. The provisioning could complete in ≤2s with the gateway already running.

---

### F3 [P0] — "Sign in to see your own workspace" banner inside the post-signup app shell

After the wizard, the user lands on `/workers/{id}` (inside the `(app)` route group). The shared `DemoBanner` (`web/components/skycot/DemoBanner.tsx:43-45`) reads:

> **Demo mode —** showing sample data. **Sign in to see your own workspace.**

The user JUST signed in (demo-mint via `/api/auth/demo-mint`) and JUST went through a 5-step wizard explicitly framed as "onboarding YOUR workforce." The banner contradicts every preceding screen and tells the investor "actually, none of that was real." This is the single most narrative-damaging string in the whole flow.

**Reproduction**: Open any in-app route after demo-mint (e.g. `/workers`, `/team`, `/audit`); banner is at the top of the viewport, gradient-tinted, sticky z-30.

**File**: `web/components/skycot/DemoBanner.tsx:44-45`.

**Suggested fix**: Demo banner copy should match the wizard's frame. Try: "**Demo mode** — your changes are seeded; this workspace resets nightly." Drop "Sign in to see your own workspace." (the user did sign in.)

---

### F4 [P0] — Newly-hired worker doesn't appear in `/workers` or `/team`

The wizard's final CTA pushes to `/workers/<id>` (the detail page). But the canonical roster surfaces — `/workers` (which redirects to `/team` when workers exist, per `web/app/(app)/workers/page.tsx:25-27`) and `/team` — both pull from `useTeam()` which falls back to the hardcoded `SAMPLE_TEAM` array (Mariah, Peta, Priya, Jamal, Nori) in demo mode. The "Maya" the user just described in plain English never shows up. If the camera pans from the detail page back to the roster, the new hire vanishes.

**Reproduction**: After the wizard, click "Workers" or "Team" in the sidebar; the roster shows 5 sample workers; the wizard's prompt ("Maya") is nowhere.

**File**: `web/lib/sample-team.ts:97`; `web/lib/api/team.ts` (read-fallback path); `web/app/(app)/workers/page.tsx:25`.

**Suggested fix**: Same as F2 — wire the wizard to a real seed endpoint that creates a worker row, then the roster reads it back through the normal path. Until then, persist the demo-mode hire in `sessionStorage` and overlay it onto `useTeam()`'s data in demo mode so the new hire actually appears.

---

### F5 [P1] — Wizard offers 5 vertical templates instead of the ~30 the gateway has loaded

The pick-vertical step renders 5 cards (Finance AR, GTM Engineer, Customer Support, Marketing Intern, Sales SDR), labelled "v1 catalogue (offline · 5 verticals)." But the gateway has ~30 verticals loaded — `curl /verticals` returns Bookkeeping Assistant, Customer Onboarding, Data-Entry Clerk, Event Coordinator, Executive Assistant, Fundraising Coordinator, Healthcare Intake, HR Screener, Lead Research, Legal Paralegal, Marketplace Seller, Medical Coder, Ops On-Call, Pull Request Checker, Procurement Assistant, Product Manager, QA Engineer, Real Estate Agent, AI Receptionist, Recruiter, Recruiter Screener, Research Analyst, Sales Follow-up, Social Media Drafter, Tax Prep, and more. The wizard's data layer hits `apiGet("/verticals")` which is **NOT proxied by Next** (`web/next.config.ts:30-34` only rewrites `/api/v1/*`), so the browser fetches `/verticals` on the SPA origin, gets the HTML page back, JSON-parses fails, the catch falls through to `SEED_VERTICALS` (5 hardcoded entries). The "v1 catalogue (offline · 5)" label reads as "offline" — but the gateway is online, just on the wrong path.

Additionally, `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158` has a 7-entry `FALLBACK_VERTICALS` array (adds Recruiter + Marketplace Seller) — but because `useVerticals` always returns ≥1 row (the 5-entry `SEED_VERTICALS`), this 7-entry fallback is **dead code**. Recruiter and Marketplace-Seller never appear in the wizard despite living in two different fallback paths.

**Reproduction**:
- Visit `/onboarding/pick-vertical`; count cards (5)
- `curl -s http://127.0.0.1:7400/verticals | jq '.loaded | length'` → 30+
- Network tab: `GET /verticals` returns the SPA HTML page, not JSON

**File**: `web/lib/api/verticals.ts:226-243` (path mismatch); `web/next.config.ts:30-34` (rewrite scope); `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158` (dead fallback).

**Suggested fix**: Either add `/verticals` to the rewrite list in `next.config.ts`, OR change the path in `fetchVerticals` to `/api/v1/verticals/catalog` (and add that endpoint on the gateway) so it goes through the existing rewrite. Then delete the dead `FALLBACK_VERTICALS` in the page.

---

### F6 [P1] — `WorkerPersonalityCard` is a hardcoded fiction-map keyed by worker name

The `/workers/{id}/overview` "personality card" (locked C2 design, surfaces "Patient · Detail-oriented · …" traits + "Great at" + "Escalate when" + "Signature behaviors") sources every word from a 5-entry hardcoded `PERSONALITIES` map in `WorkerPersonalityCard.tsx`. It is NOT sourced from a charter, persona, memory schema, or vertical manifest — there is no API call. For the 5 sample workers (Mariah, Peta, Priya, Jamal, Nori) the content reads beautifully; for any worker name not in the map (e.g. a real hire named "Maya") it falls through to a generic `DEFAULT_PERSONALITY` with placeholder bullets ("Working through the responsibilities you set", "Asking when something is ambiguous"). This breaks the function-vs-dressing rule because the camera DOES drill into the personality card during Scene 3 — it's the headline of the worker-detail page.

**Reproduction**:
- Visit `/workers/018f67d0-0000-7000-8000-000000000001/overview` (Mariah) — rich personality copy
- Visit `/workers/018f67d0-0000-7000-8000-000000000007/overview` (Peta — newer addition) — `PERSONALITIES` map at lines 35-96 doesn't include Peta; check at line 117 falls through to `DEFAULT_PERSONALITY`
- Generic bullets appear; the locked C2-personality lock contract is violated

**File**: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` (5-entry hardcoded map); `web/components/skycot/worker/WorkerPersonalityCard.tsx:117` (fallback to DEFAULT_PERSONALITY for unknown names).

**Suggested fix**: Long-term, ship a `GET /api/v1/workers/{id}/personality` endpoint sourced from the vertical manifest's `role.persona` field. Short-term, add Peta + Nori to the `PERSONALITIES` map and have the wizard's first-hire route redirect to a real sample worker whose personality entry exists.

---

### F7 [P1] — `meta/error-reporting` cross-origin call fails CORS on every page mount

The error-reporting init code (`web/lib/error-reporting/init.ts:47-62`) constructs the URL as `${NEXT_PUBLIC_GATEWAY_URL}/api/v1/meta/error-reporting` — i.e. directly to `http://127.0.0.1:7400/...` from the browser, bypassing the Next.js rewrite. The Rust gateway has no `OPTIONS` handler (per `next.config.ts:24-29`'s own comment), so the CORS preflight 404s and the GET returns `net::ERR_FAILED`. This fires on every route mount — visible in DevTools network as a recurring failure (≥5 entries per page load) — and pollutes any future error-reporting telemetry with init-failure churn.

**Reproduction**: Open DevTools → Network tab on any in-app route; filter "failed"; see repeated `GET http://127.0.0.1:7400/api/v1/meta/error-reporting [FAILED: net::ERR_FAILED]`.

**File**: `web/lib/error-reporting/init.ts:48-53`.

**Suggested fix**: Replace `${base}/api/v1/meta/error-reporting` with the same-origin path `/api/v1/meta/error-reporting` (which goes through the Next rewrite). The `base` constant becomes dead.

---

### F8 [P1] — "Workers" and "Team" sidebar entries are confusingly duplicative

The sidebar nav lists both **Workers** and **Team** (and Cohorts, and "AI TEAMMATES" in the top brand). `Workers` redirects to `/team` when any worker exists (`web/app/(app)/workers/page.tsx:25-27`) and renders `HireStarterPrompts` when none exist. The post-onboarding redirect goes to `/workers/{id}`, which is fine. But the navigation gives the user three near-synonymous concepts — Workers (the roster but redirects), Team (the actual roster), Cohorts (groups of workers). The wizard refers to "your first worker" / "first hire" / "AI teammate" interchangeably, and the sidebar reflects the same naming confusion. For an investor demo, the camera should not have to explain why "Workers" and "Team" are both there.

**Reproduction**: After the wizard, look at the sidebar; click each in turn.

**File**: `web/app/(app)/_AppShell.tsx` (sidebar config); `web/app/(app)/workers/page.tsx:1-46` (redirect-to-/team logic).

**Suggested fix**: Merge — keep "Team" or "Workers", not both. The brand says "AI TEAMMATES" up top; pick one downstream label and use it everywhere.

---

### F9 [P1] — Hire CTA copy promises "this week" but no schedule/duty is captured

The first-hire step's sub-copy says: "Give them a **name, a role, and what they should do this week**." The example placeholder: "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans." But the wizard sends only `{description: <full string>, vertical_id: <id>}` to the gateway (and in demo mode, doesn't even send that). No schedule / duty / cron is parsed out of "weekly" — the worker's "Schedules" tab on the detail page will show whatever the vertical pack's default duties are, not whatever the operator just typed. The copy is making a promise the wiring doesn't keep.

**Reproduction**: Submit with "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans" — observe that the redirect target (or in demo mode the spinning detail page) doesn't surface a "weekly" schedule for Maya.

**File**: `web/app/(setup)/onboarding/first-hire/page.tsx:15-16` (copy + example).

**Suggested fix**: Either parse the description on the gateway side into a charter + schedule (would require Genesis to be wired through), OR soften the copy to "Give them a name and a role" and let scheduling happen on the worker detail page.

---

### F10 [P1] — `/workers/{id}/overview` `WorkerOverviewContent` retries indefinitely on 400

This is the consequence of F1: because `fetchWorker` doesn't translate 400 into `WorkerNotFoundError`, the React-Query default retry policy (5 attempts, exponential backoff) hammers the gateway with the same 400 5 times before giving up. After the retries exhaust, the layout falls into `<ErrorState message="Couldn't load the worker" onRetry={...} />` — a fine error but only AFTER ~15s of spinner. The investor camera will read this as a hung app.

**File**: `web/lib/api/workers.ts:147-149` (only 404→WorkerNotFoundError, no 400 branch); `web/lib/hooks/use-worker.ts` (React Query default retry).

**Suggested fix**: Mirror F1 fix — translate 400 to `WorkerNotFoundError` so the user sees the friendly empty state immediately, and disable retries on 4xx in `useWorker`.

---

### F11 [P1] — Connect-provider "skip" copy understates the consequence

The provider step's skip warning reads: "Mock provider — workers run but responses are simulated. Skipping uses a mock provider until you add a real key in Settings · Providers." On a real (non-demo) install this is true. But in demo mode, the user is ALREADY using mock data for everything — the warning is a tautology, but it makes the wizard feel like the user is making a real choice. If the demo banner stays as-is (per F3), this whole branch reads as a fake choice the operator made about fake data — eroding trust in the rest of the demo.

**File**: `web/app/(setup)/onboarding/connect-provider/page.tsx:195-201`.

**Suggested fix**: When `isDemoMode()`, the skip note should explicitly say "Demo mode uses a mock provider regardless — you can skip OR enter any sk-ant- key." Currently any `sk-ant-` prefix validates without an upstream call (demo short-circuit in `use-provider-key.ts:40-42`), so the user has no signal that their key isn't actually being checked.

---

### F12 [P2] — Vertical card "Finance — Accounts Receivable" wraps onto 3 lines

The pick-vertical step renders cards in a 3-column grid. The "Finance — Accounts Receivable" card's name wraps onto 3 lines ("Finance —" / "Accounts" / "Receivable") because the line break treats the em-dash as a soft break. Visually awkward next to the single-line names ("GTM Engineer", "Customer Support"). The card text-balance / hyphenation isn't catastrophic but draws the eye.

**Reproduction**: Visit `/onboarding/pick-vertical` at desktop 1600px viewport; inspect the leftmost card.

**File**: `web/app/(setup)/onboarding/pick-vertical/page.tsx:298` (`<span className="ob-role-name">`); `OnboardingFormStyles.tsx` (.ob-role-name CSS).

**Suggested fix**: Shorten the seed name to "Finance · AR" or "Accounts Receivable" — matching the brand voice of "GTM Engineer" / "Sales SDR" (terse role labels).

---

### F13 [P2] — Mid-wizard navigation can leave the page scrolled to mid-form

Navigating from `/onboarding/workspace` back to `/onboarding/welcome` retains the scroll position from the workspace input field's `autoFocus`-triggered scrollIntoView, so the user lands mid-page on welcome (header partially out of view; the giant Audiowide hero half-hidden behind the sticky bar). The earlier screenshot captured during the audit showed this artifact. A `useEffect(() => { window.scrollTo(0, 0); }, [])` on each onboarding step would resolve it.

**File**: each step page in `web/app/(setup)/onboarding/*/page.tsx`.

**Suggested fix**: Add a one-line `useEffect` resetting scroll on mount in each step (or once in `OnboardingShell`).

---

### F14 [P2] — Demo banner takes a vertical slice from the wizard top bar

The shell stacks the demo banner (sticky-top, ~34px) ABOVE the wizard's own sticky top bar (`STEP 01 OF 05 ... SAVE & EXIT`). The wizard top bar's z-30 sticky is fine in isolation but the doubled-sticky shrinks the editorial hero into a smaller viewport slice. Less an issue for desktop investor demo (1920×1080) but mobile (375×812) loses ~70px of vertical real estate to two stacked headers.

**Reproduction**: Resize viewport to mobile (375×812); visit `/onboarding/welcome`; observe the welcome hero pushed below two stacked headers.

**File**: `web/components/skycot/DemoBanner.tsx` (sticky banner); `web/components/auth/WizardEditorialShell.tsx` (sticky top bar).

**Suggested fix**: Hide the demo banner on `/onboarding/*` routes — the welcome page's own "WELCOME TO SKYCOT" eyebrow already signals demo intent for the recording.

---

### F15 [P3] — "STEP TWO · WORKSPACE" / "STEP THREE · CONNECT AI PROVIDER" eyebrow inconsistency

Steps 1, 2, 3, 5 use word-spelled-out step numbers ("WELCOME TO SKYCOT", "STEP TWO · WORKSPACE", "STEP THREE · CONNECT AI PROVIDER", "STEP FIVE · FINAL"). Step 4 uses "STEP FOUR · PICK A ROLE". But the page-internal step strip uses numerals ("STEP 04 OF 05"). Two different conventions side-by-side. Minor consistency nit.

**File**: each step page's `eyebrow` prop in `web/app/(setup)/onboarding/*/page.tsx`.

**Suggested fix**: Pick one — either spell out all or use numerals throughout. The numerals are easier to scan; the spellings are more editorial — both legitimate. Drop one.

---

## Resolved since last audit

None — this is the first audit specifically scoped to the onboarding hire-flow against current main. The 2026-05-25 / 2026-05-26 audits were broader and were not used to seed this one.

---

## Orientation score: **52 / 100**

The wizard's visual design and copy are A-tier — F1 editorial-center looks like the locked design memory, Audiowide hero is rendering correctly, gradient is on-brand, the "Hire your first AI teammate" + "Onboard them like a human" frame is the strongest investor-narrative copy in the product. **But the function side is hollow**: the wizard literally provisions nothing, validates nothing, hires nothing, and the demo-mode end-state is an infinite spinner on a 400-rejected synthetic worker id. For the v1.0 demo MP4, F1 (loading-forever spinner), F2 (zero backend state), F3 (contradictory "sign in" banner), and F4 (no new hire on the roster) are camera-visible cracks that would each individually undermine the "hire is real" claim. Fix all four and this rates ≥85.

The "build it real, fake the dressing" rule (memory `feedback_no_theater_in_demo`) is being inverted here: the dressing (workspace name, KPI fixtures, sample personas) is fine, but the function (provision → install → hire → see them) is the theatrical part. Per the operator-locked POC scope (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`), the demo doesn't have to ship to production but it does have to be REAL when the camera drills in — and the wizard's "Hire them" CTA is exactly the moment the camera drills in.

---

## Suggested order of operations for fixes

1. **F1 + F2 together**: rewrite the demo first-hire path to call a real seed endpoint on the gateway that creates a tenant + installs the vertical + creates a worker row with a real UUID, then redirect to that worker. This single fix also resolves F4 and substantially reduces F10.
2. **F3**: change the demo banner copy from "Sign in to see your own workspace" to something that doesn't contradict the user having just signed up. ~5 minute fix.
3. **F5**: add `/verticals` to the Next rewrite OR move the gateway endpoint to `/api/v1/verticals/catalog`. Either is a few-line change. Then delete the dead 7-entry FALLBACK_VERTICALS.
4. **F6**: add Peta + Nori to the PERSONALITIES map (3 minutes) and either route demo-hires to a real sample worker, or wire a `GET /workers/{id}/personality` to the manifest.
5. **F7 + F8 + F9–F15**: polish round, after the function side is solid.
