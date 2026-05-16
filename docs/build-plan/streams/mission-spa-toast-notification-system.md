# mission-spa-toast-notification-system — global toast/notification primitives (Wave 7 P2)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED (#1744) + `claude-spa-motion-design-system` (parallel in queue; if not yet merged, ship motion-less and wire up post-merge).
**Authored against**: origin/main `111a6559` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-spa-toast-notification-system`.
**Source**: Wave 7 Jony-Ive polish — today's success/error UX is `alert()` or inline divs with random styling. Unified toast system replaces ad-hoc notifications across the SPA.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — coherent feedback after every action.
- **Reliability** (#1) — error toasts surface what failed and what to try.

## Scope

P2. Toast primitive (via Sonner — minimal, accessible, opinionated default for shadcn-ecosystem) + a typed `useToast()` hook + ~10 callsite migrations.

### API

```typescript
import { useToast } from '@/lib/ui/use-toast';

const { success, error, info, warning, loading, promise } = useToast();

success("Worker hired", { description: "Maya is now active in your team." });
error("Couldn't hire worker", { description: "Provider returned 503. Retrying in 30s..." });
info("Eval running", { description: "12 fixtures to grade." });
warning("Quota near limit", { description: "You've used 85% of this month's actions." });
loading("Hiring Maya...");
promise(api.hireWorker(input), {
  loading: "Hiring...",
  success: (worker) => `${worker.name} is ready.`,
  error: (err) => `Couldn't hire: ${err.message}`,
});
```

### Discipline

- **Positioning**: bottom-right on desktop, top-center on mobile (Sonner default).
- **Auto-dismiss**: success/info 4s, warning 6s, error 8s; loading stays until updated.
- **Stacking**: max 3 visible; older toasts collapse to "+ N more".
- **Brand colors**: success uses `--success` (existing green), error `--destructive`, warning `--warning`, info `--brand-primary`.
- **Iconography**: Lucide icons (CheckCircle / XCircle / Info / AlertTriangle). Consistent line weight.
- **Motion**: if `claude-spa-motion-design-system` MERGED, use `fadeUp` from bottom-right; else Sonner default.
- **Action button optional**: `error()` can carry an `action: { label: 'Retry', onClick: () => ... }`.

### Callsites to migrate

1. `/auth/signup` form submit → success/error
2. `/hire` worker-create flow → promise() during hire
3. `/workers/:id/skills/new` install-skill → success/error
4. `/audit/action/:id` approve/deny button (operator-side) → success/error
5. `/team` invite-member form → success
6. Provider-key form (`/onboarding/connect-provider`) → success/error
7. Stripe portal-redirect failures → error
8. Any place currently using `alert()` or `window.confirm` (grep + replace)
9. ESLint rule: `no-alert` enforced workspace-wide (post-migration)

LOC: ~300.

## Files you may touch

- `web/package.json` — narrowly: add `sonner` dep. ≤3 modified LOC.
- `web/components/ui/sonner.tsx` (NEW) — shadcn-style Sonner wrapper. ≤60 LOC.
- `web/lib/ui/use-toast.ts` (NEW) — typed API hook. ≤80 LOC.
- `web/lib/ui/use-toast.test.ts` (NEW). ≤60 LOC.
- `web/app/(app)/layout.tsx` — mount `<Toaster/>`. ≤6 modified LOC.
- 8-10 callsite migrations (worker enumerates in PR body). ≤200 modified LOC total.
- `.eslintrc` / `eslint.config.mjs` — add `no-alert` rule. ≤5 modified LOC.

## Files you must NOT touch

- Brand tokens — consume only.
- Marketing site (`marketing/`) — toasts are SPA-only (marketing pages are mostly static).
- API/data layer — toasts are pure presentation.

## Acceptance criteria

- [ ] `useToast()` exposes `success | error | info | warning | loading | promise`.
- [ ] `promise()` correctly transitions loading → success/error based on Promise resolution.
- [ ] All toast variants render with brand-aligned colors + Lucide icons.
- [ ] Max 3 stacked toasts; 4th collapses old ones.
- [ ] `prefers-reduced-motion: reduce` → toasts appear/disappear without motion.
- [ ] ESLint `no-alert` rule passes (no `alert()` / `confirm()` left in `web/`).
- [ ] All 8-10 callsites migrated (PR body lists each before → after).
- [ ] Bundle delta ≤ 8 KB gzipped (Sonner is tiny).
- [ ] `pnpm test && pnpm typecheck && pnpm lint` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-spa-toast-notification-system`
- Title: `feat(web): Sonner-based toast system + 8 callsite migrations + no-alert lint (mission-spa-toast-notification-system)`
- Worker verification: pnpm checks + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Sonner over shadcn's `Toast` primitive.** Shadcn's built-in toast is verbose + has bugs around stacking. Sonner is what shadcn maintainers actually use in their own projects.
- **Don't lose error context.** When migrating from `alert(err.message)`, the toast should carry both a short headline AND the err.message in `description`.
- **`window.confirm` doesn't have a 1:1 equivalent.** For destructive confirms, use the existing `<AlertDialog/>` shadcn primitive — file as separate `<Confirm/>` brief if any callsites are tricky.
- **Sonner positioning** — keep its default; don't bottom-center on mobile (collides with mobile browser chrome).
- **`description` is optional**, headline always present. Don't force description for short success cases.
- **Idempotency**: clicking "Approve" twice quickly shouldn't show two success toasts. Sonner's `id` prop handles this; pass `{ id: 'action-resolve-' + actionId }`.
