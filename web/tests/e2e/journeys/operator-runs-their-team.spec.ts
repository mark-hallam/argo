import { expect, test } from "@playwright/test";

/**
 * JOURNEY — Operator runs their team (the governance loop).
 *
 * The daily-driver path: see the roster → open a teammate → review what
 * they've done in the audit trail → drill into one action → land in the
 * approvals inbox where the human sign-off lives. This chains the four
 * surfaces that prove claims 1/2/6 (teammate · delegation · multi-channel
 * approvals) as one continuous flow.
 *
 * Local lane: auth-bypass + dead gateway → deterministic SAMPLE_* data.
 * Anchors are testids, never networkidle.
 */
test("team → worker detail → audit drill-in → approvals inbox", async ({ page }) => {
  // 1. Roster renders the trading-card deck (never the empty state).
  await page.goto("/team");
  const cards = page.getByTestId(/^teammate-[0-9a-f]{8}-/);
  await expect(cards.first()).toBeVisible();
  await expect(page.getByTestId("team-empty-state")).toHaveCount(0);

  // 2. Open a teammate's profile via the card's "Open profile" link.
  await page.getByRole("link", { name: /open profile/i }).first().click();
  await expect(page).toHaveURL(/\/workers\/[^/]+\/overview$/);
  await expect(page.getByTestId("worker-name")).toBeVisible();
  await expect(page.getByTestId("worker-not-found")).toHaveCount(0);

  // 3. Cross to the audit trail and drill into the first action row.
  await page.goto("/audit");
  const firstRow = page.locator('a[href*="/audit/action/"]').first();
  await expect(firstRow).toBeVisible();
  await firstRow.click();
  await expect(page).toHaveURL(/\/audit\/action\//);
  await expect(page.getByTestId("audit-detail-page")).toBeVisible();
  await expect(page.getByTestId("error-shell")).toHaveCount(0);

  // 4. End in the approvals inbox — the human sign-off surface. Every
  //    pending row must carry Approve + Refuse (the claim-6 contract).
  await page.goto("/approvals");
  await expect(page.getByTestId("approvals-ops-dashboard")).toBeVisible();
  await expect(page.locator('[data-testid^="approve-"]').first()).toBeVisible();
  await expect(page.locator('[data-testid^="refuse-"]').first()).toBeVisible();
});
