# fix-pwa-install-prompt-suppress-during-demo-and-localhost

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pwa-install-prompt-suppress-during-demo-and-localhost`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F8 [observation].

🟡 **P2 — "Install SKYCOT — Get it on your homescreen for one-tap access. [Not now] [Install]" toast sits bottom-right on /team, /audit, /onboarding/workspace, and others. Overlaps the help bubble and pollutes every screenshot during demo recording. Should be suppressed on localhost origins and during recording-mode sessions.**

## Root cause

The PWA install prompt is a browser-driven `beforeinstallprompt` event that the SPA chooses to surface as a toast. Currently it fires whenever the browser deems the PWA installable, with no override for development or recording contexts. The toast is fine for the operator's daily browsing but ruins screenshot polish.

## Findings this brief closes

- chrome-live-walk-and-playwright F8 [observation] — PWA install prompt overlays interactive areas

## Fix recipe

1. **Suppress on localhost origins**. In the PWA-install component (locate via grep for "Install SKYCOT" or "beforeinstallprompt"):
   ```ts
   const isLocalhost = typeof window !== "undefined" 
     && /^(localhost|127\.0\.0\.1|\.local)$/.test(window.location.hostname);
   if (isLocalhost) return null;
   ```

2. **Suppress when `?hideInstallPrompt=1` is in the URL** (operator override for ad-hoc cases):
   ```ts
   const url = new URL(window.location.href);
   if (url.searchParams.has("hideInstallPrompt")) return null;
   ```

3. **Suppress when `ARGO_DEMO_RECORDING=1` env var is set** (operator-locked flag for recording sessions, distinct from `ARGO_DEMO_MODE` which is for sample-data):
   ```ts
   if (process.env.NEXT_PUBLIC_SKYCOT_DEMO_RECORDING === "1") return null;
   ```

4. **Persist the "Not now" dismissal** so a dismissed prompt stays dismissed across the recording session (localStorage with a 30-day TTL — current behaviour may already do this, verify).

5. **Add a Playwright spec** at `web/playwright/recording-mode-suppresses-install-prompt.spec.ts`:
   - Set `NEXT_PUBLIC_SKYCOT_DEMO_RECORDING=1` via Playwright env
   - Load `/team`
   - Assert no element matching `[data-testid="pwa-install-prompt"]` exists

## Acceptance criteria

- [ ] Loading any page on localhost: no install prompt visible
- [ ] Loading any page with `?hideInstallPrompt=1`: no install prompt
- [ ] Loading any page with `NEXT_PUBLIC_SKYCOT_DEMO_RECORDING=1`: no install prompt
- [ ] Loading on production / staging without overrides: install prompt still works
- [ ] Dismissal persists across session
- [ ] New Playwright spec passes

## May-touch

- `web/components/skycot/PwaInstallPrompt.tsx` (or wherever the toast lives)
- `web/lib/pwa/use-install-prompt.ts` (NEW or existing hook)
- `web/playwright/recording-mode-suppresses-install-prompt.spec.ts` (NEW)
- README / dev docs noting the recording-mode env var

## Out of scope

- The PWA install logic itself (preserve)
- Manifest changes (separate concern)
- The Help floating-button positioning (separate polish brief if needed)

## Effort

~30-80 LOC + 1 spec. ~1-2 hours. P2 polish; matters every demo recording.
