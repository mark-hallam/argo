/**
 * Onboarding wizard top-bar at mobile viewport (375×812).
 *
 * UX-audit F9 [P2] (docs/skycot/ux-audits/2026-05-26-2007/onboarding-
 * hire-an-ai-teammate.md) — at a 375×812 mobile viewport the wizard top
 * bar's "SKYCOT · STEP 01 OF 05 · [5 dots] · WELCOME" row plus the
 * right-side Help + SAVE & EXIT controls overflow the right edge, so a
 * phone visitor on /onboarding/* sees clipped chrome.
 *
 * Fix: WizardEditorialShell adds a < 640px branch that collapses the
 * dot-progress strip to a single "N / 5" pill, hides the Help link
 * (the `/help` route still resolves), and folds SAVE & EXIT into a
 * 3-dot icon button. This spec pins all three behaviours at 375×812.
 *
 * Note on path: the brief asked for the spec at
 * `web/playwright/onboarding/mobile-viewport-top-bar.spec.ts`, but the
 * project's `playwright.config.ts` has `testDir: "./tests/e2e"`, so a
 * spec under `web/playwright/` would never run. Filed under
 * `web/tests/e2e/` so the test runner actually picks it up.
 *
 * Requires the dev server to be running with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` so middleware does not
 * redirect onboarding routes to /auth/signin. The playwright config
 * sets that env var on the spawned `pnpm dev` web server already.
 */

import { expect, test, type TestInfo } from "@playwright/test";

const MOBILE_VIEWPORT = { width: 375, height: 812 };

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

test.use({ viewport: MOBILE_VIEWPORT });

test.describe("onboarding wizard top bar at 375×812 (F9 P2)", () => {
  test("3-dot menu replaces SAVE & EXIT and sits inside the viewport", async ({
    page,
  }, testInfo) => {
    await page.goto(appUrl(testInfo, "/onboarding/welcome"), {
      waitUntil: "domcontentloaded",
    });
    await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();

    const mobileMenu = page.getByTestId("wiz-shell-mobile-menu");
    await expect(mobileMenu).toBeVisible();
    const box = await mobileMenu.boundingBox();
    expect(box).not.toBeNull();
    if (!box) throw new Error("wiz-shell-mobile-menu has no bounding box");
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(MOBILE_VIEWPORT.width);

    // The desktop SAVE & EXIT text button + Help link must collapse.
    await expect(page.getByTestId("wiz-shell-exit")).toBeHidden();
    await expect(page.getByTestId("wiz-shell-help")).toBeHidden();
  });

  test("dot-progress strip collapses to a compact 'N / 5' pill", async ({
    page,
  }, testInfo) => {
    await page.goto(appUrl(testInfo, "/onboarding/welcome"), {
      waitUntil: "domcontentloaded",
    });
    const pill = page.getByTestId("wiz-shell-steps-mobile");
    await expect(pill).toBeVisible();
    await expect(pill).toHaveText(/01\s*\/\s*05/);

    const pillBox = await pill.boundingBox();
    expect(pillBox).not.toBeNull();
    if (!pillBox) throw new Error("wiz-shell-steps-mobile has no bounding box");
    expect(pillBox.x).toBeGreaterThanOrEqual(0);
    expect(pillBox.x + pillBox.width).toBeLessThanOrEqual(MOBILE_VIEWPORT.width);

    // The desktop pip strip is hidden at this viewport.
    await expect(page.getByTestId("wiz-pip-1")).toBeHidden();
  });

  test("page does not overflow horizontally", async ({ page }, testInfo) => {
    await page.goto(appUrl(testInfo, "/onboarding/welcome"), {
      waitUntil: "domcontentloaded",
    });
    await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();
    const scrollWidth = await page.evaluate(
      () => document.documentElement.scrollWidth,
    );
    // A small float tolerance for sub-pixel rounding from scaled chrome.
    expect(scrollWidth).toBeLessThanOrEqual(MOBILE_VIEWPORT.width + 1);
  });
});
