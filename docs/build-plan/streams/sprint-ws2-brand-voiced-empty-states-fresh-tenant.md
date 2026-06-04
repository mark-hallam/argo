# sprint-ws2-brand-voiced-empty-states-fresh-tenant

**Owner**: claude or codex. **Frontend lane — parallelizable with WS1 backend work.**
**Depends on**: none to BUILD (empty states render for any zero-data tenant, including today). Fully *exercised* once WS1-1 lands and fresh tenants are real.
**Authored against**: origin/main `35d9225d2` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws2-brand-voiced-empty-states-fresh-tenant`.

## Why

Day-0 gap-audit (WS2): once a stranger gets their own fresh, empty workspace
(WS1), the read pages must not render a blank screen or a raw "No activity yet".
A fresh tenant has zero workers, skills, verticals, workflows, audit rows, and
approvals. Today only the seeded demo tenant has data, so these empty states are
either missing or read as broken. For a paying-ready product the **first-run
experience** (the moment right after signup) is the most important screen we
have — it must say "here's how to get started", on-brand, not "nothing here".

## What

Add a real, brand-consistent empty state to each fresh-tenant read surface, each
with a single clear next action that routes the user forward (usually → hire /
Genesis / onboarding). Reuse the **D3 brand recipe** (Audiowide eyebrow,
purple→PINK `#8B5CF6 → #EC4899` gradient, Inter body, warm radial-blob bg) — see
`docs/skycot/brand/DESIGN_SYSTEM.md` (LOCKED — do NOT run a fresh optometrist
exploration; consume the locked tokens) and the existing empty-state precedents
already in the repo (`rg 'empty' web/components/skycot/` to find the established
pattern + any shared `<EmptyState>` component — extend it, don't fork it).

Surfaces (each gets a tailored headline + 1 primary CTA):
- `/team` and `/workers` → "Your team starts with one hire" → CTA to Genesis/first-hire.
- `/skills` → "No skills yet — your teammates earn and share them as they work" → CTA to the marketplace.
- `/workflows` → "No workflows yet" → CTA to the template gallery.
- `/audit` → "Every action your team takes will be logged here, tamper-evident" → (no CTA needed; reassuring copy — this is the moat surface).
- `/approvals` → "No approvals waiting — sensitive actions will queue here for your call" → (reassuring; no CTA).

**Truth-bounded copy rule (NO THEATER):** the empty state describes what WILL
happen, framed as future ("will be logged", "your teammates earn them") — it must
NOT imply data exists or fabricate counts. Do not show fake "0 of 100" style
metrics unless the real usage endpoint returns them.

## May-touch
- A shared `web/components/skycot/EmptyState.tsx` (extend if it exists; create
  if not — re-verify with `rg` first).
- The 6 page/components above (`web/app/(app)/{team,workers,skills,workflows,audit,approvals}/` + their content components). Render the empty state when the live fetch returns zero rows (not when it's still loading — keep the existing skeleton/loading branch intact).
- vitest specs pinning the empty branch renders + the CTA href.
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

Do NOT touch the data-fetching gates (that's WS3) or the backend. This is purely
the zero-rows render branch + copy.

## Acceptance criteria
- [ ] Each of the 6 surfaces renders the brand-consistent empty state when its
      live fetch returns an empty list (NOT during loading).
- [ ] Copy is truth-bounded (future-framed, no fabricated data/counts).
- [ ] Uses the LOCKED brand tokens (purple→PINK gradient, Audiowide eyebrow,
      Inter body) — not the deprecated purple→cyan.
- [ ] CTAs route correctly (team/workers → hire; skills → marketplace; workflows
      → templates). vitest pins each href.
- [ ] `pnpm typecheck` clean; `pnpm lint` clean; `pnpm vitest run` GREEN on the
      touched specs (report counts).
- [ ] No regression to the existing loading/skeleton or populated branches
      (the demo tenant still renders its roster).

## Out of scope
- The fresh-tenant baseline SEED (installing default skills/verticals) → **WS2-1** (backend).
- Flipping the prod no-bypass fetchers → **WS3-3**.
- Any new backend endpoint.

## Parallel-surface check
- `rg 'EmptyState|empty-state|No activity yet' web/components web/app` — find the
  established pattern; extend the canonical one, do not introduce a parallel
  component family.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS2). Brand:
`docs/skycot/brand/DESIGN_SYSTEM.md` (LOCKED).
