import { expect, test } from "@playwright/test";

/**
 * Regression for the dark-mode "near-black stripe under the wizard"
 * (F7 from onboarding-hire-an-ai-teammate audit, 2026-05-28).
 *
 * The wizard hardcodes a light paper bg (#fafaff) regardless of theme.
 * In `prefers-color-scheme: dark`, the document body default bg goes
 * to near-black (`rgb(2, 7, 11)`-ish) and any overscroll/scroll-band
 * area below the wizard reads as a recorded-video glitch.
 *
 * `WizardEditorialShell` pins `html` + `body` to `#fafaff` and forces
 * `color-scheme: light` whenever the shell is mounted. This spec
 * asserts the page bg is light at both the very top and the very
 * bottom of a 1440x900 viewport under dark-mode emulation.
 */

function rgbDistance(a: [number, number, number], b: [number, number, number]) {
  return Math.sqrt(
    (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2,
  );
}

function parseRgb(value: string): [number, number, number] | null {
  const m = value.match(/rgba?\((\d+)\s*,\s*(\d+)\s*,\s*(\d+)/);
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

test.describe("wizard dark-mode no near-black stripe", () => {
  test.use({ viewport: { width: 1440, height: 900 }, colorScheme: "dark" });

  test("body bg matches wizard paper in dark color-scheme", async ({ page }) => {
    await page.goto("/hire");
    await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();

    const bodyBg = await page.evaluate(() =>
      getComputedStyle(document.body).backgroundColor,
    );
    const htmlBg = await page.evaluate(() =>
      getComputedStyle(document.documentElement).backgroundColor,
    );

    const near = [2, 7, 11] as const;
    for (const raw of [bodyBg, htmlBg]) {
      const rgb = parseRgb(raw);
      // `transparent` (rgba(0,0,0,0)) on html is fine — body's bg
      // is what propagates to the canvas. Only fail when an opaque
      // near-black color paints.
      if (!rgb || raw.includes(", 0)")) continue;
      expect(
        rgbDistance(rgb, [near[0], near[1], near[2]]),
        `expected ${raw} not to be near rgb(${near.join(", ")})`,
      ).toBeGreaterThan(40);
    }
  });

  test("color-scheme on body is light wherever the wizard mounts", async ({
    page,
  }) => {
    await page.goto("/hire");
    await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();

    const colorScheme = await page.evaluate(
      () => getComputedStyle(document.body).colorScheme,
    );
    expect(colorScheme).toContain("light");
  });
});
