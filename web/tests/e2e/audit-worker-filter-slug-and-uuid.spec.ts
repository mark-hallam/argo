import { expect, test } from "@playwright/test";

/**
 * Slug vs UUID parity for the `/audit?worker=` deep-link
 * (`fix-audit-worker-filter-uses-uuid-not-slug`).
 *
 * The worker Activity tab used to deep-link with the raw UUID
 * (`/audit?worker=018f67d0-…`). Screen-shares + Slack pastes would
 * leak a 36-character internal id. The fix:
 *
 *   1. Worker Activity tab builds the link with the persona slug
 *      (`?worker=mariah`).
 *   2. `/audit` parses both — slugs translate to UUIDs at parse
 *      time, UUIDs pass through unchanged, so existing
 *      filter-popover-emitted URLs still work.
 *
 * Boots against the bundled mock dataset via the gateway-bypass env
 * in `playwright.config.ts`. The 30d range ensures every seed row is
 * in scope before the worker filter is applied. The chip rail shows
 * the canonical persona name when either id form resolves correctly.
 *
 * Brief sensible deviation: the brief named
 * `web/playwright/audit/worker-filter-slug-and-uuid.spec.ts`, but
 * the repo's e2e specs all live under `web/tests/e2e/` (see
 * `playwright.config.ts`'s `testDir`). Spec lands here to match
 * the existing convention.
 */

// Mariah — canonical Scene-1 persona; seeds the audit fixture.
const MARIAH_UUID = "018f67d0-0000-7000-8000-000000000001";
const MARIAH_SLUG = "mariah";

test("/audit?worker=<slug> filters to the matching persona", async ({ page }) => {
  await page.goto(`/audit?worker=${MARIAH_SLUG}&range=30d`);

  await expect(page.getByTestId("audit-chip-rail")).toBeVisible();
  await expect(page.getByTestId("audit-chip-worker")).toContainText("Mariah");

  // Feed renders rows (i.e. the slug actually filtered, did not
  // accidentally exclude every row).
  await expect(page.getByTestId("audit-feed")).toBeVisible();
});

test("/audit?worker=<uuid> still filters identically (backward compat)", async ({ page }) => {
  await page.goto(`/audit?worker=${MARIAH_UUID}&range=30d`);

  await expect(page.getByTestId("audit-chip-rail")).toBeVisible();
  await expect(page.getByTestId("audit-chip-worker")).toContainText("Mariah");
  await expect(page.getByTestId("audit-feed")).toBeVisible();
});

test("worker Activity tab 'Open in Audit timeline' link uses the slug, not the UUID", async ({
  page,
}) => {
  await page.goto(`/workers/${MARIAH_UUID}/activity`);

  const link = page.getByTestId("activity-open-audit");
  await expect(link).toBeVisible();

  const href = await link.getAttribute("href");
  expect(href).toBeTruthy();
  expect(href).toContain(`worker=${MARIAH_SLUG}`);
  expect(href).not.toContain(MARIAH_UUID);
});
