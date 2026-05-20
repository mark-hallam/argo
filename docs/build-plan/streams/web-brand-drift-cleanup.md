# web-brand-drift-cleanup — fix settings/members CTA + Jamal accent + /approvals register

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-brand-drift-cleanup`.
**Priority**: P3 — wave-2 brand-consistency audit found 3 cleanup items + 1 operator-decision item.
**Sizing**: ~80-150 LOC (3 surgical edits + 1 deferred-to-operator note + tests).

## Why this brief exists

From the wave-2 brand-consistency audit:

> Top 5 drift fixes (3 still outstanding after wave-2):
> 4. `web/app/(app)/settings/members/page.tsx:179,428` — gradient CTAs in brand-restrained register (drift)
> 5. `web/app/landing/page.tsx:868` — Jamal's name accent uses `#0891b2` (cyan)
>
> Register conflict to flag: operator memory locks `/approvals` as
> Inter-only enterprise-restrained, but the implementation uses
> `<BrandLoudHero>` gradient banner. Either the memory is stale or
> impl drifted — needs operator decision.

## Goals

1. /settings/members CTAs: swap gradient → restrained (Inter, slate, no gradient).
2. /landing Jamal accent: swap `#0891b2` (cyan) → `#8b5cf6` (purple) to match canonical gradient stops.
3. /approvals register conflict: surface to operator (this brief flags it; doesn't fix without operator decision).

## Steps

1. /settings/members:179,428: swap CTA className from gradient-loud to brand-restrained primary.
2. /landing:868: replace the `#0891b2` color token with `#8b5cf6` (the purple-side stop).
3. /approvals: leave as-is BUT add a comment flagging the conflict + open a `docs/skycot/operator/decisions/approvals-register.md` file documenting the conflict for the operator to call.

## may-touch

- `web/app/(app)/settings/members/page.tsx`
- `web/app/landing/page.tsx`
- New: `docs/skycot/operator/decisions/approvals-register.md`

## must-NOT-touch

- BrandLoudHero (used correctly on other surfaces)
- Other operator-locked designs
- /approvals page rendering (operator decision pending)

## Acceptance criteria

- [ ] /settings/members CTAs render as brand-restrained (Inter, no gradient).
- [ ] /landing Jamal name no longer references `#0891b2`.
- [ ] /approvals brand-register-conflict doc exists for operator to action.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Visual sanity: settings/members + /landing still feel coherent.

## References

- Audit: `docs/skycot/audit/brand/2026-05-20-wave-2-brand-consistency.md`
- Memory: `project_approvals_page_design_locked.md` (locked Inter-only)
