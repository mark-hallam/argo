# fix-appshell-user-menu-radix-popover-hydration

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-appshell-user-menu-radix-popover-hydration`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F07 (P1).

🟠 **P1 — Every (app)-group page hydrates with a Radix popover ID mismatch on the `appshell-user-menu-trigger` and `help-widget-trigger` elements. React's console reports `aria-controls="radix-_R_7lmitmrlb_"` server-rendered ID differs from `aria-controls="radix-_R_7lqitmrlb_"` on the client. This fires on every (app)-group page load. Investors / SOC2 reviewers with DevTools open during the demo see a red React hydration error; the affected buttons also keep mismatched `aria-controls` for assistive tech (React explicitly does not patch these up).**

## Root cause

Radix UI's `Popover` (and `Tooltip`, `DropdownMenu`, etc.) auto-generates internal IDs (`radix-_R_*`) at render time. When the component renders on both server and client and the ID generator produces a different value in each environment (different React render counter, different process state), React's hydration phase detects the mismatch on the `aria-controls` attribute and warns. Two specific places in `web/components/skycot/AppShell.tsx` (the user-menu Popover) and `web/components/ui/HelpWidget.tsx` (the help-widget Popover) trip this.

Two clean fixes:
1. **`useId()` wrapper** — wrap the trigger's `id` in a `React.useId()`-derived value so the ID is deterministic across server + client. Radix's newer versions support passing an explicit `id` prop on the Popover anchor.
2. **Client-only mount** — load AppShell user-menu and help widget via `next/dynamic(() => import(...), { ssr: false })` so the server doesn't emit the trigger at all. The chrome briefly appears empty on first paint (negligible) and the popover hydrates client-side without an ID mismatch.

Option 2 is the lower-risk fix; option 1 requires verifying Radix's version exposes the id-prop API for the popover variant in use. Recommend option 2 for v1.0 and follow up with option 1 once we tune the chrome.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F07 [P1] — Hydration mismatch on every (app)-group page (Radix popover IDs on appshell-user-menu-trigger + help-widget-trigger)

## Fix recipe

1. Identify the Popover (or Tooltip) wrappers in `web/components/skycot/AppShell.tsx` (user-menu trigger) and `web/components/ui/HelpWidget.tsx` (help-widget trigger).
2. Wrap both in `next/dynamic(() => import("..."), { ssr: false })`:
   ```ts
   const UserMenuTrigger = dynamic(() => import("./UserMenuTrigger"), { ssr: false });
   const HelpWidgetTrigger = dynamic(() => import("./HelpWidgetTrigger"), { ssr: false });
   ```
3. Verify the chrome doesn't flicker on first paint — both triggers are in the persistent app-shell, so the user only sees the first-paint state on a hard reload.
4. (Optional follow-up) audit the rest of the app for Radix-related hydration mismatches (`rg -n "radix-_R_" .next/server/`) and apply the same fix to any other repeat offenders.
5. Add a playwright smoke test: open `/team` in headless mode, capture the console; assert zero `Warning: Text content does not match` or `aria-controls` hydration warnings.

## Acceptance criteria

- [ ] Loading any (app)-group page in Chrome with DevTools open shows no Radix hydration warnings in the console.
- [ ] User-menu trigger and help-widget trigger still function (click opens the popover, escape closes, focus traps correctly).
- [ ] No visible flicker / first-paint regression beyond the ~50ms client-mount delay.
- [ ] Playwright smoke asserts the console is clean of hydration warnings.

## May-touch

- `web/components/skycot/AppShell.tsx` (~10 LOC dynamic-import wrapper)
- `web/components/ui/HelpWidget.tsx` (~10 LOC dynamic-import wrapper)
- (potentially) `web/components/skycot/UserMenuTrigger.tsx` (new file or extracted module)
- `web/tests/e2e/app-shell-hydration.spec.ts` (NEW — ~30 LOC playwright smoke)

## Out of scope

- Full Radix migration to deterministic IDs (would touch every popover/tooltip/dropdown across the app; broader v1.x cleanup).
- Other hydration warnings that aren't Radix-induced (covered by `fix-spa-hydration-warnings-theme-and-motion-providers` for the theme/motion provider class).

## Effort

S — ~50 LOC across two wrappers + a playwright test. ~1 hour. P1 because the warning appears on every app-group page load and is visible to anyone with DevTools open during the demo recording. The fix is mechanically cheap.
