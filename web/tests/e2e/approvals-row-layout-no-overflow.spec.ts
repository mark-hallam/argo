/**
 * Regression guard for
 * `fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport`.
 *
 * Before the fix, the approvals row used a single `lg:` 5-col grid
 * (`minmax(0,1fr)_7.5rem_5rem_6.5rem_auto`). When the inline Approve /
 * Refuse cluster landed on every row (#2414), the cluster crowded the
 * confidence donut on 1024-1279px viewports — the lg breakpoint kicks
 * in at 1024px but the dense row content overflows horizontally until
 * the xl breakpoint at 1280px.
 *
 * The fix splits the row template into:
 *   - md (768-1279px): 4-col grid with tighter widths, actions wrap to
 *     a second visual line via col-span-4.
 *   - xl (>=1280px):   restored 5-col grid, actions inline on row 1.
 *
 * This spec walks `/approvals` at 3 representative widths (mobile,
 * laptop, desktop) and asserts:
 *   1. The dashboard scrollWidth does not exceed clientWidth (no
 *      horizontal overflow on the page).
 *   2. The inline action cluster on the first pending row is visible
 *      and clickable.
 */

import { expect, test, type Page } from "@playwright/test";

async function assertNoHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => {
    const root = document.documentElement;
    return {
      docScroll: root.scrollWidth,
      docClient: root.clientWidth,
    };
  });
  expect(
    overflow.docScroll,
    `document horizontal overflow: scrollWidth=${overflow.docScroll} > clientWidth=${overflow.docClient}`,
  ).toBeLessThanOrEqual(overflow.docClient);
}

const VIEWPORTS = [
  { name: "mobile-375", width: 375, height: 800 },
  { name: "laptop-1024", width: 1024, height: 768 },
  { name: "desktop-1920", width: 1920, height: 1080 },
];

for (const vp of VIEWPORTS) {
  test.describe(`approvals row layout at ${vp.name}`, () => {
    test.use({ viewport: { width: vp.width, height: vp.height } });

    test(`renders without horizontal overflow and keeps inline actions reachable`, async ({
      page,
    }) => {
      await page.goto("/approvals");
      await expect(page.getByTestId("approvals-ops-dashboard")).toBeVisible();

      // Wait for at least one pending row to mount before measuring.
      const firstApprove = page.locator('[data-testid^="approve-"]').first();
      await expect(firstApprove).toBeVisible();

      await assertNoHorizontalOverflow(page);

      // Action cluster must be on-screen + clickable at every width.
      const firstRefuse = page.locator('[data-testid^="refuse-"]').first();
      await expect(firstRefuse).toBeVisible();

      const approveBox = await firstApprove.boundingBox();
      expect(approveBox, "Approve button has no bounding box").not.toBeNull();
      if (approveBox) {
        // Button must sit fully within the viewport horizontally.
        expect(approveBox.x).toBeGreaterThanOrEqual(0);
        expect(approveBox.x + approveBox.width).toBeLessThanOrEqual(vp.width);
      }
    });
  });
}
