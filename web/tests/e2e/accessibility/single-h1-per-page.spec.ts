import { expect, test } from "@playwright/test";

/**
 * Heading-landmark regression — every in-app surface must render
 * exactly one `<h1>`. Multiple H1s split the document's top-level
 * landmark and confuse screen-reader tab order (WCAG 2.4.10 / AXE
 * `page-has-heading-one`).
 *
 * The brand-loud hero (`<BrandLoudHero>`) IS the page H1 on /team,
 * /cohorts, and /approvals; sticky-bar / sub-shell titles below it
 * must be H2. The original drift on /team was a sticky-bar "Team"
 * `<h1>` rendered alongside the hero `<h1>` — the fix in this brief
 * demotes the sticky bar.
 *
 * `/workers/<id>/overview` is intentionally omitted: the page
 * currently has zero visible H1s (a separate accessibility gap
 * tracked outside this brief's may-touch).
 */
const PAGES_WITH_SINGLE_H1: ReadonlyArray<{ path: string; note: string }> = [
  { path: "/team", note: "brand-loud hero is the H1; sticky bar demoted to H2" },
  { path: "/skills", note: "skills marketplace hero is the H1" },
  { path: "/workflows", note: "workflows hero is the H1" },
  { path: "/cohorts", note: "brand-loud hero is the H1" },
  { path: "/audit", note: "audit page header is the H1" },
  { path: "/approvals", note: "brand-loud hero is the H1" },
];

for (const { path, note } of PAGES_WITH_SINGLE_H1) {
  test(`${path} renders exactly one <h1> (${note})`, async ({ page }) => {
    await page.goto(path);
    // Suspense fallbacks on /team render an interim H1 too — wait for
    // the loaded H1 to be visible so the count below sees the
    // post-load DOM, not the fallback DOM.
    await expect(page.locator("h1").first()).toBeVisible();
    const h1Count = await page.locator("h1").count();
    expect(h1Count, `${path} should have exactly one <h1>`).toBe(1);
  });
}
