# Stream E3-b — Team page: TeammateCard composite

**Owner**: claude.
**Depends on**: E0 MERGED; E3-a MERGED.
**Authored against**: origin/main `da089c9a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E3-b-team-teammate-card`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E3 brief 2 of 5.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Card data freshness | TanStack Query stale-time 30s; auto-refetch on focus | Live WS subscription to worker status changes; sub-second freshness | `mission-enterprise-team-status-ws` |
| Status display | Inferred from `Worker.status` enum returned by gateway | Computed server-side from action ledger + duty schedule + connector health; client never derives | already enforced — client just renders |
| Headline bullets | Server returns precomputed; client renders | Same; v1 ships the API contract | n/a |
| Avatar | Initials fallback | Operator/worker-controlled avatar (initials / generated / uploaded) per E2-e hardening | `mission-enterprise-persona-avatar-options` (named in E2-e) |
| Pause/resume actions | Client-side optimistic update + gateway mutation | Server-side ACL: only delegators-of-this-worker may pause; operator can pause any in tenant | `mission-enterprise-worker-pause-acl` |

v1 is acceptable: the card is read-mostly + 2 mutations
(pause/resume) which respect the existing approval-action
audit infrastructure.

## Why

E3-a renders skeleton placeholders in the grid; this
brief lights up the real composite. Per the wireframe
([wireframe-team-page.md §"TeammateCard composition"](../../skycot/ux-research/wireframes/wireframe-team-page.md)),
each card shows avatar + name + status + role + today
bullets + open-profile CTA, with a context menu for
pause/resume.

## Scope

- New `TeammateCard.tsx` composite. Replaces the
  skeleton render in E3-a's grid.
- Renders, top to bottom:
  - Avatar (lg) + name (font-semibold)
  - Status indicator (dot + label, semantic color)
  - Role tagline (one line, muted)
  - "Today" block: 2-3 bullets from
    `worker.headline.bullets`
  - "Tomorrow" preview line (if `worker.headline.tomorrow`)
  - Divider
  - `[ Open profile → ]` ghost button (full-width at
    bottom)
- Card-level context menu (right-click desktop, long-
  press mobile, `⋮` button top-right of card):
  - Pause Mariah (if Working / Waiting / Idle)
  - Resume Mariah (if Paused)
- Click anywhere on the card body → routes to
  `/workers/<workerId>`.
- Hover state: subtle shadow + slight lift (-2px Y).
- `useWorkerActions(workerId)` hook with
  `pauseWorker`, `resumeWorker` mutations.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/TeammateCard.tsx`
  + the team page's grid renderer.

## Files you may touch

- `web/components/skycot/TeammateCard.tsx` — NEW.
  Composite component. ≤ 280 LOC.
- `web/components/skycot/TeammateCardContextMenu.tsx`
  — NEW. Right-click / long-press / ⋮ menu with
  pause/resume entries. ≤ 200 LOC.
- `web/app/(app)/team/page.tsx` — narrowly scoped:
  swap `TeammateCardSkeleton` for `TeammateCard`
  in the grid render. ≤ 350 LOC after.
- `web/lib/api/workers.ts` — narrowly scoped: extend
  with `pauseWorker(workerId)` + `resumeWorker(workerId)`
  mutations. ≤ 250 LOC after (was 180 from E2-e).
- `web/lib/hooks/use-worker-actions.ts` — NEW.
  TanStack Query mutations + optimistic update of
  the team cache. ≤ 180 LOC.
- `web/components/skycot/TeammateCard.test.tsx` — NEW.
  ≥ 7 tests: renders all states (working/waiting/
  idle/blocked/paused), context menu opens, pause
  fires mutation + optimistic update, resume fires,
  click routes, keyboard-accessible. ≤ 300 LOC.

## Files you must NOT touch

- All E1/E2/E3-a files except as narrowly extended.
- `web/components/skycot/ApprovalCard.tsx` — locked.
- `crates/**` — pause/resume endpoints sibling brief.

## Acceptance criteria

- [ ] TeammateCard renders all parts per wireframe:
      - Avatar (lg, initials from worker.name)
      - Name (font-semibold)
      - Status dot + label
      - Role tagline
      - Today bullets (2-3)
      - Optional Tomorrow preview line
      - Divider
      - "Open profile →" ghost button (full-width)
- [ ] Status colors (from existing Avatar status enum):
      - Working: var(--info)
      - Waiting: var(--warning)
      - Idle: muted
      - Blocked: var(--destructive)
      - Paused: muted + "Paused" label
- [ ] Hover state: shadow-md + transform -2px Y
      (transition 150ms).
- [ ] Click anywhere on card (except ⋮ + Open button)
      routes to `/workers/<workerId>`.
- [ ] ⋮ button top-right opens context menu.
- [ ] Right-click desktop + long-press mobile also
      open menu.
- [ ] Menu items:
      - Pause: visible when status ∈ {working,
        waiting, idle}.
      - Resume: visible when status = paused.
      - Both: confirm dialog before firing.
- [ ] Pause:
      - Optimistic update: card flips to Paused state
        immediately.
      - POST `/api/v1/workers/<id>/pause` (mock
        fallback in dev returns success).
      - On error: rollback + toast.
- [ ] Resume: symmetric.
- [ ] Confirm copy:
      - Pause: Title `"Pause Mariah?"` · Body `"She'll
        stop taking new actions until you resume her.
        Current pending approvals stay where they are."`
        · CTAs `[ Cancel ]` `[ Pause ]`.
      - Resume: Title `"Resume Mariah?"` · Body `"She'll
        pick up where she left off."` · CTAs `[ Cancel
        ]` `[ Resume ]`.
- [ ] Toast after action: `"Mariah paused. Resume
      from her profile."` / `"Mariah resumed."`.
- [ ] Keyboard accessibility:
      - Tab focuses card; Enter opens profile.
      - Shift+F10 / context-menu-key opens menu.
      - Escape closes menu.
- [ ] 7 unit tests pass.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 5KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E3-b-team-teammate-card`
- PR title: `web(team): TeammateCard composite with pause/resume (E3-b)`

## Notes for the implementer

- **Composition reuses existing primitives** — Card,
  Avatar, StatusPill, Button. The card is a thin
  composition over these; the only "new" structural
  thing is the context menu + the pause/resume hook.

- **Status pill ↔ avatar status ring** are different
  indicators of the same data. The Card uses the
  StatusPill primitive for the inline label; the
  Avatar's status ring (small dot at bottom-right
  per Track-C ring change) reinforces visually.
  Don't double-render — pick one. Per the wireframe:
  StatusPill for the label, Avatar ring stays simple.

- **Today block.** Render `worker.headline.bullets`
  as a small bulleted list (≤ 3 items):
  ```
  Today:
  • 8 replies sent (2 waiting your OK)
  • $340 in pending offers
  ```
  Each bullet is plain text from the server. Don't
  bold or color-code unless the server signals (it
  doesn't yet in v1).

- **Tomorrow line.** One-line preview:
  ```
  Tomorrow: morning routine
  ↳ inbox summary at 06:30
  ```
  Render only if `worker.headline.tomorrow` is set;
  otherwise omit the section entirely.

- **Context menu.** Use shadcn's `DropdownMenu`
  primitive (Radix). Trigger by:
  - Right-click anywhere on the card (preventDefault
    + open menu at cursor).
  - Long-press on touch devices (500ms hold).
  - ⋮ button top-right of card.
  All three open the same menu.

- **Pause/resume optimistic update.** Use
  TanStack Query's `setQueryData` to flip the
  worker's status in the `["team"]` cache
  immediately. On error, restore the cached snapshot
  + show toast.

- **Audit considerations.** Pause/resume actions
  are existing approval-action types (not new). They
  go through the same audit log as any other action.
  No new audit wiring in this brief.

- **Don't add archive or delete here.** Those are
  destructive actions; they live on the worker
  profile's Settings tab (E4-f), not on the team
  card. Keep the card's menu to non-destructive
  reversible actions only.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-team-status-ws` — live WS
  subscription for status changes.
- `mission-enterprise-worker-pause-acl` — server-
  side ACL for pause/resume permission.

Sibling backend briefs:

- `mission-gateway-worker-pause-endpoint` — POST
  `/api/v1/workers/<id>/pause` + `/resume`.
