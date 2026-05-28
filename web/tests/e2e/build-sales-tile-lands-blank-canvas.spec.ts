import { expect, test } from "@playwright/test";

/**
 * Regression for scene-4-and-5 F8 [P2]:
 *
 *   `/workflows/build` → click a vertical tile (Sales) → land on
 *   `/workflows/<vertical>/untitled/build` with the blank-canvas
 *   builder. The audit caught the SPA bouncing the operator to
 *   `/onboarding/workspace` instead — confusing wrong flow.
 *
 * Lock the contract: the new-workflow sentinel `untitled` must
 * render the builder shell with no gateway redirect under it.
 */
test("clicking the Sales tile on /workflows/build opens the blank canvas, not onboarding", async ({
  page,
}) => {
  await page.goto("/workflows/build");

  // Picker is rendered (either gateway-backed or fallback list).
  await expect(page.getByTestId("workflow-build-vertical-sales")).toBeVisible();

  await page.getByTestId("workflow-build-vertical-sales").click();

  // URL lands on the sentinel name `untitled` under the sales vertical.
  await expect(page).toHaveURL(/\/workflows\/sales\/untitled\/build$/);

  // Blank-canvas mode of the builder is on screen.
  await expect(page.getByTestId("workflow-blank-canvas")).toBeVisible();

  // Draft-setup aside is the visible signal of blank-canvas mode.
  await expect(page.getByTestId("builder-new-workflow-name")).toBeVisible();

  // Hard negative — no onboarding redirect under the picker.
  await expect(page).not.toHaveURL(/\/onboarding\//);
});

const OTHER_TILES = [
  ["support", /\/workflows\/support\/untitled\/build$/],
  ["gtm-engineer", /\/workflows\/gtm-engineer\/untitled\/build$/],
  ["people-ops", /\/workflows\/people-ops\/untitled\/build$/],
  ["finance", /\/workflows\/finance\/untitled\/build$/],
] as const;

for (const [verticalId, urlMatcher] of OTHER_TILES) {
  test(`clicking the ${verticalId} tile lands on the blank canvas`, async ({ page }) => {
    await page.goto("/workflows/build");
    await page.getByTestId(`workflow-build-vertical-${verticalId}`).click();
    await expect(page).toHaveURL(urlMatcher);
    await expect(page.getByTestId("workflow-blank-canvas")).toBeVisible();
    await expect(page).not.toHaveURL(/\/onboarding\//);
  });
}
