/**
 * Regression guard for `fix-demo-banner-sidebar-overlap-desktop`.
 *
 * Before the fix the `<DemoBanner />` sat at the root of `<AppShell>`,
 * spanning the entire viewport including the area under the 256px
 * fixed-position sidebar. The banner's inner `max-w-5xl mx-auto`
 * container centered against x=0, so on desktop the leading word
 * "Demo " of "Demo mode — showing sample data" hid behind the SKYCOT
 * brand block in the sidebar — visible on the v1.0 demo recording.
 *
 * Fix: move `<DemoBanner />` inside `<main>` so it inherits the
 * `md:pl-64` content-column offset. Mobile (sidebar collapsed) is
 * unaffected because `md:pl-64` only applies at >=768px.
 *
 * Requires the dev server to be running with
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` so the banner actually renders
 * (`DemoBanner` no-ops in non-demo mode). The spec `test.skip`s itself
 * when the var isn't set, matching the pattern in
 * `onboarding-demo-mode-step-2-continue.spec.ts`.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

const SIDEBAR_WIDTH_PX = 256;

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the DemoBanner renders",
);

test.describe("DemoBanner sidebar overlap regression", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("banner sits inside the main content column at desktop width", async ({ page }) => {
    await page.addInitScript(() => {
      try {
        window.localStorage.removeItem("skycot:demo-banner-dismissed");
      } catch {}
    });

    await page.goto(new URL("/audit", SPA_ORIGIN).toString());

    const banner = page.getByTestId("demo-banner");
    await expect(banner).toBeVisible();

    // Structural assertion: banner is a descendant of <main>, so it
    // inherits the `md:pl-64` content-column offset.
    const insideMain = await banner.evaluate(
      (node) => !!node.closest("main#main"),
    );
    expect(insideMain, "DemoBanner must render inside <main id=\"main\">").toBe(true);

    // Visual assertion: banner box starts at the content-column edge,
    // not at x=0 under the sidebar.
    const box = await banner.boundingBox();
    expect(box, "banner has no bounding box").not.toBeNull();
    if (box) {
      expect(
        box.x,
        `banner.x=${box.x} should be >= sidebar width ${SIDEBAR_WIDTH_PX}`,
      ).toBeGreaterThanOrEqual(SIDEBAR_WIDTH_PX);
    }

    // Copy assertion: the leading "Demo mode —" prefix renders without
    // being clipped behind the sidebar.
    await expect(banner).toContainText("Demo mode");
    await expect(banner).toContainText("showing sample data");
  });

  test("dismiss button still hides the banner and persists via localStorage", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      try {
        window.localStorage.removeItem("skycot:demo-banner-dismissed");
      } catch {}
    });

    await page.goto(new URL("/audit", SPA_ORIGIN).toString());
    const banner = page.getByTestId("demo-banner");
    await expect(banner).toBeVisible();

    await page.getByTestId("demo-banner-dismiss").click();
    await expect(banner).toHaveCount(0);

    const stored = await page.evaluate(() =>
      window.localStorage.getItem("skycot:demo-banner-dismissed"),
    );
    expect(stored).toBe("1");
  });
});

test.describe("DemoBanner on mobile viewport", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("banner spans full width when sidebar is hidden", async ({ page }) => {
    await page.addInitScript(() => {
      try {
        window.localStorage.removeItem("skycot:demo-banner-dismissed");
      } catch {}
    });

    await page.goto(new URL("/audit", SPA_ORIGIN).toString());

    const banner = page.getByTestId("demo-banner");
    await expect(banner).toBeVisible();

    // On mobile the sidebar is a bottom-sheet (not a fixed left rail),
    // so the banner — though still inside <main className="md:pl-64"> —
    // gets no left-padding from the `md:` breakpoint and starts at x=0.
    const box = await banner.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.x).toBeLessThan(10);
    }
  });
});
