import { expect, test } from "@playwright/test";

/**
 * Regression for F6 from chrome-live-walk-and-playwright 2026-05-26:
 * the "/workflows" index used to render the seed-fallback grid in
 * parallel with the "Loading workflows…" label, because the grid was
 * gated on `workflows.length` (which fell through to the bundled seed
 * while `data` was undefined) and the loading label was gated on the
 * separate `isLoading` flag.
 *
 * After the fix, while `isLoading` is true ONLY the loading state UI
 * renders; the grid + section label + count-pill content only appear
 * after the query settles.
 */

test("Workflows index hides the 'Loading workflows…' label once cards render", async ({
  page,
}) => {
  await page.goto("/workflows");

  // The intentionally-unroutable gateway in the local lane settles the
  // useWorkflows() query quickly with the seed-fallback shape. Wait for
  // the first card to render, then assert the loading label is gone.
  const firstCard = page.locator(".workflow-card").first();
  await expect(firstCard).toBeVisible();

  await expect(page.getByTestId("workflows-loading")).toHaveCount(0);
  await expect(page.getByText("Loading workflows…")).toHaveCount(0);
});
