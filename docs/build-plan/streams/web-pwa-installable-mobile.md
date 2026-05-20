# web-pwa-installable-mobile — Progressive Web App manifest + service worker + offline shell

**Owner**: claude. **Sizing**: ~200-300 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-pwa-installable-mobile`.
**Priority**: P2 — once a customer has signed up, mobile installability turns SKYCOT into a daily habit on their phone homescreen. Cheap LOC for big perceived-polish points + a story Apple and Google amplify.

## Scope

Make the SKYCOT web app installable as a Progressive Web App on iOS (Safari add-to-homescreen), Android (Chrome install prompt), and desktop (Chrome / Edge install).

### 1. Web app manifest

- New file: `web/public/manifest.webmanifest`.
- Fields:
  - `name`: "SKYCOT"
  - `short_name`: "SKYCOT"
  - `description`: "AI teammates for your business."
  - `start_url`: "/"
  - `display`: "standalone"
  - `background_color`: "#fef7f0" (brand light page bg)
  - `theme_color`: "#8B5CF6" (brand primary purple)
  - `icons`: array of icons at standard sizes (192×192, 256×256, 384×384, 512×512). Use the SKYCOT logo as the source; export PNGs at all sizes. Placeholder icons (single-color square with "S" wordmark) are acceptable if production icons aren't available yet.
  - `screenshots`: 1-2 screenshots of the main app surface (placeholder accepted).
- Link from `web/app/layout.tsx`: `<link rel="manifest" href="/manifest.webmanifest" />`.
- Also add `<meta name="theme-color" content="#8B5CF6" />` and Apple-specific meta tags (`apple-mobile-web-app-capable`, `apple-mobile-web-app-status-bar-style`, `apple-touch-icon`).

### 2. Service worker

- New file: `web/public/sw.js`.
- Minimal service worker:
  - On install: precache the offline shell page + brand assets (logo, favicon, manifest).
  - On fetch: network-first for API calls, cache-first for static assets, fall back to the offline shell page for navigation requests when offline.
- Register from a top-level client component (e.g. `web/components/pwa/ServiceWorkerRegister.tsx` mounted in `web/app/layout.tsx`):
  ```tsx
  useEffect(() => {
    if ("serviceWorker" in navigator) {
      navigator.serviceWorker.register("/sw.js").catch(console.error);
    }
  }, []);
  ```

### 3. Offline shell page

- New route: `web/app/offline/page.tsx`.
- Brand-loud register: Audiowide "You're offline." + warm radial blob bg + a 1-sentence hint ("SKYCOT will reconnect automatically when you're back online.") + brand mark.
- Optionally: a static list of recently-viewed pages (read from localStorage) so the user can navigate from the offline shell when they regain connection.

### 4. iOS Safari add-to-homescreen

- Add `<link rel="apple-touch-icon" href="/icons/apple-touch-icon.png" />` (180×180 PNG).
- `<meta name="apple-mobile-web-app-capable" content="yes" />` so it runs in standalone mode.
- `<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />`.
- `<meta name="apple-mobile-web-app-title" content="SKYCOT" />`.

### 5. Install prompt UX (Chrome / Edge / Android)

- Listen for `beforeinstallprompt` event in a top-level component.
- Stash the event; show a slim banner at the bottom of the viewport: "Install SKYCOT — get it on your homescreen." with Install + Dismiss buttons.
- On Install click, call `event.prompt()`.
- Dismiss persists to localStorage so the banner doesn't re-show for 30 days.

### 6. Lighthouse PWA audit

- Run the Chrome Lighthouse PWA audit locally. Target: ≥90 PWA score.
- Document any failing checks in a comment in the PR description (placeholder icons count as a known gap; v1.x ships real ones).

## Acceptance criteria

- [ ] `/manifest.webmanifest` returns valid JSON with name, short_name, icons, theme_color.
- [ ] Service worker at `/sw.js` registers successfully on first page load (verify in DevTools → Application → Service Workers).
- [ ] Chrome shows the install prompt on /landing (or somewhere in the app).
- [ ] The slim install banner appears + Install / Dismiss buttons work.
- [ ] Dismiss persists for 30 days.
- [ ] iOS Safari "Add to Home Screen" shows the SKYCOT icon + name correctly.
- [ ] When offline, navigating renders the offline shell page with brand styling.
- [ ] Lighthouse PWA audit score ≥90 locally.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Push notifications (v1.x — requires permission flow + gateway notification dispatch).
- Background sync (v1.x).
- Full offline functionality beyond the shell (v1.x — proper offline-first needs IndexedDB caching of API responses).
- Real-quality production icons (placeholder squares are acceptable; design owns the real export).
