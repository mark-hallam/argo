import { expect, test } from "@playwright/test";

/**
 * Regression test for fix-compare-page-html-payload-bloat-321kb.
 *
 * The original `/compare` page shipped 321 KB of HTML — over 5× the
 * next-largest marketing page — because the 16×5 comparison matrix
 * rendered as a deep server-component JSX tree, so React's Flight
 * payload re-serialised every score span + table cell on the wire.
 *
 * Fix: the matrix lives in a client component now, so the Flight
 * payload only carries the dimensions array as a JSON prop. SSR'd
 * markup is unchanged.
 *
 * Ceiling chosen: 160 KB. Post-fix the dev-mode payload measures
 * ~71 KB; we set the bar at roughly 2× headroom so legitimate copy
 * additions don't flake, but a regression that re-introduces the
 * per-node Flight bloat will trip it well before 321 KB.
 */
test("/compare HTML payload stays under 160 KB (dev mode)", async ({ request }) => {
  const response = await request.get("/compare");
  expect(response.status()).toBe(200);
  const body = await response.body();
  // 160 KB ceiling. Original regression was 321 KB; the fix lands at
  // ~71 KB. Headroom protects against incremental matrix growth.
  const MAX_BYTES = 160 * 1024;
  expect(body.byteLength).toBeLessThan(MAX_BYTES);
});

test("/compare still renders the 16×5 comparison matrix", async ({ page }) => {
  await page.goto("/compare");
  // 80 = 16 dimensions × 5 competitors. Same as the original page.
  await expect(page.locator(".compare-score")).toHaveCount(80);
  // Header + 5 group rows + 16 question rows + 1 footer = 23.
  await expect(page.locator(".compare-table tr")).toHaveCount(23);
  await expect(page.locator(".compare-cta")).toBeVisible();
});
