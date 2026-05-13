import { expect, test } from "@playwright/test";

/**
 * Audit filter popover (E5-b).
 *
 * Boots against the bundled mock dataset via the gateway-bypass env in
 * `playwright.config.ts`. The 30d range is used so every seed row is
 * in scope before applying multi-select dimensions.
 *
 * Asserts:
 *   1. Filter trigger opens the popover (7 sections).
 *   2. Selecting two filters writes both to the URL and shows the
 *      chip rail.
 *   3. The filter-trigger dot lights up when any dimension is active.
 *   4. Removing one chip drops just that dimension.
 *   5. "Clear all" clears every dimension and hides the chip rail.
 */
test("multi-dimension filter popover → chip rail → clear", async ({ page }) => {
  await page.goto("/audit?range=30d");

  // Open the popover.
  await page.getByTestId("audit-filter-trigger").click();
  await expect(page.getByTestId("audit-filter-popover-content")).toBeVisible();
  await expect(page.getByTestId("audit-filter-section-worker")).toBeVisible();
  await expect(page.getByTestId("audit-filter-section-status")).toBeVisible();

  // Select a worker dimension value (Priya).
  await page.getByTestId("audit-filter-worker-wkr_01J0PRIYA").click();
  await expect(page).toHaveURL(/worker=wkr_01J0PRIYA/);

  // Expand the channel section and select Slack.
  await page.getByTestId("audit-filter-section-toggle-channel").click();
  await page.getByTestId("audit-filter-channel-slack").click();
  await expect(page).toHaveURL(/channel=slack/);

  // Filter-trigger dot is visible.
  await expect(page.getByTestId("audit-filter-trigger-dot")).toBeVisible();

  // Close popover via Escape; chip rail surfaces both chips.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("audit-chip-rail")).toBeVisible();
  await expect(page.getByTestId("audit-chip-worker")).toContainText("Priya");
  await expect(page.getByTestId("audit-chip-channel")).toContainText("Slack");

  // Click the channel chip to remove just that dimension.
  await page.getByTestId("audit-chip-channel").click();
  await expect(page).not.toHaveURL(/channel=slack/);
  await expect(page).toHaveURL(/worker=wkr_01J0PRIYA/);
  await expect(page.getByTestId("audit-chip-channel")).toBeHidden();

  // "Clear all" wipes the rail.
  await page.getByTestId("audit-chip-rail-clear-all").click();
  await expect(page.getByTestId("audit-chip-rail")).toBeHidden();
  await expect(page).not.toHaveURL(/worker=/);
});
