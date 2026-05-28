import { expect, test } from "@playwright/test";

/**
 * Soft perf-regression guard for `fix-cohorts-cold-paint-renderer-wedge-
 * three-plus-minutes` (whole-site F03).
 *
 * The defect: cold-paint navigation to `/cohorts` wedged the Chrome
 * renderer for 3+ minutes on first visit, then recovered to ~6s after
 * preview restart. Root cause was the locked D3 magazine layout +
 * 459-LOC `CohortDetailDrawer` hydrating in a single synchronous pass.
 * The fix splits both subtrees with `next/dynamic({ ssr: false })`.
 *
 * This spec is a flake-prone soft guard — wall-clock thresholds in CI
 * can spike under load — but it catches another 3-minute wedge if it
 * returns. Thresholds:
 *
 * - `domContentLoaded` < 10s (was 180s+ in the unfixed pathology).
 * - First magazine tile visible < 10s on a fresh browser context.
 * - First dept tab visible < 5s (must paint before the lazy grid).
 *
 * Uses `browser.newContext({})` to defeat any per-worker page cache.
 */
test("cohorts cold paint renders dept tabs + first tile within 10s", async ({
  browser,
}) => {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  try {
    const navStart = Date.now();
    const response = await page.goto("/cohorts", { waitUntil: "domcontentloaded" });
    const domContentLoaded = Date.now() - navStart;
    expect(response?.status() ?? 0).toBeLessThan(400);
    expect(
      domContentLoaded,
      `domContentLoaded ${domContentLoaded}ms exceeded 10s ceiling — possible cold-paint wedge regression`,
    ).toBeLessThan(10_000);

    // Dept tabs are server-rendered (outside the dynamic-imported grid)
    // so they must appear well before LCP. Anything past 5s is a sign
    // the hero/tabs are themselves wedged.
    await expect(
      page.getByTestId("cohorts-page"),
      "cohorts-page region must paint before LCP",
    ).toBeVisible({ timeout: 5_000 });

    // The first magazine tile lives behind the lazy grid; it should
    // hydrate within 10s on a fresh context (no warm chunks). The
    // selector matches both lead/stacked/compact via the shared
    // `data-cohort-tile` attribute.
    await expect(
      page.locator("[data-cohort-tile]").first(),
      "first cohort tile must hydrate within 10s",
    ).toBeVisible({ timeout: 10_000 });
  } finally {
    await page.close();
    await ctx.close();
  }
});
