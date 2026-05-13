import { expect, test } from "@playwright/test";

/**
 * End-to-end smoke for the hire-flow shell (E2-a).
 *
 * Verifies that the seven-step shell renders, navigates forward and
 * back, and ends on the celebration page which redirects to the new
 * worker's profile.
 *
 * **Status — runs once Playwright infra lands.** The repo currently
 * ships `@playwright/test` as a dev dep but no `playwright.config.ts`
 * or test:e2e CI wiring. This spec is authored against the route
 * shape this brief locks; a follow-on infra brief wires Playwright +
 * dev-server harness so `pnpm test:e2e` discovers and runs it.
 *
 * To run locally once the harness lands:
 *   pnpm --filter web build
 *   pnpm --filter web exec playwright test tests/e2e/hire-flow-shell.spec.ts
 */
test("walks through all 6 hire-flow steps and lands on the celebration redirect", async ({
  page,
}) => {
  await page.goto("/hire");
  await expect(page.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "1");
  await expect(page.getByText(/Step 1 — placeholder for E2-b/)).toBeVisible();

  // Step 1 -> Step 2 (Continue is disabled on the shell-only build, so
  // route directly until E2-b unlocks it). Step 2 now renders the real
  // Listen UI (E2-c); without a `problem` in the draft store it
  // bounces back to `/hire`, so a direct visit no longer paints Step 2
  // unless we seed state first. Skip the placeholder check and let the
  // step-2-specific spec cover that path.
  await expect(page.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "1");

  await page.goto("/hire/step/3");
  await expect(page.getByText(/Step 3 — placeholder for E2-d/)).toBeVisible();

  await page.goto("/hire/step/4");
  await expect(page.getByText(/Step 4 — placeholder for E2-e/)).toBeVisible();

  await page.goto("/hire/step/5");
  await expect(page.getByText(/Step 5 — placeholder for E2-g/)).toBeVisible();

  // Step 6 lives at /hire/done/<workerId> and auto-redirects to the
  // approvals inbox after the 1.5s celebration overlay (E2-h).
  await page.goto("/hire/done/stub");
  await expect(page).toHaveURL(/\/$/, { timeout: 5000 });
});

test("Back button is disabled on step 1 and navigates between later steps", async ({
  page,
}) => {
  await page.goto("/hire");
  await expect(page.getByTestId("hire-shell-back")).toBeDisabled();

  await page.goto("/hire/step/3");
  await page.getByTestId("hire-shell-back").click();
  // Step 3 → back lands on step 2; step 2's no-problem invariant then
  // bounces to `/hire`. Accept either as a valid landing point.
  await expect(page).toHaveURL(/\/hire(\/step\/2)?$/);
});

test("invalid step segment returns 404 within the hire route group", async ({ page }) => {
  const response = await page.goto("/hire/step/99");
  expect(response?.status()).toBe(404);
});
