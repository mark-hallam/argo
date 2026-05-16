# claude-spa-motion-design-system — Framer-Motion variants per brand tokens (Wave 7 P2)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED (#1744) + `claude-design-system-component-extension` MERGED (#1745).
**Authored against**: origin/main `111a6559` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-spa-motion-design-system`.
**Source**: Wave 7 Jony-Ive polish arc — brand identity locked, components exist, but they're static. Coherent motion vocabulary is what separates "good components" from "feels alive".

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — motion is brand impression; static UIs feel dead.
- **Repeatability** (#3) — single source of truth for motion timings, easings, springs. Every component opts into the same vocabulary, no bespoke per-component animations.

## Scope

P2. Build a Framer-Motion vocabulary file + apply it to 8 primary interaction surfaces.

### Vocabulary file: `web/lib/motion/variants.ts`

```typescript
// Brand-locked timings (from web/app/globals.css --motion-* tokens)
export const motion = {
  // Standard variants
  fadeUp:    { initial: {opacity:0, y:8}, animate: {opacity:1, y:0}, transition: {duration: 0.18, ease: [0.4,0,0.2,1]} },
  fadeIn:    { initial: {opacity:0},      animate: {opacity:1},      transition: {duration: 0.15, ease: 'easeOut'} },
  scaleIn:   { initial: {opacity:0, scale:0.96}, animate: {opacity:1, scale:1}, transition: {duration: 0.2, ease: [0.4,0,0.2,1]} },
  // List / collection
  staggerChildren: { animate: {transition: {staggerChildren: 0.04}} },
  // Success affordance
  successPulse: { animate: {scale: [1, 1.03, 1], transition: {duration: 0.4, ease: 'easeOut'}} },
  // Spring-based for cards/modals
  springModal: { type: 'spring', stiffness: 340, damping: 32 },
} as const;

export const reducedMotionFallbacks = { /* same shape, all durations → 0 */ };
```

Plus a `<MotionRoot>` provider that detects `prefers-reduced-motion` and swaps to the fallback variants automatically.

### 8 surfaces to apply motion to

| # | Surface | Variant |
|---|---|---|
| 1 | Page route transitions | `fadeUp` on the App-Router segment boundary |
| 2 | `ApprovalCard` enter | `scaleIn` |
| 3 | `ApprovalCard` resolve (approve/deny) | `successPulse` + fade-out |
| 4 | Worker list rendering | `staggerChildren` + `fadeUp` per row |
| 5 | Audit timeline row append (WS push) | `fadeUp` from the top |
| 6 | Modal/Sheet/Dialog open | `springModal` |
| 7 | Skeleton → content swap | crossfade |
| 8 | Toast appear/disappear | `fadeUp` from bottom-right |

**Discipline:**
- No animation longer than 400ms. Snappy by default.
- No bouncy springs > 1.05 scale. Subtle, not cartoony.
- `prefers-reduced-motion: reduce` honored everywhere — fallback durations all 0.
- Motion is opt-in: components don't auto-animate; they wrap children in motion primitives explicitly.

LOC: ~400.

## Files you may touch

- `web/lib/motion/variants.ts` (NEW) — the vocabulary. ≤120 LOC.
- `web/lib/motion/provider.tsx` (NEW) — `<MotionRoot>` + reduced-motion detection. ≤60 LOC.
- `web/lib/motion/test.ts` (NEW) — vitest unit tests for variant shape + reduced-motion swap. ≤80 LOC.
- `web/components/ui/ApprovalCard.tsx` — wrap enter/resolve in motion. ≤30 modified LOC.
- `web/components/ui/AuditTimeline.tsx` — stagger rows; new-row append fade. ≤30 modified LOC.
- `web/components/ui/WorkerCard.tsx` (list wrapper) — stagger. ≤20 modified LOC.
- `web/components/ui/Dialog.tsx` / `Sheet.tsx` — `springModal`. ≤25 modified LOC each.
- `web/app/(app)/layout.tsx` — mount `<MotionRoot>`. ≤8 modified LOC.
- `web/app/page.tsx` (or relevant route layouts) — fadeUp on segment transitions. ≤15 modified LOC.

## Files you must NOT touch

- Brand tokens themselves (`web/app/globals.css` `--motion-*`) — consume only.
- Marketing site (`marketing/`) — pure landing pages with their own motion philosophy (less per-PR; out of scope here).
- Non-display components (data hooks, route handlers, API clients) — pure UI brief.

## Acceptance criteria

- [ ] `web/lib/motion/variants.ts` exports the documented shape.
- [ ] `prefers-reduced-motion: reduce` → all animations complete in 0ms (test with media-query overrides in vitest).
- [ ] 8 surfaces wired (PR body lists each with screenshot or video reference).
- [ ] No animation exceeds 400ms.
- [ ] No spring scale exceeds 1.05.
- [ ] Bundle delta ≤ 8 KB gzipped (Framer-Motion tree-shakeable; only used variants ship).
- [ ] `pnpm test` passes (motion variant tests + existing component snapshots — snapshots may regenerate with motion props; review diff is non-behavioral).
- [ ] `pnpm typecheck && pnpm lint` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-spa-motion-design-system`
- Title: `feat(web): Framer-Motion vocabulary + apply to 8 primary surfaces (claude-spa-motion-design-system)`
- Worker verification: pnpm checks + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Motion-on-scroll** — out of scope. That's a separate concept (`useInView` triggers); file follow-up.
- **Tree-shake awareness** — import from `framer-motion` only the variants used; don't `import *`.
- **`AnimatePresence` for unmount animations** — wrap conditional renders (modal close, toast dismiss); without it, components vanish instantly.
- **Layout animations off by default** — `layout` prop is expensive; only use when explicitly needed (e.g. ApprovalCard resize on data load).
- **No global keyframe CSS.** All motion goes through Framer-Motion primitives so the variant file is the single source of truth.
- **Operator preview**: brief author runs `pnpm dev` and records a 30-second video clip of all 8 surfaces in action; embed link in PR body. Reviewer can eyeball without checking out.
