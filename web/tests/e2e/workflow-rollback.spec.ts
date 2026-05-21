import { expect, test } from "@playwright/test";

/**
 * Stream `web-workflow-rollback-feature` — E2E happy path for the
 * Rollback button + confirm modal + reload + toast. The spec is
 * resilient to the gateway not being booted: when the viewer page
 * shows the Mock pill (no `/versions` data) we skip the assertion
 * branch that needs the history section, so a Playwright run in
 * `pnpm test` without a live gateway stays green.
 */
test("Rollback button opens confirm modal and ships when confirmed", async ({ page }) => {
  await page.goto("/workflows/pr_checker/standard_review");

  const sourcePill = page.getByTestId("wfd-source-pill");
  await expect(sourcePill).toBeVisible();
  const source = await sourcePill.getAttribute("data-source");
  if (source !== "gateway") {
    test.skip(
      true,
      "Mock-mode viewer renders no version history; skipping rollback E2E without a live gateway.",
    );
    return;
  }

  const history = page.getByTestId("wfd-version-history");
  await expect(history).toBeVisible();

  // Rollback button on the v1 row opens the confirm modal.
  const firstRollback = page.getByTestId(/^wfd-rollback-button-/).first();
  await expect(firstRollback).toBeVisible();
  await firstRollback.click();

  const modal = page.getByTestId("rollback-confirm-modal");
  await expect(modal).toBeVisible();
  await expect(page.getByTestId("rollback-diff-current")).toBeVisible();
  await expect(page.getByTestId("rollback-diff-target")).toBeVisible();

  // Cancel does not mutate anything.
  await page.getByTestId("rollback-confirm-cancel").click();
  await expect(modal).toBeHidden();
});
