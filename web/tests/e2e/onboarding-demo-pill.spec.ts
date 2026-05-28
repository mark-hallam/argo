/**
 * Smoke test for `fix-onboarding-shell-demo-mode-pill-affordance` F18 [P1].
 *
 * Asserts the bottom-right "Demo mode" pill mounts inside the wizard
 * shell when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` and is absent when the
 * env var is unset. The pill is operator-side tooling so the
 * pre-record dress rehearsal has a visible cue confirming demo mode
 * is active during Scene 3 of the v1.0 demo MP4.
 *
 * Path note: the brief originally referenced `web/playwright/onboarding/
 * demo-pill.spec.ts`, but `playwright.config.ts` discovers specs under
 * `./tests/e2e` (every other onboarding spec lives there), so this file
 * is in the conventional location to actually run.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.describe("onboarding demo-mode pill", () => {
  test.skip(
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
    "demo-mode lane: pill only renders when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1",
  );

  test("renders bottom-right on every wizard step when demo mode is set", async ({
    page,
  }) => {
    for (const path of [
      "/onboarding/welcome",
      "/onboarding/workspace",
      "/onboarding/connect-provider",
      "/onboarding/pick-vertical",
      "/onboarding/first-hire",
    ]) {
      await page.goto(new URL(path, SPA_ORIGIN).toString());
      const pill = page.getByTestId("onboarding-demo-pill");
      await expect(pill, `pill missing on ${path}`).toBeVisible();
      await expect(pill).toContainText(/demo mode/i);
      await expect(pill).toContainText(/sample data/i);

      // Anchored to the viewport's bottom-right so it does not collide
      // with the F1 editorial-center hero or CTAs.
      const box = await pill.boundingBox();
      const viewport = page.viewportSize();
      expect(box, "pill has no bounding box").not.toBeNull();
      expect(viewport, "viewport size unknown").not.toBeNull();
      if (box && viewport) {
        expect(
          box.x + box.width,
          "pill right edge should be near the viewport right edge",
        ).toBeGreaterThan(viewport.width - 64);
        expect(
          box.y + box.height,
          "pill bottom edge should be near the viewport bottom edge",
        ).toBeGreaterThan(viewport.height - 64);
      }
    }
  });
});

test.describe("onboarding demo-mode pill (real mode)", () => {
  test.skip(
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1",
    "real-mode lane: only runs when NEXT_PUBLIC_SKYCOT_DEMO_MODE is unset",
  );

  test("is absent on the welcome step when demo mode is off", async ({ page }) => {
    await page.goto(new URL("/onboarding/welcome", SPA_ORIGIN).toString());
    await expect(page.getByTestId("onboarding-demo-pill")).toHaveCount(0);
  });
});
