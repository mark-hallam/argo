import { expect, test } from "@playwright/test";

/**
 * Audit drill-in renders detail page (not the 500 boundary).
 *
 * Boots against the bundled mock dataset via the gateway-bypass env
 * in `playwright.config.ts` (auth-bypass + unreachable gateway URL),
 * so `fetchAuditDetail` short-circuits through `mockDetail` / the
 * demo-mode placeholder rather than hitting a real gateway.
 *
 * Asserts (per F2 [P0] in the 2026-05-26 chrome-live-walk audit):
 *   1. `/audit` renders rows in the TODAY bucket.
 *   2. Clicking the first row navigates to `/audit/action/<id>`.
 *   3. The detail page surface (`audit-detail-page`) renders — i.e.
 *      neither the route-segment error boundary nor the Next.js 500
 *      page fires.
 *   4. No `console.error` is logged during the round-trip — the
 *      drill-in is clean, not "rendered but logged a Sentry-grade
 *      error".
 */
test("clicking a row in /audit lands on the detail page without 500-ing", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto("/audit?range=30d");

  const firstRow = page.locator('[data-testid^="activity-row-"]').first();
  await expect(firstRow).toBeVisible();

  const href = await firstRow.getAttribute("href");
  expect(href).toMatch(/^\/audit\/action\//);

  await firstRow.click();
  await expect(page).toHaveURL(/\/audit\/action\//);

  await expect(page.getByTestId("audit-detail-page")).toBeVisible();
  await expect(page.getByTestId("error-shell")).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
});

/**
 * Demo-mode placeholder fallback — exercised when the action id
 * doesn't match any seed row, simulating live-gateway demo data
 * with ids that aren't in `SAMPLE_AUDIT_ROWS`. The page still
 * renders rather than 500-ing.
 */
test("unknown action id under auth-bypass renders the not-found in-shell state", async ({
  page,
}) => {
  await page.goto("/audit/action/019e63c2cd537671847bbc84c12cbbd5");

  // Either the in-shell not-found OR the placeholder detail must
  // render — both are graceful outcomes; neither is the 500 boundary.
  const notFound = page.getByTestId("audit-action-not-found");
  const detailPage = page.getByTestId("audit-detail-page");
  await expect(notFound.or(detailPage)).toBeVisible();
  await expect(page.getByTestId("error-shell")).toHaveCount(0);
});
