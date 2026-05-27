# fix-cohorts-empty-cohort-drawer-focus-trap-leak-renders-unresponsive

**Owner:** claude
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-cohorts-empty-cohort-drawer-focus-trap-leak-renders-unresponsive`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — After dismissing an empty-cohort drawer/modal on `/cohorts`, subsequent eval/click operations against the page timed out for ~30s. Likely a focus-trap leak or stuck animation consuming the renderer thread.**

## Root cause

> After dismissing the "Developing Support" empty-cohort modal via the close button, subsequent eval/click operations against the page timed out for ~30s — looked like a focus-trap leak or a stuck animation/transition consuming the renderer thread. Manifested in the preview tool, but the underlying issue is likely a real bug.
> — `whole-site-mvp-claims-coverage.md` F16

Likely culprits:
1. A radix-ui `<DialogPortal>` or `<DialogContent>` not being unmounted on close — leaving an invisible focus-trap layer over the page.
2. A `framer-motion` `<AnimatePresence>` orphan that never resolves its exit animation.
3. A `useEffect` cleanup function missing on the drawer component.

The audit observed it via the Claude Preview MCP eval timeout — real-user impact would be hover/keyboard inputs landing on an invisible layer that captures them silently, or the close button appearing to "stick".

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F16 [P2] — escape-to-close on the cohort drawer leaves the page in an unresponsive state for ~30s

## Fix recipe

1. **Locate the cohort drawer component** — likely under `web/components/skycot/cohorts/` or `web/app/(app)/cohorts/`. Grep for `DialogPortal`, `Drawer`, `useDialog`, `useDisclosure`.
2. **Add a React-DevTools Profiler trace** in development:
   - Open `/cohorts` with the React Profiler open.
   - Click a cohort card with 0 members ("Developing Support").
   - Close the drawer with the close button (and again with Escape).
   - Inspect the "Commit" entries — any with >50ms? Any orphaned components?
3. **Check for missing cleanup**:
   - In the drawer's `useEffect`, ensure event listeners (keydown for Escape) are unsubscribed on close.
   - Verify radix Dialog `onOpenChange` correctly flips the parent state.
   - If using framer-motion `<AnimatePresence>`, ensure `mode="wait"` or proper exit-animation completion.
4. **Add a Playwright regression spec** `web/playwright/cohorts/drawer-close-no-hang.spec.ts`:
   - Visit `/cohorts`.
   - Click an empty cohort.
   - Close the drawer.
   - Immediately click another card.
   - Assert the second card click responds within 500ms.
5. **If the cohort drawer is a stub** (per `fix-cohorts-empty-state-render-locked-d3-magazine-preview`), the fix may be to skip rendering the drawer altogether for empty cohorts — just show an inline "no members yet" badge on the card.

## Acceptance criteria

- [ ] After closing the empty-cohort drawer, subsequent clicks on the cohorts page respond within 500ms.
- [ ] No orphaned focus-trap layer remains in the DOM after close.
- [ ] React Profiler shows no stuck-animation entries > 100ms.
- [ ] Playwright spec passes cold (3 runs).
- [ ] PR body cites finding F16 with the profiler trace evidence.

## May-touch

- `web/components/skycot/cohorts/*` (drawer / dialog / panel components)
- `web/app/(app)/cohorts/page.tsx`
- `web/playwright/cohorts/drawer-close-no-hang.spec.ts` (NEW)

## Out of scope

- Empty-state design for cohort cards (covered by `fix-cohorts-empty-state-render-locked-d3-magazine-preview`)
- Cohorts tab count / registry drift (covered by `fix-cohorts-dept-registry-design-lock-drift-6-vs-8`)
- Other dialog / drawer surfaces (audit them separately if this fix surfaces a pattern)

## Effort

M — Diagnostic-led; ~50-200 LOC + 1 spec. ~3-6 hours depending on root cause depth.
