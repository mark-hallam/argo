# Stream E2-h — Hire flow: Step 6 (Celebration banner on approvals inbox)

**Owner**: claude.
**Depends on**: E0 MERGED; E2-a..g MERGED.
**Authored against**: origin/main `c564e039` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-h-hire-flow-celebration-banner`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 8 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Banner persistence | localStorage flag `skycot.celebrate.<workerId>` set on done-page mount; banner shown on next approvals-inbox load; dismissed via × | Server-side post-hire state machine ("celebrating", "active", "inert"); banner driven by server state, not client storage | `mission-enterprise-hire-post-state` |
| Banner content (inert worker) | Generic "tools needed" copy with link to connect later | Per-worker required-connector list inferred from charter; CTA per missing connector | `mission-enterprise-hire-inert-checklist` |
| First-action surfacing | When worker produces first approval, banner auto-dismisses (the approval card itself is the "first opinion") | Same; plus push-notification on user's primary channel | `mission-enterprise-hire-first-action-notify` |
| Day-1 wrap | Separate E2-i brief — backend duty | Same | `mission-enterprise-hire-day1-server-duty` |

v1 is acceptable because the celebration banner is purely
a UX affordance, not a security surface. The localStorage
flag is per-device + per-browser — if the user logs in from
a different device, they don't see the banner, but they're
not missing safety-critical info.

## Why

After hire (E2-e), the user routes through Step 5 (E2-g)
and lands at `/hire/done/<workerId>`. Per wireframe
([wireframe-hire-flow.md §"Step 6 — First action (handoff)"](../../skycot/ux-research/wireframes/wireframe-hire-flow.md)),
this isn't a destination page — it's a brief celebration
moment that redirects to `/` (approvals inbox) with a banner.

The banner is the *psychological completion* of the hire
flow: visual confirmation the worker exists, link to their
profile, hint that they'll start working now.

## Scope

- `/hire/done/<workerId>` route (already a stub from E2-a)
  ships its real implementation:
  - Sets `localStorage.skycot.celebrate.<workerId> = {
    workerName, inert?, hiredAt, ttl }` (TTL: 24 hours).
  - Clears the hire draft from localStorage.
  - Shows a 1.5s celebration overlay
    ("🎉 ${workerName} is hired.") before redirecting
    to `/`.
- New `CelebrationBanner` component on the approvals
  inbox:
  - On approvals-inbox mount, checks for celebration
    flags within TTL.
  - Renders a dismissible banner per recently-hired
    worker (max 1 visible at a time; if multiple,
    queue by hire order).
  - Two banner variants:
    - **Active worker** (default): `"🎉 ${workerName}
      is hired. She'll start working in the background.
      The first time she has an opinion, you'll see it
      here."` + `[ Open ${name}'s profile → ]` CTA.
    - **Inert worker** (`?inert=1` from Step 5 skip):
      `"${workerName} is hired but needs tools to start
      working. [ Connect tools → ]"` + dismiss.
  - Banner dismisses:
    - On `×` click → flag removed from localStorage.
    - On first approval card from this worker arriving
      → flag auto-removed (the card itself replaces
      the banner as the "first opinion" signal).
    - On TTL expiry (24h) → flag removed.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(hire)/hire/done/[workerId]/page.tsx` — locked
    by this brief.
  - `web/app/(app)/page.tsx` — narrowly extended for
    banner mount.

## Files you may touch

- `web/app/(hire)/hire/done/[workerId]/page.tsx` —
  REPLACE the E2-a stub with the real done-handler:
  set the celebration flag + 1.5s overlay + redirect.
  ≤ 150 LOC.
- `web/components/skycot/hire/CelebrationOverlay.tsx`
  — NEW. The 1.5s mid-screen overlay before redirect.
  ≤ 120 LOC.
- `web/components/skycot/CelebrationBanner.tsx` — NEW.
  The dismissible banner on the approvals inbox.
  ≤ 200 LOC.
- `web/lib/hire/celebration-storage.ts` — NEW. Storage
  wrapper: `setCelebration(workerId, data)`,
  `getCelebrations(): CelebrationEntry[]`,
  `clearCelebration(workerId)`, `pruneExpired()`. ≤ 150
  LOC.
- `web/lib/hire/celebration-storage.test.ts` — NEW.
  ≥ 6 tests: set/get round-trip, TTL expiry, prune,
  multi-worker queue, malformed-entry safe, no-
  localStorage fallback. ≤ 200 LOC.
- `web/lib/hire/use-celebrations.ts` — NEW. Hook that
  reads celebrations + filters against approvals cache
  (auto-clears celebration when first approval arrives
  from that worker). ≤ 150 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: mount
  `<CelebrationBanner />` above the page header.
  ≤ 420 LOC after.
- `web/components/skycot/CelebrationBanner.test.tsx`
  — NEW. ≥ 5 tests: renders active variant, renders
  inert variant, dismiss removes flag, auto-clears on
  first approval, TTL expiry. ≤ 200 LOC.

## Files you must NOT touch

- All E2-a..g files — locked.
- `web/components/skycot/ApprovalCard.tsx` — locked.
- Any non-hire surface beyond the narrow page.tsx
  extension.
- `crates/**` — backend post-hire state machine
  is a sibling Track-H brief.

## Acceptance criteria

- [ ] `/hire/done/<workerId>`:
      - Reads `?inert=1` from query string.
      - Reads `workerName` from draft state (or query
        param fallback `?name=`).
      - Sets celebration flag in localStorage.
      - Renders CelebrationOverlay (centered, ~1.5s).
      - Clears the hire draft.
      - Redirects to `/` after 1.5s.
- [ ] CelebrationOverlay copy: `"🎉 ${workerName} is
      hired."` + tagline `"Heading to your inbox..."`.
- [ ] CelebrationBanner on approvals inbox:
      - Reads celebrations from localStorage on mount.
      - Renders only the most recent unconsumed
        celebration (max 1 banner visible).
      - Active variant copy: `"🎉 ${workerName} is
        hired. She'll start working in the background.
        The first time she has an opinion, you'll see
        it here."` + `[ Open ${workerName}'s profile
        → ]` link to `/workers/${workerId}`.
      - Inert variant copy: `"${workerName} is hired
        but needs tools to start working. [ Connect
        tools → ]"`. Connect link routes to
        `/workers/${workerId}?tab=settings`.
- [ ] Banner dismissal:
      - `×` click removes the flag.
      - First approval from this worker arriving
        (detected via cache scan) auto-removes.
      - TTL > 24h removes on next mount.
- [ ] Banner styled distinct from approval cards:
      - Subtle accent background (`var(--primary)` at
        8% alpha).
      - Border-left accent stripe (3px wide).
      - Compact height (≤ 56px desktop).
- [ ] Multiple recent hires (rare but possible):
      celebrations queue by hire timestamp; user sees
      one at a time; dismissing reveals the next.
- [ ] 11 unit tests pass (6 storage + 5 banner).
- [ ] Playwright e2e: complete full hire flow,
      assert `/` shows the banner, click ×, assert
      banner gone.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 3KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-h-hire-flow-celebration-banner`
- PR title: `web(hire): Step 6 celebration banner on approvals inbox (E2-h)`

## Notes for the implementer

- **Celebration storage shape.**
  ```ts
  interface CelebrationEntry {
    workerId: string;
    workerName: string;
    hiredAt: string;  // ISO
    inert: boolean;
    ttl: string;      // ISO of expiry
  }
  ```
  Stored at `localStorage.skycot.celebrations` as a
  serialized array. Single key, multiple entries
  (avoids per-key prefix scanning).

- **TTL of 24h.** After 24h the celebration is stale —
  if the user hasn't been back in a day, the moment
  has passed; surfacing a "you hired Mariah!" banner
  later feels weird. Drop silently.

- **Auto-clear on first approval.** Hook scans the
  current `useApprovals()` data for any approval
  with `worker.id === celebration.workerId`. If
  found, calls `clearCelebration(workerId)`.

- **First-render vs hydration.** The banner reads
  localStorage on mount, which means SSR has no
  data. Render the banner only after first client
  render (`useEffect`) to avoid hydration mismatch.
  This is OK — banner is a celebratory bonus, not
  content the user demands instantly.

- **Routing from inert banner.** "Connect tools" CTA
  routes to `/workers/<id>?tab=settings`. The worker
  profile page (E4) handles the `?tab=` query param.

- **No banner on the worker profile page itself.**
  Banner is approvals-inbox-only. If a user navigates
  straight to `/workers/<id>` after hire, they see
  the worker profile, not the banner. The
  approvals-inbox is the canonical "home" for the
  delegator.

- **Banner copy is locked** in the brief — don't let
  copy drift during implementation. If the operator
  asks for different copy later, that's a separate
  PR.

- **Don't make the celebration overlay any longer
  than 1.5s.** Anything more feels like a delay,
  not a celebration. 1.5s is enough for the
  emoji + name + tagline to register without
  becoming annoying.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-hire-post-state` — server-
  side post-hire state machine.
- `mission-enterprise-hire-inert-checklist` — per-
  worker required-connector list.
- `mission-enterprise-hire-first-action-notify` —
  push notification on first action.
