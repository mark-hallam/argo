# fix-retro-page-ssr-loading-no-empty-state

**Owner**: claude or codex.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-retro-page-ssr-loading-no-empty-state`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/whole-site-mvp-claims-coverage.md` F18 [P2].

🟡 **P2 — `/retro` SSR renders the literal string "Loading retros…" with no skeleton, no empty-state component, no progress affordance. If the query resolves to zero retros (likely on a fresh tenant), the loading text just sits there forever — the page never converts to an empty-state or a "no retros yet" empty card. Out-of-runbook path so demo is unaffected, but a buyer probing the sidebar lands on a half-built page.**

## Root cause

`web/app/(app)/retro/page.tsx:142` (or thereabouts — `grep -n "Loading retros" web/app/(app)/retro/page.tsx` confirms):

```tsx
{retrosQuery.isPending ? (
  <p>Loading retros…</p>
) : retros.length === 0 ? (
  <p>Loading retros…</p>   // ← bug: same string in both branches
) : (
  <RetroList items={retros} />
)}
```

Or similar — the page emits the loading string even when the query has resolved with zero rows. There's no empty-state component for `/retro`, and the loading text doesn't transition to anything meaningful.

This compounds two issues:

1. **No skeleton during pending** — a literal `<p>Loading retros…</p>` is the loading affordance. No animated skeleton; not consistent with the rest of the SPA where pending states get a `<Skeleton>` shimmer (e.g. `/team`, `/workflows`).
2. **No empty state on zero rows** — when the query resolves to `[]`, the user sees the same "Loading retros…" string indefinitely, which is incorrect and unhelpful.

The page is out-of-runbook (not in the 6-scene recording) so this is a polish miss, not a demo gate. But the sidebar exposes `/retro` (per the cohorts memory it's part of the Project & Ops dept tab), and a buyer clicking it lands on a perpetually-loading page.

## Findings this brief closes

- whole-site-mvp-claims-coverage F18 [P2] — `/retro` SSR renders "Loading retros…" with no fallback empty-state

## Fix recipe

1. **Add a real empty-state component** for `/retro`:
   ```tsx
   <RetroEmptyState
     headline="No retros yet."
     hint="Retros land here once a workflow completes a cycle."
     illustration={RetroIllustration}
   />
   ```
   Pattern matches `CohortsEmptyPreview`, `TeamEmptyState`, etc.
2. **Add a real skeleton** for pending state:
   ```tsx
   {retrosQuery.isPending ? (
     <RetroListSkeleton />
   ) : retros.length === 0 ? (
     <RetroEmptyState />
   ) : (
     <RetroList items={retros} />
   )}
   ```
3. **Don't show empty-state during SSR** — gate on `retrosQuery.isSuccess` (same fix shape as cohorts SSR flash, covered by `fix-cohorts-ssr-empty-preview-flash-before-hydrate`):
   ```tsx
   {!retrosQuery.isSuccess ? <RetroListSkeleton /> :
    retros.length === 0 ? <RetroEmptyState /> :
    <RetroList items={retros} />}
   ```
4. **Vitest pin**: simulate pending, resolved-empty, and resolved-with-data → assert correct branch renders.
5. **Optional** — drop `/retro` from the sidebar if it's not v1 scope. Memory check needed: is `/retro` part of the v1.0 demo MP4 surface? If not, leaving it as an orphan-but-tidy page is fine.

## Acceptance criteria

- [ ] `/retro` does NOT show "Loading retros…" indefinitely.
- [ ] When the query is pending, a `<RetroListSkeleton>` renders.
- [ ] When the query resolves to zero rows, a `<RetroEmptyState>` renders with helpful copy.
- [ ] When the query resolves with rows, the list renders.
- [ ] Vitest pins each branch.

## May-touch

- `web/app/(app)/retro/page.tsx:142` (the ternary + states)
- `web/components/skycot/retro/RetroEmptyState.tsx` (NEW)
- `web/components/skycot/retro/RetroListSkeleton.tsx` (NEW)
- Vitest in `web/app/(app)/retro/__tests__/`

## Out of scope

- `/retro` page out of v1 nav (separate operator decision).
- Default-title leak on `/retro` (covered by `fix-spa-page-titles-metadata-leak-default-approvals`).
- Other orphan pages (covered by `fix-spa-polish-help-retro-billing-orphan-pages`).
- Retro feature itself (out of v1 scope).

## Effort

~80-150 LOC: new empty-state + skeleton + page wiring + tests. ~1.5 hours. P2 because out-of-runbook polish but a real sidebar trap when a buyer probes.
