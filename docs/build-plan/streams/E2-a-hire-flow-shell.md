# Stream E2-a — Hire flow: shell + routing + progress + draft persistence

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED.
**Authored against**: origin/main `dd19f54e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-a-hire-flow-shell`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 (hire flow) brief 1 of 9.

## v1 posture vs enterprise target

Per the enterprise-grade-defaults guidance, every brief states
v1 implementation explicitly and tags the gap to the strict
architecture:

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Draft persistence | localStorage (client-only) | Gateway-backed draft store with tenant + user scope | `mission-enterprise-hire-draft-server-store` |
| Draft expiry | 7-day client check on load | Server-enforced 7-day TTL on the draft record | same |
| Quota check | None (any user can hire) | Tenant-level worker-hire quota enforced at gateway | `mission-enterprise-tenant-quotas` |
| Auth | NextAuth bearer (from E0) | SSO/SAML/OIDC | `mission-enterprise-auth-sso` |
| Abandon recovery | Client refreshes session | Server-side recovery token + email link | `mission-enterprise-hire-resume-link` |

These shortcuts are acceptable for v1 because the *types and
interfaces* (route shape, session model, draft schema) match
the strict architecture — swapping localStorage for a gateway
draft store is a body change, not a contract change.

## Why

The hire flow
([wireframe-hire-flow.md](../../skycot/ux-research/wireframes/wireframe-hire-flow.md))
is the largest surface in v1 — 7 steps, all in the `(hire)`
route group from E0. This brief lands the skeleton:
all 7 routes exist, the HireShell (header bar + progress dots
+ escape button) renders, step-to-step navigation works, draft
state persists across reloads. **Step content is empty** —
each step renders a placeholder text. The 9 follow-on E2
briefs (E2-b through E2-i) fill in step content one at a time.

This is a contract-style brief for the hire flow specifically:
downstream E2-* briefs implement against the route shape and
state model this PR locks. The same brief-template marker
applies — locked decisions documented below.

## Scope

- 7 routes under `web/app/(hire)/`:
  - `/hire` — Step 1 landing (placeholder for E2-b).
  - `/hire/step/2` — Step 2 placeholder (E2-c).
  - `/hire/step/3` — Step 3 placeholder (E2-d).
  - `/hire/step/4` — Step 4 placeholder (E2-e).
  - `/hire/step/5` — Step 5 placeholder (E2-g).
  - `/hire/done/<workerId>` — Step 6 celebration banner
    (E2-h handles the celebration UI; this brief stubs
    a placeholder route).
  - Plus a NotFound page inside the hire group for any
    `/hire/step/<bad>` value.
- HireShell composite from E0 wired with:
  - Brand square top-left (clicks return to `/` with
    confirm if `unsavedHire = true`).
  - ProgressDots component showing N of 7 with current
    step highlighted.
  - Escape × button top-right (same as brand-square
    click).
  - Footer nav: Back + Continue buttons.
- Draft state shape + persistence:
  - `HireDraftState` interface: `{ step: 1..6, problem?:
    string, answers?: Record<string, unknown>, workerDraft?:
    WorkerDraft, ... }`.
  - `useHireDraft()` hook: localStorage-backed in v1; same
    interface that swaps to gateway-backed in v1.5.
  - 7-day expiry on load (timestamp check).
- Navigation logic:
  - Back: goes to previous step, preserves state.
  - Continue: disabled until current step's required
    fields are filled; advances to next step on click.
  - Closing the tab mid-flow leaves the draft; returning
    to `/hire` resumes at the last completed step.
  - Once `Step 6` reached, draft is cleared and the route
    becomes a no-op (redirects to `/`).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Shared hotspots: `web/app/(hire)/**` (locks this whole
  subtree until E2-a MERGES; E2-b–E2-i then take per-step
  locks on individual page files)

## Files you may touch

- `web/app/(hire)/layout.tsx` — REPLACE the placeholder
  from E0 with the real HireShell wiring. ≤ 120 LOC.
- `web/app/(hire)/hire/page.tsx` — REPLACE the E0
  placeholder with a Step 1 stub: `"Step 1 — placeholder
  for E2-b"`. Renders the HireShell footer (Back disabled
  on Step 1; Continue disabled because no input yet).
  ≤ 100 LOC.
- `web/app/(hire)/hire/step/[step]/page.tsx` — NEW
  dynamic route. Validates `step` is 1–6; renders a
  step placeholder; returns 404 for invalid step
  values. ≤ 150 LOC.
- `web/app/(hire)/hire/done/[workerId]/page.tsx` — NEW.
  Stub that renders the celebration banner placeholder
  and redirects to `/workers/<workerId>` after 800ms.
  ≤ 100 LOC.
- `web/components/skycot/HireShell.tsx` — REPLACE the
  E0 placeholder with the real component: brand,
  ProgressDots, escape button, draft-state-aware
  confirm-on-leave. ≤ 250 LOC.
- `web/components/ui/ProgressDots.tsx` — NEW. Reusable
  N-of-M dot indicator. Already named in E0's notes;
  this brief ships the implementation. ≤ 100 LOC.
- `web/lib/hire/draft-store.ts` — NEW. The localStorage
  wrapper for `HireDraftState`. Exports:
  `loadDraft()`, `saveDraft()`, `clearDraft()`,
  `hasActiveDraft()`. ≤ 150 LOC.
- `web/lib/hire/draft-types.ts` — NEW. Type defs.
  `HireDraftState`, `WorkerDraft`. ≤ 80 LOC.
- `web/lib/hire/draft-store.test.ts` — NEW. ≥ 8 tests:
  save/load round-trip, expiry on load, clearDraft
  removes the entry, partial state preserved across
  saves, malformed entry returns null, multi-tab
  conflict (last write wins), version migration stub,
  no-localStorage fallback. ≤ 250 LOC.
- `web/lib/hire/use-hire-draft.ts` — NEW. React hook
  wrapping the draft store + a state-context that
  components throughout the hire flow can read.
  ≤ 150 LOC.
- `web/lib/hire/use-hire-draft.test.tsx` — NEW. ≥ 4
  tests: hook hydrates from store, updates persist,
  clear resets state. ≤ 150 LOC.
- `web/components/skycot/HireShell.test.tsx` — NEW. ≥ 5
  tests: renders progress dots, Back navigates, escape
  confirms when draft active, escape no-confirms when
  no draft, brand square click respects same flow.
  ≤ 200 LOC.
- `web/tests/e2e/hire-flow-shell.spec.ts` — NEW. End-to-
  end: open /hire, advance through all 6 steps via
  Continue, see celebration redirect to /workers/stub.
  ≤ 120 LOC.

## Files you must NOT touch

- `web/app/(app)/**` — locked by E1 deliverables and
  user-installed page wiring.
- `web/components/skycot/ApprovalCard.tsx`,
  `ApprovalCard.test.tsx`,
  `ThemePicker.tsx`, `ThemeProvider.tsx`,
  `QueryProvider.tsx` — locked from Track C / E0.
- `web/lib/api/*` — locked by E0/E1-a; hire flow's
  draft store stays separate from the gateway client
  in v1.
- Other E2 sibling files (E2-b through E2-i won't exist
  yet, so this brief authors the placeholder pages).
- `crates/**` — backend draft-store brief is sibling, not
  in this PR.

## Acceptance criteria

- [ ] `/hire` route renders HireShell + Step 1
      placeholder text.
- [ ] `/hire/step/2` through `/hire/step/5` render
      Step 2–5 placeholders. `/hire/step/0` and
      `/hire/step/7` return 404 inside the hire shell.
- [ ] `/hire/done/<workerId>` renders the celebration
      placeholder and redirects to `/workers/<workerId>`
      after 800ms.
- [ ] HireShell renders:
      - Brand square top-left (lucide `Sparkles` icon
        for now; replaceable with logo in branding
        brief).
      - ProgressDots showing current step (filled
        accent), prior steps (filled accent at 40%
        opacity), upcoming (empty).
      - Tooltips on dot hover show step names.
      - Escape × button top-right.
- [ ] Back button:
      - Disabled on Step 1.
      - On Steps 2–5: routes to previous step.
- [ ] Continue button:
      - Disabled on placeholder steps (no input to
        validate yet).
      - When enabled (downstream briefs unlock it):
        advances to next step + persists draft.
- [ ] Draft persistence:
      - `localStorage.setItem("skycot.hireDraft", JSON.
        stringify({ step, ...state, ts: now() }))` on
        every state change.
      - On `/hire` mount: load draft. If `ts > 7 days
        old`, discard. If valid, route to the saved
        `step`.
      - Closing/reopening tab mid-flow resumes at
        last step.
- [ ] Escape-with-draft confirm:
      - If `unsavedHire = true` (i.e. user has typed
        anything in Step 1 or progressed past it),
        clicking brand or escape opens a confirm
        dialog: `"Discard your in-progress
        teammate?"` · `[ Keep working ]`
        `[ Discard and leave ]`.
      - "Discard and leave" clears draft + routes
        to `/`.
- [ ] On Step 6 (done) load: draft is cleared.
- [ ] 17 unit tests (8 draft store + 4 hook + 5 shell)
      pass.
- [ ] Playwright e2e passes.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 5KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-a-hire-flow-shell`
- PR title: `web(hire): shell + routing + draft persistence (E2-a)`

## Notes for the implementer

- **Route group recap.** `(hire)` from E0 is the
  layout-providing group. Adding files under
  `web/app/(hire)/hire/*` produces routes WITHOUT the
  `(hire)` segment in the URL — so `/hire`, `/hire/step/2`,
  etc.

- **ProgressDots API.**
  ```ts
  interface ProgressDotsProps {
    total: number;            // 7
    current: number;          // 1..6
    stepLabels?: string[];    // tooltips on hover
    onJumpBack?: (step: number) => void;
  }
  ```
  Clicking a dot LOWER than current jumps back (preserves
  state); upcoming dots are not clickable.

- **Draft state versioning.** Include a `version: 1` field
  in the stored object. On load, if version doesn't match,
  return null (forces a fresh start). This is the v1
  "migration" — proper migrations live in the gateway-
  backed draft store (sibling brief).

- **localStorage caveats.**
  - Private-mode browsers + storage-quota errors: catch
    and degrade gracefully. The draft store's `saveDraft`
    must not throw if storage is unavailable — log and
    no-op.
  - Multi-tab last-write-wins. v1 doesn't lock; v1.5
    server-backed handles it.
  - SSR-safe: all localStorage access is guarded by
    `typeof window !== "undefined"` and runs in
    useEffect.

- **Architectural seams (so v1.5 hardening is a swap).**
  - `loadDraft()` / `saveDraft()` / `clearDraft()` are
    async-typed even though v1 impl is sync. v1.5 swaps
    to gateway calls without changing the signatures.
  - The `useHireDraft()` hook returns
    `{ state, update, clear, isLoading, error }` — same
    shape as a typical TanStack Query hook so the
    v1.5 swap is mechanical.
  - Authentication context: hire flow reads
    `useSession()` from E0; v1 trusts the bearer; v1.5
    SSO swap inherits.
  - Quota check: stub function `checkHireQuota()`
    returns `{ allowed: true, remaining: Infinity }` in
    v1. v1.5 hits the gateway.

- **The placeholder copy** for each step:
  - Step 1: `"Step 1 — placeholder for E2-b (Land)"`.
  - Step 2: `"Step 2 — placeholder for E2-c (Listen)"`.
  - Step 3: `"Step 3 — placeholder for E2-d (Drafting transition)"`.
  - Step 4: `"Step 4 — placeholder for E2-e (Introduce)"`.
  - Step 5: `"Step 5 — placeholder for E2-g (Connect)"`.
  - Step 6: `"Step 6 — celebration placeholder for E2-h"`.

  When a downstream brief lands, it replaces the
  placeholder content but keeps the route file in place.

- **Don't add Genesis-lite calls yet.** This brief has
  no LLM dependency. The Step 3 "Drafting transition"
  is a static UI placeholder; the real Genesis-lite call
  arrives in E2-d.

- **Confirm dialog.** Use Radix AlertDialog (from
  shadcn). If `AlertDialog.tsx` isn't installed yet, copy
  it as part of this brief.

## Open questions (architectural, flag at next checkpoint)

- The draft state survives across browser sessions on
  the same device. Should it also survive across
  devices? v1.5 server-backed brief solves this; v1
  is device-only.
- 7-day expiry — accept, or shorten to 48h to reduce
  stale-draft risk?
- Should "Discard and leave" require typing the
  worker's would-be name to confirm? For v1, no.

## Hardening briefs implied (Track H — Enterprise hardening)

These are explicitly tagged for the post-v1 hardening
backlog. NOT blocking this PR.

- `mission-enterprise-hire-draft-server-store` —
  gateway-backed draft store with tenant + user scope,
  enforced 7-day TTL, cross-device sync.
- `mission-enterprise-tenant-quotas` — gateway-side
  enforcement of per-tenant worker-hire quotas.
- `mission-enterprise-hire-resume-link` — server-issued
  recovery token + email link for resuming an abandoned
  hire flow.
