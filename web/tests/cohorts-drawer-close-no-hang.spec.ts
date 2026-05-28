import { expect, test } from "@playwright/test";

// Regression for audit T2351 F16: after dismissing the empty-cohort
// drawer, the page was unresponsive for ~30s. Root cause was missing
// focus restoration + body-scroll lock on the drawer; both are now
// implemented. This spec asserts that closing the drawer leaves the
// page click-responsive immediately.
test("closing the cohort drawer leaves the page click-responsive", async ({
  page,
}) => {
  await page.goto(appUrl("/cohorts"));

  await expect(page.getByTestId("cohorts-page")).toBeVisible();
  const firstTile = page.locator("[data-cohort-tile]").first();
  await expect(firstTile).toBeVisible();
  await firstTile.click();

  const drawer = page.getByTestId("cohort-detail-drawer");
  await expect(drawer).toBeVisible();

  const closeBtn = page.getByTestId("cohort-detail-close");
  await closeBtn.click();
  await expect(drawer).toHaveCount(0);

  // After close, body scroll must not be locked. The drawer's cleanup
  // restores the prior `overflow` value (empty string in a fresh page).
  const overflow = await page.evaluate(() => document.body.style.overflow);
  expect(overflow).not.toBe("hidden");

  // The next tile click must register within 500ms — the audit's
  // pre-fix timing was ~30s. Use a tight timeout to surface any
  // regression instead of waiting on Playwright's default 30s.
  const t0 = Date.now();
  const tiles = page.locator("[data-cohort-tile]");
  const tileCount = await tiles.count();
  const targetTile = tileCount > 1 ? tiles.nth(1) : tiles.first();
  await targetTile.click({ timeout: 500 });
  const elapsed = Date.now() - t0;
  expect(elapsed).toBeLessThan(2000);

  // The drawer should re-open for the second tile — confirms the page
  // is genuinely interactive, not silently swallowing clicks.
  await expect(page.getByTestId("cohort-detail-drawer")).toBeVisible();
});

function appUrl(pathname: string): string {
  const base =
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(pathname, base).toString();
}
