import { expect, test } from "@playwright/test";

/**
 * Approvals inbox — mock-fallback smoke (E1-a).
 *
 * The webServer in `playwright.config.ts` boots `pnpm dev` with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` and `NEXT_PUBLIC_GATEWAY_URL`
 * pointing at a closed port. Both knobs route the page into the
 * bundled mock dataset (auth-bypass short-circuits the data fetch,
 * and even if it didn't the gateway is unreachable) — that is the
 * deterministic e2e shape the brief calls for.
 *
 * Asserts:
 *   1. The page-header `data-source` attribute reflects a mock origin
 *      (the visible DataSourceChip is suppressed by default per
 *      `fix-team-page-mock-offline-chip-suppress-during-recording`).
 *   2. Approving the first card flips its post-decision bar to
 *      "Approved" — the optimistic mutation path still wires
 *      through the (now mock) `submitDecision`.
 */
test("approves the first card under mock fallback", async ({ page }) => {
  await page.goto("/");

  const header = page.getByTestId("approvals-page-header");
  const source = await header.getAttribute("data-source");
  expect(source === "mock-offline" || source === "mock-auth-bypass").toBe(true);
  // The visible chip is opt-in (NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP=1)
  // and must NOT render with the default Playwright env profile.
  await expect(page.getByTestId("data-source-chip")).toHaveCount(0);

  const firstApprove = page.getByTestId("approval-approve").first();
  await firstApprove.click();

  // Post-decision bar replaces the action row with "Approved".
  await expect(page.getByText("Approved").first()).toBeVisible();
});
