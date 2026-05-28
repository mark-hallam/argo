import { expect, test } from "@playwright/test";

/**
 * /settings/* — exactly one tab carries `aria-current="page"` on every
 * sub-page. Pre-fix, the Workspace tab's `match` array contained
 * `/settings` and the matcher used `pathname.startsWith("/settings/")`,
 * so every `/settings/<sub>` route highlighted Workspace AND the
 * actual active tab simultaneously (F17 in the 2026-05-27 site UX
 * audit; double-aria-current is a screen-reader breakage).
 *
 * Brief: fix-settings-tabs-prefix-startswith-double-highlights-workspace.
 */
const SUB_PAGES = [
  { path: "/settings", expected: "Workspace" },
  { path: "/settings/workspace", expected: "Workspace" },
  { path: "/settings/members", expected: "Team" },
  { path: "/settings/notifications", expected: "Notifications" },
  { path: "/settings/security", expected: "Security" },
  { path: "/settings/providers", expected: "Providers" },
  { path: "/settings/api-keys", expected: "API keys" },
  { path: "/settings/notify-channels", expected: "Channels" },
  { path: "/settings/connectors", expected: "Connectors" },
  { path: "/settings/billing", expected: "Billing" },
  { path: "/settings/data-export", expected: "Data export" },
] as const;

for (const { path, expected } of SUB_PAGES) {
  test(`${path} renders exactly one aria-current tab (${expected})`, async ({ page }) => {
    await page.goto(path);
    const tabs = page.getByTestId("settings-tabs");
    await expect(tabs).toBeVisible();

    const active = tabs.locator('a[aria-current="page"]');
    await expect(active).toHaveCount(1);
    await expect(active).toContainText(expected);
  });
}
