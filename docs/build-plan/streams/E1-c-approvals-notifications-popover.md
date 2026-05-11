# Stream E1-c — Approvals inbox: notifications popover (the bell)

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED.
**Authored against**: origin/main `6ce200f3` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-c-approvals-notifications-popover`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1.

## Why

The approvals-inbox header has a non-functional bell icon
(see [wireframe-approvals-inbox.md §"(4)"](../../skycot/ux-research/wireframes/wireframe-approvals-inbox.md)).
This brief makes it real: a popover that surfaces *system-level
notifications* about the workspace — distinct from per-action
approval requests (which are the cards themselves).

System notifications include:
- A worker's daily budget is approaching its cap.
- A skill version was promoted and is now in use.
- The skill-regression-detector flagged behavioral drift.
- An OAuth connector needs re-auth.
- A scheduled duty fired and produced an output worth surfacing.
- The trust-ratchet suggests autonomy promotion for a worker.

These don't fit on a cards-list because they're not card-shaped
(no "Approve / Refuse" binary). They're informational + sometimes
actionable, with their own decision shapes.

## Scope

- A `NotificationsPopover` component, opened by the bell
  button in the page header.
- Three sections inside: **Today**, **Yesterday**, **Earlier**.
- Per-notification row: icon + title + timestamp + dismiss
  button. Optional inline CTA (e.g. "Re-auth Gmail" for an
  OAuth-renewal notification).
- Unread badge: small dot on the bell icon when any
  notification is unread.
- Mark-all-read action.
- Persistent unread state via `useNotifications()` hook,
  cache + mutation against the gateway. Mock fallback like
  the approvals path (404 → empty list, no errors).
- A real-time "1 new" pulse on the bell when a new
  notification arrives mid-session (via TanStack Query
  refetch-on-focus + 60s polling). Live WebSocket is a
  later concern.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/page.tsx` (the bell wiring)

## Files you may touch

- `web/components/skycot/NotificationsPopover.tsx` — NEW.
  Popover content + per-section render + action handlers.
  ≤ 350 LOC.
- `web/components/skycot/NotificationRow.tsx` — NEW. Single
  notification row component. ≤ 150 LOC.
- `web/lib/api/notifications.ts` — NEW. `fetchNotifications`,
  `markAsRead`, `markAllAsRead`, `dismissNotification`. Mock
  fallback on 404. ≤ 200 LOC.
- `web/lib/hooks/use-notifications.ts` — NEW. TanStack Query
  hooks: `useNotifications()` (60s polling),
  `useMarkAsRead()`, `useDismiss()`. ≤ 150 LOC.
- `web/lib/sample-notifications.ts` — NEW. 8-10 sample
  notifications spanning all the categories listed in
  "Why". ≤ 150 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: wire the bell
  button to open the popover, render the unread-dot badge.
  ≤ 350 LOC after changes.
- `web/components/skycot/NotificationsPopover.test.tsx` —
  NEW. ≥ 6 unit tests: empty state, rendered list,
  mark-read flip, dismiss removes row, mark-all-read
  clears unread count, inline CTA action triggers
  callback. ≤ 200 LOC.

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — locked.
- `web/lib/api/approvals.ts` — locked.
- `web/lib/sample-approvals.ts` — locked.
- Other E1 sibling files (E1-a, E1-b output) — independent
  surfaces, no shared mutation paths.

## Acceptance criteria

- [ ] Bell button in the header opens
      `NotificationsPopover` anchored below it.
- [ ] Popover shows three sections (Today / Yesterday /
      Earlier) with notifications grouped by `created_at`.
- [ ] Each row shows: an icon (per notification kind),
      title text, relative timestamp ("3m ago"),
      [Dismiss] icon button, and optional inline CTA.
- [ ] Unread notifications have a subtle accent treatment
      (background tint or leading colored bar). Read
      notifications are muted.
- [ ] Bell shows a dot indicator when any notification is
      unread; clicking the popover and seeing them counts
      as "read" but doesn't dismiss.
- [ ] "Mark all read" link in the popover header clears
      the unread dot.
- [ ] Dismiss removes the row from the list (and posts
      to gateway).
- [ ] Inline CTA (e.g. "Re-auth Gmail") fires the callback
      AND auto-dismisses the notification.
- [ ] Empty state: `"Nothing new."` · `"You'll see system
      notifications here — budget alerts, OAuth renewals,
      drift warnings."`.
- [ ] Mock fallback when gateway 404s: returns
      `SAMPLE_NOTIFICATIONS`.
- [ ] 60s polling for new notifications when popover is
      closed and tab is focused.
- [ ] 6 unit tests pass.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-c-approvals-notifications-popover`
- PR title: `web(approvals): notifications popover with unread tracking (E1-c)`

## Notes for the implementer

- **Notification kind enum.** Define in
  `web/lib/sample-notifications.ts`:
  ```ts
  type NotificationKind =
    | "budget-approaching"
    | "budget-exceeded"
    | "skill-promoted"
    | "skill-drift-detected"
    | "oauth-needs-reauth"
    | "duty-output-ready"
    | "autonomy-promotion-suggested"
    | "incident";
  ```
  Each kind maps to an icon (lucide) + a default tone.

- **Icon mapping** (single source in
  `NotificationRow.tsx`):
  ```
  budget-approaching      → DollarSign      (warning tone)
  budget-exceeded         → AlertTriangle   (destructive)
  skill-promoted          → Sparkles        (success)
  skill-drift-detected    → TrendingDown    (warning)
  oauth-needs-reauth      → KeyRound        (warning)
  duty-output-ready       → FileText        (info)
  autonomy-promotion-...  → Zap             (info)
  incident                → AlertOctagon    (destructive)
  ```

- **Inline CTA convention.** A notification can optionally
  carry a `cta: { label: string, href?: string,
  action?: "dismiss" | "callback" }`. The row renders the
  CTA as a small ghost button on the right. `href` routes
  to that URL; `action: "callback"` fires a kind-specific
  handler.

- **Popover positioning.** Anchor below the bell, align-end
  so the right edge of the popover lines up with the bell's
  right edge. Width: 380px desktop, full-width on mobile
  (with the popover becoming a bottom-sheet on `< sm` per
  shadcn's responsive Popover pattern).

- **Read vs dismissed semantics.**
  - Opening the popover marks all currently-visible
    unread items as `read` (the unread dot on the bell
    disappears) but they remain in the list.
  - Dismiss removes the row from the list permanently
    (audit-logged).
  - "Mark all read" is the same as opening; redundant on
    desktop but useful on mobile where the popover may
    only show a subset.

- **Polling cadence.** 60 seconds when the tab is focused.
  Disable polling when the tab is backgrounded (TanStack
  Query handles this via `refetchInterval` + `refetch
  IntervalInBackground: false`).

- **What this brief does NOT do.**
  - Doesn't add WebSocket real-time push. That's a v1.5
    optimization (also relates to E1-f for approval-card
    live updates).
  - Doesn't surface notifications as OS-level pushes
    (browser-API-level — push API). Out of v1.
  - Doesn't filter or search notifications. The list is
    short by design; if it grows, surface a "see all"
    link to a dedicated `/notifications` route in v1.5.
  - Doesn't add notification preferences (which kinds to
    show). Future enhancement, surfaced in the operator
    Settings surface.

- **Empty state copy is deliberately welcoming**, not
  fatalistic. "Nothing new." beats "0 notifications." in
  every UX test ever conducted.
