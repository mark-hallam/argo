import { expect, test } from "@playwright/test";

/**
 * Approvals inbox — filter popover (E1-b).
 *
 * The webServer in `playwright.config.ts` boots the page against
 * the bundled mock dataset (see approvals-inbox.spec.ts notes).
 *
 * Asserts:
 *   1. Filter button opens the popover.
 *   2. Selecting worker "Mariah" filters the visible card list.
 *   3. The chip rail surfaces the active filter.
 *   4. Removing the chip restores all cards.
 *   5. Filter state round-trips through the URL.
 */
test("filter by worker, then clear via chip", async ({ page }) => {
  await page.goto("/");

  const initialCount = await page.getByTestId("approval-approve").count();
  expect(initialCount).toBeGreaterThan(1);

  await page.getByTestId("filter-trigger").click();
  await expect(page.getByTestId("filter-popover-content")).toBeVisible();

  await page.getByTestId("filter-worker-Mariah").click();

  await expect(page).toHaveURL(/worker=Mariah/);
  await expect(page.getByTestId("filter-trigger-dot")).toBeVisible();

  // Close popover by clicking outside; chip rail shows.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("filter-chip-rail")).toBeVisible();
  await expect(page.getByTestId("filter-chip-worker")).toContainText("Mariah");

  const filteredCount = await page.getByTestId("approval-approve").count();
  expect(filteredCount).toBeLessThan(initialCount);
  expect(filteredCount).toBeGreaterThan(0);

  await page.getByTestId("filter-chip-worker-remove").click();
  await expect(page).not.toHaveURL(/worker=/);

  const restoredCount = await page.getByTestId("approval-approve").count();
  expect(restoredCount).toBe(initialCount);
});
