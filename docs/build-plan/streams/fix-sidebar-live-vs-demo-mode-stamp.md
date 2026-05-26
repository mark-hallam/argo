# fix-sidebar-live-vs-demo-mode-stamp

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-sidebar-live-vs-demo-mode-stamp`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F16 (P1).

🟠 **P1 — There is no SPA-side affordance for the operator to tell at a glance whether the dev server is running in demo mode or honest mode. The DemoBanner shows when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set, but honest mode shows nothing — yet the difference is load-bearing for recording day (in honest mode, the hire wizard 404s on readback; in demo mode it walks cleanly). The dev server inherits env vars from the shell that launched it, not `.env.local`. If the operator opens a fresh terminal and runs `npm run dev` without re-exporting the env var, demo mode is silently off and Scene 3 of the runbook breaks.**

## Root cause

The runtime mode (demo vs honest, auth-bypass on vs off) is invisible on the rendered chrome. `DemoBanner.tsx` only renders when `isDemoMode()` is true; in honest mode the sidebar carries no marker. Result: the operator pre-record can't tell which mode they're in until they click through Scene 3 and either land on Maya's profile (demo) or "Worker not found" (honest).

The fix is a small, visible-but-low-key footer stamp on the persistent sidebar: "Demo mode" with a purple dot, OR "Live mode" with a green dot. Either is honest signaling. The operator can read it before they hit record.

This is complementary to `fix-demo-recording-disable-auth-bypass-prep` (which warns the operator at startup) — that's a CLI warning; this is the always-visible chrome marker.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F16 [P1] — Sidebar shows demo persona "Sam Harlow · Acmeco · Owner" even with `NEXT_PUBLIC_SKYCOT_DEMO_MODE` off; operator has no visible signal of mode

## Fix recipe

1. Add a `<ModeStamp>` component at the bottom of the persistent sidebar (`web/components/skycot/Sidebar.tsx` or similar):
   ```tsx
   const mode = isDemoMode() ? "demo" : "live";
   return (
     <div className={`mode-stamp mode-stamp-${mode}`} data-testid="sidebar-mode-stamp">
       <span className="mode-stamp-dot" />
       <span className="mode-stamp-label">
         {mode === "demo" ? "Demo mode" : "Live mode"}
       </span>
     </div>
   );
   ```
2. Style: small font (~11px), low-contrast text (`color: var(--ink-quiet)`), dot colored by mode (purple for demo, green for live).
3. Hover tooltip: in demo mode → "Showing sample data. Set `NEXT_PUBLIC_SKYCOT_DEMO_MODE=0` to go live."; in live mode → "Connected to gateway at <gateway-url>."
4. Verify visibility at the bottom of the sidebar at all viewport widths.
5. (Optional) Hide the stamp on `next build` production builds (it's a dev-mode operator aid; in prod everyone should be live and the dot is noise).

## Acceptance criteria

- [ ] Sidebar carries a visible "Demo mode" or "Live mode" stamp at the bottom.
- [ ] Operator can tell, at a glance, which mode the dev server is in before hitting record.
- [ ] Hover tooltip explains how to flip the mode.
- [ ] Stamp respects the brand-restrained register (low-contrast text, single dot — no banner / no gradient).
- [ ] No regression to existing sidebar layout (cohorts order, nav items, workspace pill, user menu).

## May-touch

- `web/components/skycot/Sidebar.tsx` (or wherever the persistent sidebar lives) — ~20 LOC stamp component
- `web/components/skycot/ModeStamp.tsx` (NEW — ~30 LOC)
- `web/components/skycot/ModeStamp.test.tsx` (NEW — ~20 LOC vitest)
- (optional) `web/.env.example` add a comment about `NEXT_PUBLIC_SKYCOT_DEMO_MODE` semantics

## Out of scope

- The deeper demo-banner copy-honesty issue (covered by `fix-demo-banner-copy-branch-on-data-source-or-suppress`).
- Auto-disabling auth-bypass for the recording (covered by `fix-demo-recording-disable-auth-bypass-prep`).
- The env-var naming mismatch on `web/.env.local` (covered by `fix-web-mock-bypass-env-var-rename`).

## Effort

S — ~70 LOC across the component, vitest, and sidebar wiring. ~1 hour. P1 because the missing signal has bitten Scene 3 of the demo recording at least once (per audit observation) and will bite again on every fresh-terminal session.
