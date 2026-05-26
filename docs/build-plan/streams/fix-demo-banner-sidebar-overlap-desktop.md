# fix-demo-banner-sidebar-overlap-desktop

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-banner-sidebar-overlap-desktop`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-6-claim-6-trust-and-approvals.md` F6 [P1].

🟠 **P1 — `<DemoBanner />` spans the full viewport width including the area under the left sidebar. On desktop, the banner text "Demo mode — showing sample data. Sign in to see your own workspace." has its leading word "Demo " hidden behind the SKYCOT logo block. Visible on every (app)-group page when the banner is undismissed. Camera will read partial text.**

## Root cause

`web/components/skycot/AppShell.tsx:38` places `<DemoBanner />` at the root of the shell, OUTSIDE the `<main className="md:pl-64">` content column:

```tsx
<AppShell>
  <Sidebar />            {/* fixed left, w-64 */}
  <DemoBanner />         {/* starts at x=0, includes the area under the sidebar */}
  <main className="md:pl-64">{children}</main>
</AppShell>
```

The banner's inner container has `max-w-5xl mx-auto`, so the visible text is centered — but the centering measures from x=0, not from x=256px (the sidebar width). The first chars of the banner copy fall under the sidebar.

Two fixes:
- (a) Move `<DemoBanner />` INSIDE `<main>` so it lives in the content column.
- (b) Add `md:pl-64` to the banner's outer div so its centering aligns with the content column.

Either is valid; (a) is the cleaner refactor.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F6 [P1] — DemoBanner spans full viewport; sidebar overlaps the first 2 words on desktop

## Fix recipe

1. **Move `<DemoBanner />` inside `<main>`** at `web/components/skycot/AppShell.tsx:38`:
   ```tsx
   <main className="md:pl-64">
     <DemoBanner />
     <WideRailStyles />
     {children}
   </main>
   ```
2. **Verify mobile layout** — when sidebar collapses on `<md` screens, the banner should remain full-width. The `md:pl-64` class only applies the offset on `md+`, so mobile is unaffected.
3. **Verify the styled-jsx pattern** — per `feedback_styled_jsx_requires_use_client`, if the DemoBanner uses `<style jsx>` blocks it must keep `"use client"` at the top. Don't remove it during this refactor.
4. **Smoke test** by booting in demo mode and screenshotting at desktop (1440px) and tablet (768px); verify the banner text starts at the content-column edge, not at x=0.
5. **Add a visual-regression spec** (or update the existing app-shell spec):
   - At desktop viewport, assert the banner's bounding box starts at x ≥ 256px (sidebar width).
   - Assert the banner text "Demo mode — showing sample data" is fully visible (not clipped).

## Acceptance criteria

- [ ] DemoBanner does not overlap with the sidebar at desktop viewport (≥768px).
- [ ] Banner text "Demo mode — showing sample data" reads in full without truncation.
- [ ] Mobile viewport keeps the banner full-width (sidebar is hidden).
- [ ] Dismissed banner state (localStorage `skycot:demo-banner-dismissed`) still suppresses correctly.
- [ ] New visual-regression assertion passes.

## May-touch

- `web/components/skycot/AppShell.tsx:38`
- `web/components/skycot/DemoBanner.tsx` (verify styled-jsx behavior)
- `web/playwright/app-shell/demo-banner-sidebar-no-overlap.spec.ts` (NEW)

## Out of scope

- The banner copy itself (covered by `fix-demo-mode-banner-investor-safe-copy`).
- The MOCK-OFFLINE pill on `/team` (covered by `fix-team-page-mock-offline-chip-suppress-during-recording`).
- The PWA install prompt suppression (covered by `fix-pwa-install-prompt-suppress-during-demo-and-localhost`).

## Effort

~10-30 LOC + 1 spec. ~1-2 hours. P1 because the banner is on-screen for every demo session and the text-clipping is camera-visible.
