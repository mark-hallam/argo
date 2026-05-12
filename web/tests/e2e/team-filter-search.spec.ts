import { expect, test } from "@playwright/test";

/**
 * Team page — status filter + search with URL-persistent state (E3-d).
 *
 * The webServer in `playwright.config.ts` boots the page against the
 * bundled mock dataset (`SAMPLE_TEAM` — six personas) by pointing
 * `NEXT_PUBLIC_GATEWAY_URL` at a dead port so the gateway client
 * falls back deterministically.
 *
 * Asserts:
 *   1. Picking a status from the dropdown filters the grid + writes ?status=.
 *   2. Typing a query into the search input filters the grid + writes ?q=.
 *   3. Active filters surface as chips; clicking × on a chip removes that
 *      dimension and restores the corresponding subset.
 *   4. "Clear all" wipes both dimensions from the URL.
 */
test("filter by status, search by name, then clear chips", async ({ page }) => {
  await page.goto("/team");

  const initialCount = await page.getByTestId(/^teammate-wkr_/).count();
  expect(initialCount).toBeGreaterThan(1);

  // 1. Status filter shrinks the visible set + persists to URL.
  await page.getByTestId("team-status-filter").selectOption("waiting");
  await expect(page).toHaveURL(/status=waiting/);
  await expect(page.getByTestId("team-status-filter-dot")).toBeVisible();
  const waitingCount = await page.getByTestId(/^teammate-wkr_/).count();
  expect(waitingCount).toBeGreaterThan(0);
  expect(waitingCount).toBeLessThan(initialCount);

  // 2. Chip rail surfaces the active status.
  await expect(page.getByTestId("team-filter-chip-rail")).toBeVisible();
  await expect(page.getByTestId("team-filter-chip-status")).toContainText("Status: Waiting on you");

  // 3. Add a search query on top.
  await page.getByTestId("team-search-input").fill("mariah");
  await expect(page).toHaveURL(/q=mariah/);
  await expect(page.getByTestId("team-filter-chip-search")).toContainText('Search: "mariah"');

  // 4. Remove the search chip → q cleared, status retained.
  await page.getByTestId("team-filter-chip-search-remove").click();
  await expect(page).not.toHaveURL(/q=/);
  await expect(page).toHaveURL(/status=waiting/);

  // 5. Clear all → both gone, full roster back.
  await page.getByTestId("team-filter-chip-rail-clear-all").click();
  await expect(page).not.toHaveURL(/status=/);
  await expect(page).not.toHaveURL(/q=/);
  await expect(page.getByTestId("team-filter-chip-rail")).toHaveCount(0);
  const restoredCount = await page.getByTestId(/^teammate-wkr_/).count();
  expect(restoredCount).toBe(initialCount);
});
