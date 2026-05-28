/**
 * Regression guard for `fix-appshell-user-menu-radix-popover-hydration`.
 *
 * Radix UI's `Popover.Root` (used by the sidebar user-menu pill) and
 * `Dialog.Root` (used by the floating help widget) auto-generate
 * `radix-_R_*` IDs from a per-render counter. Server-rendering the
 * trigger and then re-rendering it on the client produces different
 * IDs on each side, which React surfaces as an `aria-controls`
 * hydration mismatch in the browser console on every (app)-group
 * page load — visible during demo recording with DevTools open.
 *
 * The fix loads `UserPill` via `next/dynamic({ ssr: false })` from
 * `AppShell.tsx` and gates `HelpWidget` behind a client-only
 * `mounted` flag so the server emits nothing where either trigger
 * goes and both popovers hydrate cleanly client-side.
 *
 * This spec navigates to an (app)-group page, waits for the page to
 * settle, and asserts the captured console transcript contains no
 * Radix / hydration mismatch warnings.
 */

import { expect, test } from "@playwright/test";

const HYDRATION_PATTERNS = [
  /hydration/i,
  /aria-controls/i,
  /did not match/i,
  /text content does not match/i,
  /radix-_R_/i,
];

function looksLikeHydrationWarning(text: string): boolean {
  return HYDRATION_PATTERNS.some((re) => re.test(text));
}

test("(app)-group pages render without Radix hydration warnings", async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() !== "error" && msg.type() !== "warning") return;
    const text = msg.text();
    if (looksLikeHydrationWarning(text)) consoleErrors.push(`[${msg.type()}] ${text}`);
  });
  page.on("pageerror", (err) => {
    const text = err.message;
    if (looksLikeHydrationWarning(text)) consoleErrors.push(`[pageerror] ${text}`);
  });

  await page.goto("/team");

  // Wait for the user-menu trigger to mount client-side; this proves
  // both the dynamic-loaded `UserPill` and (by extension) the rest of
  // the app shell finished hydration.
  await expect(page.getByTestId("appshell-user-menu-trigger")).toBeVisible();
  await expect(page.getByTestId("help-widget-trigger")).toBeVisible();

  expect(
    consoleErrors,
    `Hydration warnings leaked to the console:\n${consoleErrors.join("\n")}`,
  ).toEqual([]);
});
