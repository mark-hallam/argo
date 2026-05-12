import { expect, test } from "@playwright/test";

/**
 * Approvals inbox — mock-fallback smoke (E1-a).
 *
 * The webServer in `playwright.config.ts` boots `pnpm dev` with
 * `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` and `NEXT_PUBLIC_GATEWAY_URL`
 * pointing at a closed port. Both knobs route the page into the
 * bundled mock dataset (auth-bypass short-circuits the data fetch,
 * and even if it didn't the gateway is unreachable) — that is the
 * deterministic e2e shape the brief calls for.
 *
 * Asserts:
 *   1. DataSourceChip renders with one of the mock-* sources.
 *   2. Approving the first card flips its post-decision bar to
 *      "Approved" — the optimistic mutation path still wires
 *      through the (now mock) `submitDecision`.
 */
test("approves the first card under mock fallback", async ({ page }) => {
  await page.goto("/");

  const chip = page.getByTestId("data-source-chip");
  await expect(chip).toBeVisible();
  const source = await chip.getAttribute("data-source");
  expect(source === "mock-offline" || source === "mock-auth-bypass").toBe(true);

  const firstApprove = page.getByTestId("approval-approve").first();
  await firstApprove.click();

  // Post-decision bar replaces the action row with "Approved".
  await expect(page.getByText("Approved").first()).toBeVisible();
});
