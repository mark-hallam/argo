# Stream E3-e — Team page: first-hire empty state + onboarding nudge

**Owner**: claude.
**Depends on**: E3-a MERGED; E2-b MERGED (reuses `EXAMPLE_ROLES`).
**Authored against**: origin/main `e623ba3d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E3-e-team-empty-state-onboarding`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E3 brief 5 of 5 (LAST in E3).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Onboarding personalization | Generic empty state showing all 5 example roles | Workspace-tailored: operator pre-selects ≤ 3 example roles for their org; new users see the curated set | `mission-enterprise-onboarding-curated-examples` |
| First-hire telemetry | None | Per-tenant funnel telemetry (saw empty state → clicked example → started hire → completed) for activation rate visibility | `mission-enterprise-onboarding-funnel-telemetry` |
| Cross-surface coherence | Approvals-inbox empty state already links to /hire (from E1-baseline) | All zero-state surfaces (approvals, team, audit) carry the same onboarding nudge — single source of truth, render-only divergence | `mission-enterprise-onboarding-shared-shell` |
| Sample teammate showcase | Static text descriptions | Live demo workers in a sandboxed tenant the user can preview before hiring | `mission-enterprise-onboarding-sandbox-demo` |

v1 is acceptable: this brief produces a welcoming +
discoverable empty state with zero security surface
area. It doesn't gate hiring; the basic empty state from
E3-a already worked. This brief makes it inviting.

## Why

E3-a shipped a minimal empty state (`"No teammates
yet."` + 1 CTA). That's adequate but flat. A user
arriving at `/team` for the first time benefits from
seeing what their teammates *could* be — concrete
examples that bridge "I have an abstract problem" to
"I see what a teammate looks like".

This brief upgrades the empty state into an
exploration surface: the 5 example role tiles from
E2-b rendered as inviting cards, each tappable to
start the hire flow with the role's description
pre-filled.

## Scope

- Replace E3-a's basic empty-state component on the
  Team page with `TeamEmptyState`.
- New surface contains:
  - Heading: `"Hire your first AI teammate."`
  - Sub-line: `"Pick a starting point — you can
    adjust everything in the next minute."`
  - 5 example tiles (one per `EXAMPLE_ROLES` from
    E2-b). Each tile shows: title + 1-sentence
    description + small "Start with this →" CTA.
  - Tapping a tile routes to `/hire` with
    `?example=<id>` query param.
  - Secondary "Or describe your own" link below the
    grid → routes to `/hire` with no param.
- Update `/hire` (E2-b's Step 1) to read
  `?example=<id>` and pre-fill the textarea with the
  example's description.
- Optional surface tweak: a small "What can SKYCOT
  teammates do?" disclosure below the tiles that
  expands to a short feature list (replies in your
  voice, approvals via Slack/SMS/email, learns from
  your corrections, etc.).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/team/page.tsx` (empty-state mount)
  - `web/app/(hire)/hire/page.tsx` (E2-b's Step 1 —
    narrow extension to read `?example=<id>`)

## Files you may touch

- `web/components/skycot/TeamEmptyState.tsx` — NEW.
  The rich empty-state component. ≤ 250 LOC.
- `web/components/skycot/ExampleRoleTile.tsx` — NEW.
  One tile in the grid; reusable for any "example
  role" surface. ≤ 150 LOC.
- `web/app/(app)/team/page.tsx` — narrowly scoped:
  swap `<EmptyState>` for `<TeamEmptyState>` when
  zero teammates. ≤ 450 LOC after.
- `web/app/(hire)/hire/page.tsx` — narrowly scoped:
  read `?example=<id>` from `useSearchParams()` on
  mount and pre-fill the textarea via `lib/hire/
  examples.ts` lookup. ≤ 380 LOC after.
- `web/components/skycot/TeamEmptyState.test.tsx` —
  NEW. ≥ 5 tests: renders 5 tiles, tile click routes
  with example id, disclosure toggle, fallback link
  route. ≤ 200 LOC.

## Files you must NOT touch

- `web/lib/hire/examples.ts` — locked from E2-b. This
  brief CONSUMES the `EXAMPLE_ROLES` constant; does
  not modify it.
- `web/components/skycot/hire/Step1Land.tsx` — locked
  from E2-b. The `?example=` pre-fill happens at the
  page level before passing the value to Step1Land
  as initial draft state.
- `web/components/skycot/TeammateGrid.tsx`,
  `TeammateCard.tsx`, `TeammateCardContextMenu.tsx`
  — locked from E3-a/b.
- All E1/E2 files except the narrow E2-b extension.
- `crates/**` — no backend changes.

## Acceptance criteria

- [ ] `/team` with zero teammates renders
      `<TeamEmptyState>` (not the basic empty from
      E3-a).
- [ ] Heading: `"Hire your first AI teammate."` (no
      period in v1.5 if branding shifts; lock now).
- [ ] Sub-line: `"Pick a starting point — you can
      adjust everything in the next minute."`.
- [ ] 5 example tiles in a responsive grid (1 col
      mobile, 2 col tablet, 3 col desktop with
      wrap).
- [ ] Each tile renders:
      - Title (e.g. `"Marketplace seller"`).
      - Description (1 sentence, truncated to 2
        lines max via line-clamp).
      - "Start with this →" CTA (ghost button at
        the bottom-right).
- [ ] Tapping a tile (anywhere on the body) or its
      CTA routes to `/hire?example=<id>`.
- [ ] `/hire?example=marketplace-seller` (etc.)
      pre-fills the Step 1 textarea with the
      example's description string.
- [ ] User can edit the pre-filled text before
      submitting.
- [ ] Below the tiles: `[ Or describe your own → ]`
      ghost link → routes to `/hire` with no
      `?example=` param.
- [ ] Optional disclosure (`<details>` element):
      `"What can SKYCOT teammates do?"` summary.
      Open shows 5-bullet feature list.
- [ ] Empty state respects dark mode + tenant theme
      (uses CSS vars, not hardcoded colors).
- [ ] 5 unit tests pass.
- [ ] Playwright e2e: zero-team workspace renders
      empty state → click tile → see /hire with
      pre-fill → submit advances to Step 2.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 4KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E3-e-team-empty-state-onboarding`
- PR title: `web(team): first-hire empty state with example tiles (E3-e)`

## Notes for the implementer

- **Example tile copy** lives in
  `lib/hire/examples.ts` (from E2-b). Each entry has
  `{ id, title, description }`. Reuse it directly —
  do NOT duplicate the strings here.

- **Example id mapping.** The `?example=<id>` param
  matches the `id` field on each `EXAMPLE_ROLES`
  entry. If `id` doesn't exist on the type yet, add
  it as a narrowly-scoped extension to E2-b's
  examples.ts:
  ```ts
  export const EXAMPLE_ROLES = [
    { id: "marketplace-seller", title: ..., description: ... },
    { id: "recruiter", ... },
    ...
  ];
  ```
  (E2-b's brief didn't include the `id` field. If the
  implementer of E2-b shipped without it, this brief
  adds it.)

- **5-bullet feature list** for the disclosure:
  ```
  • Replies that sound like you — voice profile
    inferred from how you write.
  • Approvals via Slack, iMessage, SMS, email — your
    pick.
  • Learns from your corrections without you editing
    a config file.
  • Audit trail for every action — exportable for
    compliance.
  • Pause or adjust her anytime — she's your teammate,
    not your tool.
  ```
  Each bullet is one sentence. Locked copy; don't
  drift.

- **Tile interaction.** The whole tile is clickable
  (cursor pointer + hover state). The bottom-right
  CTA is a focusable affordance for keyboard +
  screen-reader users; the tile body is also
  clickable. Both navigate to the same destination.

- **Pre-fill text validation.** When the user lands
  on `/hire?example=...`, the pre-filled text is ≥
  15 chars (the validation threshold from E2-b), so
  Continue is immediately enabled. Don't lock the
  user out of editing — they can still adjust.

- **Don't add a "skip to a blank flow" CTA above the
  tiles.** Putting the "or describe your own" *below*
  the tiles (not above) nudges users toward the
  example path, which is the higher-success route
  per the journey doc. The free-text path stays
  accessible at the bottom — discoverable, not
  promoted.

- **Tenant-theming validation.** Test the empty
  state with the Acme tenant theme to confirm tile
  accents flip correctly from indigo to orange (the
  test already exists in your dev preview).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-onboarding-curated-examples` —
  workspace-tailored example set selected by
  operator.
- `mission-enterprise-onboarding-funnel-telemetry` —
  per-tenant activation funnel measurement.
- `mission-enterprise-onboarding-shared-shell` —
  unified empty-state pattern across approvals /
  team / audit.
- `mission-enterprise-onboarding-sandbox-demo` —
  live preview worker in a sandboxed tenant.
