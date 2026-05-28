import { expect, test } from "@playwright/test";

/**
 * Settings stub-route redirects. Operators (and external docs /
 * quickstart guides) frequently type `/settings/branding`,
 * `/settings/integrations`, or `/settings/danger` even though the
 * locked Settings design folds those concepts into existing tabs.
 * Configured in `web/next.config.ts` `redirects()`. Spec source:
 * stream `fix-settings-stub-routes-branding-integrations-danger`.
 *
 * Two assertions per case:
 *
 * 1. Raw 307 + `Location` header (via `request.get` with
 *    `maxRedirects: 0`). This is the contract the brief locks in.
 * 2. Followed navigation lands on the destination pathname and (where
 *    applicable) carries the anchor through to the URL fragment. The
 *    appearance/danger-zone anchors are rendered in the Settings
 *    layout aside (`#danger-zone`) and the workspace page's
 *    appearance field, so the browser scrolls without 404'ing.
 */

const CASES = [
  {
    source: "/settings/branding",
    targetPath: "/settings/workspace",
    targetHash: "#appearance",
  },
  {
    source: "/settings/integrations",
    targetPath: "/settings/connectors",
    targetHash: "",
  },
  {
    source: "/settings/danger",
    targetPath: "/settings/workspace",
    targetHash: "#danger-zone",
  },
] as const;

for (const { source, targetPath, targetHash } of CASES) {
  test(`${source} 307-redirects to ${targetPath}${targetHash}`, async ({ request }) => {
    const res = await request.get(source, { maxRedirects: 0 });
    expect(res.status()).toBe(307);
    const location = res.headers()["location"];
    expect(location).toBeDefined();
    expect(location).toBe(`${targetPath}${targetHash}`);
  });
}

test("followed redirect lands on /settings/workspace + danger-zone anchor present", async ({
  page,
}) => {
  await page.goto("/settings/danger");
  await expect(page).toHaveURL(/\/settings\/workspace#danger-zone$/);
  await expect(page.locator("#danger-zone")).toBeVisible();
});

test("followed redirect lands on /settings/connectors page", async ({ page }) => {
  await page.goto("/settings/integrations");
  await expect(page).toHaveURL(/\/settings\/connectors$/);
  await expect(page.getByTestId("connectors-page")).toBeVisible();
});

test("followed redirect lands on /settings/workspace appearance field", async ({ page }) => {
  await page.goto("/settings/branding");
  await expect(page).toHaveURL(/\/settings\/workspace#appearance$/);
  await expect(page.getByTestId("settings-workspace-theme-picker")).toBeVisible();
});
