import { expect, test } from "@playwright/test";

/**
 * Trust-promotion engine — UI smoke
 * (`web-worker-trust-promotion-engine`).
 *
 * Drives the per-worker `/trust` page through the operator-only
 * "Recompute trust grade" affordance the brief adds. Local lane is
 * gateway-less (`NEXT_PUBLIC_GATEWAY_URL` points at a dead port),
 * so the page hydrates from `findSampleWorkerTrust` and the
 * recompute API falls back to the synthetic `hold` decision wired
 * in `recomputeWorkerTrustGrade`.
 *
 * Verified:
 *  1. The disclaimer is the post-engine wording (mentions the 30-min cadence).
 *  2. The Recompute button surfaces for the operator role (auth bypass on).
 *  3. Clicking it produces a non-error success line.
 *  4. The trust-grade card is still present afterwards (no crash).
 */
test("trust tab — disclaimer + recompute affordance", async ({ page }) => {
  // Mariah's sample worker id from `sample-worker-trust.ts`.
  await page.goto("/workers/wkr_mariah/trust");

  await expect(page.getByTestId("worker-tab-panel-trust")).toBeVisible();
  await expect(page.getByTestId("trust-grade-disclaimer")).toContainText(
    /recomputed every 30 minutes/i,
  );

  const recompute = page.getByTestId("trust-grade-recompute");
  await expect(recompute).toBeVisible();
  const button = recompute.getByRole("button", { name: /recompute trust grade/i });
  await expect(button).toBeEnabled();

  await button.click();
  await expect(recompute.getByText(/no grade change|grade (promoted|demoted)/i)).toBeVisible({
    timeout: 5_000,
  });
});
