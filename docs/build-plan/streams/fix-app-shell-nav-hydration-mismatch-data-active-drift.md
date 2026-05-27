# fix-app-shell-nav-hydration-mismatch-data-active-drift

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-app-shell-nav-hydration-mismatch-data-active-drift`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F5 [P1].

🟠 **P1 — Every navigation into an app-shell page logs a React hydration error to the browser console: "A tree hydrated but some attributes of the server rendered HTML didn't match the client properties." The mismatch is on `data-active` for `AppNav` links — server renders `data-active=true` against the cached previous pathname, client renders `data-active=false` against the current pathname. React warns this "won't be patched up" — so the first paint shows the wrong active tab for 100-300ms.**

## Root cause

`web/components/AppNav.tsx` (or equivalent — verify path at claim time) renders active-tab markers using `usePathname()`. Under Next.js App Router with React Server Components:

- Server renders the layout with the pathname AT THE TIME OF THE REQUEST (or stale-cached value from a prior render).
- Client hydrates with the CURRENT pathname.

When these differ — common during client-side navigations where the layout SSR cache lags the URL — the `data-active` attribute drifts. React logs the hydration warning and renders the SSR value; the client never patches it back to match.

The same root cause produces the "Audit · SKYCOT" document title leaking onto `/team` pages (observed in the audit) — the metadata `<title>` is server-stamped from the prior route before client-side route resolution. This brief covers the nav-link symptom; the title symptom is in `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`.

Console evidence from the audit:
```
A tree hydrated but some attributes of the server rendered HTML didn't match the client properties.
<_c1 href="/team" active={false}>
+   data-active={undefined}
-   data-active="true"
<_c1 href="/audit" active={true}>
+   data-active={true}
-   data-active={null}
```

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F5 [P1] — SSR/client hydration mismatch on the global nav (data-active drift)

## Fix recipe

1. **Locate `AppNav`** — `rg 'data-active' web/components/` to find the rendering component.
2. **Move active-state computation client-side only.** Two patterns:
   - **(a)** Render the nav links without `data-active` from the server (`active` defaults to `false` or `undefined`); add a `"use client"` boundary inside the nav that reads `usePathname()` and adds the active class via `useEffect`. Avoids the SSR/client mismatch entirely.
   - **(b)** Use Next.js `<Link>` with the `active` prop computed inside a `"use client"` component that wraps the nav. Server renders a neutral state; client resolves immediately on mount.
3. **Recommended: (a)** — render markup that is pathname-independent on the server, then promote `active` styling to client-side useEffect. The link still works (Next.js handles the navigation); the visual active marker is purely client-side.
4. **Update metadata `<title>`** similarly if it has the same SSR-stamps-prior-pathname pattern. Coordinate with `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages` so the two fixes converge.
5. **Vitest pin** at `web/components/__tests__/AppNav.test.tsx`: render the nav with `usePathname()` mocked to two different paths in sequence; assert no hydration warnings are logged.

## Acceptance criteria

- [ ] Navigating between `/team` / `/audit` / `/workers` produces no "A tree hydrated but some attributes" warning in console.
- [ ] Active-tab marker visually matches the current pathname from first paint (no 100-300ms wrong-tab flash).
- [ ] Vitest pin asserts no hydration warnings.
- [ ] Tab key navigation focuses links in the correct order.

## May-touch

- `web/components/AppNav.tsx` (or equivalent — locate via grep)
- `web/app/(app)/layout.tsx` (if it renders the nav)
- `web/components/__tests__/AppNav.test.tsx` (new vitest)

## Out of scope

- The `<title>` element leaking "Approvals · SKYCOT" (covered by `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`)
- Worker-detail title leaking raw UUID (covered by `fix-worker-detail-page-title-and-server-client-tab-helper`)
- Turbopack `inner_of_upper_lost_followers` panic (Next.js bug, not app)

## Effort

S — ~30-80 LOC. ~1-2 hours. P1 because while no demo scene catches the wrong-tab-flash explicitly, the hydration error in console clutters debug captures and the visible flash IS noticeable on slow connections.
