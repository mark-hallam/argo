# web-icon-route-oklch-prerender-fix

**Owner**: claude.
**Authored against**: origin/main `ad1e38ba2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-icon-route-oklch-prerender-fix`.

## Why

`pnpm -C web build` currently fails on `origin/main` while prerendering
`/icon`:

```text
Error occurred prerendering page "/icon"
[Error: Unexpected token type: function
 in CSS rule `background: oklch(0.55 0.18 256)`.]
```

The failure reproduces in `web/app/icon.tsx`, which feeds an `ImageResponse`
style object containing raw `oklch(...)` color functions. The rest of the SPA
can use OKLCH in CSS, but this route is rendered through the OG image path,
which is stricter about inline style parsing.

## Scope

Make the `/icon` route build-safe without changing its user-facing shape:

- keep the same 32×32 PNG output and same diamond-mark composition
- replace the unsupported inline color syntax with a format accepted by
  `ImageResponse`
- add a focused regression test or snapshot so the route does not silently
  regress on future design tweaks

## Files you may touch

- `web/app/icon.tsx`
- `web/app/icons/[size]/route.tsx` only if the color helper should be shared
- `web/lib/theme/**` only if a tiny reusable color-conversion helper is the
  narrowest fix
- `web/tests/**` or `web/app/**/__tests__/**` for one focused regression test

## Files you must NOT touch

- locked design docs / seed files
- unrelated auth or onboarding code
- global theme token definitions in `web/app/globals.css`

## Acceptance criteria

- [ ] `pnpm -C web build` no longer fails on `/icon`
- [ ] `/icon` still renders the same 32×32 mark composition
- [ ] any shared helper added is tiny and isolated to color conversion only
- [ ] one focused regression test or snapshot is added
- [ ] `pnpm -C web test` stays green

## Branch + PR

- Branch: `claude-web-icon-route-oklch-prerender-fix`
- PR title: `web: fix /icon prerender on ImageResponse color parsing (web-icon-route-oklch-prerender-fix)`

## Notes

- Prefer the smallest possible fix. This is a build-breaker, not a redesign.
- If `ImageResponse` accepts hex/rgb but not OKLCH, convert at the callsite
  rather than widening theme architecture.
