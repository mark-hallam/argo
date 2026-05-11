# Stream E4-f — Worker profile: Settings tab (channels / quiet hours / budget / autonomy / danger zone)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a MERGED.
**Authored against**: origin/main `87edb8b0` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-f-worker-profile-settings-tab`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 6 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Budget ceiling | Per-worker $X/day + $Y/month with soft warning at 80% + hard pause at 100% | Hierarchical: tenant cap → department cap → team cap → worker cap; tighter ceiling wins | `mission-enterprise-budget-hierarchical-caps` |
| Autonomy level | 3-state radio (Conservative / Balanced / Aggressive) per worker | Per-action-type granularity: e.g. for AP Specialist, "duplicate detection: autonomous; new-vendor invoices: always ask"; matrix UI for operators | `mission-enterprise-per-action-autonomy-policy` |
| Channel preferences | Multi-select with primary; same as hire-flow Step 5 | Per-action-type channel routing (e.g. financial-decisions go to Controller's Slack, all other to delegator's iMessage) | `mission-enterprise-channel-routing-policy` (already named) |
| Quiet hours | Single range per worker, tz-aware | Per-channel-kind quiet hours (Slack can ping after-hours; SMS strictly quiet); operator-overridable for incident-level escalations | `mission-enterprise-per-channel-quiet-hours` |
| Danger zone — Delete | Operator-only with type-name-to-confirm; v1 ships the gate but mock fallback in dev | Same; production hard-enforced via gateway ACL | `mission-enterprise-worker-delete-acl` |
| Advanced — View role manifest | Renders TOML as read-only (delegator) or editable (operator) | Operator edit creates a new manifest version + audit; restore-from-history; rollback | `mission-enterprise-role-manifest-versioning` |

v1 is acceptable: Settings is the **operator-heavy**
surface. Delegators only edit channels + quiet hours +
their own budget. Operator-only sections (Advanced,
full Danger zone) are gated by `permissions.canViewSettings`
+ `canDelete`.

## Why

E4-a's placeholder for `/workers/<id>/settings` reads
"Settings — coming in E4-f". This brief lights it up
with the 5 sections from the wireframe: Channels,
Quiet hours, Budget, Autonomy level, Advanced, Danger
zone.

For enterprise: settings are where IT / compliance /
operations meet the delegator's workflow. v1 ships
solid defaults; many fields gated to operator-only via
`permissions`.

## Scope

- Route `/workers/<id>/settings` renders the tab.
- 6 sections (per wireframe), each with its own
  edit affordances:
  - **Channels** — toggle list + primary radio.
  - **Quiet hours** — single time range editor +
    timezone.
  - **Budget** — daily + monthly caps + current
    usage bar.
  - **Autonomy level** — 3-state radio (Conservative
    / Balanced / Aggressive).
  - **Advanced** (operator-only) — view role
    manifest, trajectory replay sandbox, worker ID
    + full provenance, audit-the-audit log.
  - **Danger zone** — Pause / Archive / Delete
    (latter operator-only with type-to-confirm).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/workers/[workerId]/settings/page.tsx`
    (NEW).

## Files you may touch

- `web/app/(app)/workers/[workerId]/settings/page.tsx`
  — NEW. ≤ 350 LOC.
- `web/components/skycot/worker/settings/ChannelsSection.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/worker/settings/QuietHoursSection.tsx`
  — NEW. ≤ 180 LOC.
- `web/components/skycot/worker/settings/BudgetSection.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/settings/AutonomySection.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/worker/settings/AdvancedSection.tsx`
  — NEW (operator-gated). ≤ 250 LOC.
- `web/components/skycot/worker/settings/DangerZone.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/settings/TypeNameToConfirm.tsx`
  — NEW. Reusable type-the-name-to-confirm dialog
  used for Delete. ≤ 180 LOC.
- `web/lib/api/worker-settings.ts` — NEW. All
  settings CRUD: `getSettings(workerId)`,
  `updateChannels()`, `updateQuietHours()`,
  `updateBudget()`, `updateAutonomy()`,
  `archiveWorker()`, `deleteWorker()`. ≤ 350 LOC.
- `web/lib/hooks/use-worker-settings.ts` — NEW. ≤
  200 LOC.
- `web/components/skycot/worker/Settings.test.tsx` —
  NEW. ≥ 10 tests. ≤ 400 LOC.

## Files you must NOT touch

- All E1/E2/E3 / earlier-E4 files — read-only.
- `crates/**` — sibling backend briefs for the
  endpoints.

## Acceptance criteria

- [ ] Route `/workers/<id>/settings` renders the 6
      sections in order.
- [ ] **Channels section:**
      - Multi-select toggle list of 7 channel kinds
        (Slack, iMessage, SMS, Email, Push, WhatsApp,
        Voice).
      - One designated primary (radio).
      - Save → `updateChannels` API.
- [ ] **Quiet hours section:**
      - Single time range (start / end) + timezone
        dropdown (auto-detected from workspace).
      - Save → `updateQuietHours` API.
      - Strings: `"Mariah won't be reachable between
        ${start} and ${end} (${tz}). Outside this
        window she can ping you on your primary
        channel."`
- [ ] **Budget section:**
      - Daily cap editor + monthly cap editor.
      - Current usage bar (filled vs cap).
      - Warning state at 80% used (orange tint).
      - Hard-pause state at 100% used (worker paused
        until cap raised OR next day, whichever
        first).
      - Save → `updateBudget` API.
      - Strings: `"Daily cap: $${daily}. ${used}
        used today (${pct}%). Monthly cap: $${monthly}.
        ${monthlyUsed} used this month."`
- [ ] **Autonomy section:**
      - Radio with 3 options:
        - `( ) Conservative — ask often`
        - `(•) Balanced — current`
        - `( ) Aggressive — minimal asking`
      - Save → `updateAutonomy` API.
      - Resets the trust-ratchet (per the journey-
        retrain doc).
- [ ] **Advanced section (operator-only via
      `permissions.canViewSettings`):**
      - `[ View role manifest (TOML) ]` — opens a
        read-only modal showing the worker's TOML.
        Operator-only: shows an `[Edit]` button that
        opens an editable form (with `[Save as new
        version]` + version-history list).
      - `[ Trajectory replay sandbox ]` — opens a
        modal pointing to E5 trajectory replay
        (placeholder for v1).
      - `[ Worker ID + provenance ]` — chip showing
        worker_id + tenant_id + namespace_id +
        version hash; copy-on-click each.
      - `[ Audit-the-audit log ]` — link to filtered
        audit timeline showing every settings change
        for this worker.
- [ ] **Danger zone:**
      - `[ ⏸ Pause Mariah ]` — confirm → mutation.
        Available to anyone with `canPauseResume`.
      - `[ 🗄 Archive Mariah ]` — confirm with
        explicit consequences → mutation (soft-
        delete; recoverable for 30 days). Available
        to anyone with `canArchive`.
      - `[ 🗑 Delete Mariah ]` — operator-only via
        `permissions.canDelete`. Opens
        TypeNameToConfirm dialog: user types the
        worker's name to enable the Delete button.
        Audited; irreversible.
- [ ] All mutations have mock fallback in dev;
      production hard-fails on 404.
- [ ] 10 unit tests pass.
- [ ] Playwright e2e: visit settings, toggle a
      channel, save; change autonomy level, save;
      view danger zone, type name, confirm delete
      (mock).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 10KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-f-worker-profile-settings-tab`
- PR title: `web(workers): Settings tab — channels / quiet / budget / autonomy / danger (E4-f)`

## Notes for the implementer

- **Section ordering matters.** Most-frequent
  settings first (channels, quiet hours), then
  policy-shape (budget, autonomy), then power-user
  (advanced), then destructive (danger zone). The
  visual rhythm steers users away from
  destructive actions.

- **Budget usage bar.** Use a thin progress bar
  primitive (~6px tall):
  ```
  [████████░░░░░░░░░░░░] $4.32 / $5.00 today
  ```
  Color: muted < 50%, default 50-80%, warning
  80-95%, destructive ≥ 95%.

- **Autonomy reset behavior.** Switching from
  Balanced → Conservative explicitly resets the
  trust-ratchet for all action types. Surface this
  in the radio's tooltip: `"Conservative resets
  the trust ratchet — Mariah asks again for any
  action she's been doing autonomously."`

- **Danger zone visual.** Section header in
  destructive tone (red text, faint red top border).
  Buttons each have an icon + destructive variant
  (Delete is destructive-filled; Pause + Archive
  are destructive-outline).

- **TypeNameToConfirm dialog** — generic component
  that takes the expected name string + the action
  callback:
  ```tsx
  <TypeNameToConfirm
    title="Delete Mariah permanently?"
    bodyLines={[
      "This cannot be undone.",
      "Audit logs remain but Mariah's memory and learned skills are erased.",
    ]}
    expectedName="Mariah"
    cta="Delete"
    onConfirm={deleteWorker}
  />
  ```
  The Delete button stays disabled until the typed
  text matches `expectedName` exactly (case-
  sensitive).

- **Advanced section visibility.** Default
  `permissions.canViewSettings = false` for
  delegators, `true` for operators. In v1, sample
  data sets it true for all (so dev/test can
  exercise the full surface).

- **Archive confirm copy:**
  ```
  Title: "Archive Mariah?"
  Body: "Her memory and skills are preserved.
         You can recover her within 30 days."
  CTAs: [ Cancel ]  [ Archive ]
  ```

- **Delete confirm copy:**
  ```
  Title: "Delete Mariah permanently?"
  Body: "This cannot be undone. Audit logs remain
         but Mariah's memory and learned skills
         are erased.
         Type the worker's name to confirm:"
  Input:    [____________________]
  CTAs:     [ Cancel ]  [ Delete ]
  ```

- **Audit-the-audit log link.** Routes to
  `/audit?worker=<id>&action_type=settings_change`.
  Audit timeline (E5) handles the filter.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-budget-hierarchical-caps` —
  tenant → department → team → worker cap
  hierarchy.
- `mission-enterprise-per-action-autonomy-policy`
  — matrix UI for per-action autonomy.
- `mission-enterprise-per-channel-quiet-hours` —
  per-channel-kind quiet hours.
- `mission-enterprise-worker-delete-acl` — server-
  enforced delete ACL.
- `mission-enterprise-role-manifest-versioning` —
  manifest version history + rollback.

Sibling backend brief:

- `mission-gateway-worker-settings-endpoints` —
  all settings CRUD.
