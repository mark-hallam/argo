import { expect, test } from "@playwright/test";

/**
 * Dark-mode token-coverage regression — consolidated spec.
 *
 * Renders the marketing/policy public routes that have repeatedly
 * regressed into "white text on white background" when subcomponents
 * hardcoded color literals (e.g. `background: white`, `--bg: #fafaff`)
 * instead of inheriting `var(--background)` / `var(--foreground)`.
 *
 * The production theme system is two-attribute: `next-themes` writes
 * `class="dark"` on `<html>`, then DataThemeBridge mirrors that into
 * `data-theme="skycot-dark"`. The CSS dark block in globals.css fires
 * on `.dark, [data-theme="skycot-dark"]`. We apply both here so this
 * test exercises the same selectors a real user would hit.
 *
 * Routes covered: every public marketing/landing surface that shipped
 * its own page CSS — pricing, compare, docs, and one VerticalLandingShell
 * instance (sales) to cover the shared `/for/*` chrome.
 */

const PAGES = ["/pricing", "/compare", "/docs", "/for/sales"] as const;

test.describe("dark-mode token coverage", () => {
  for (const path of PAGES) {
    test(`${path} renders without white-on-white in dark mode`, async ({ page }) => {
      await page.goto(path);
      await page.waitForLoadState("domcontentloaded");

      const lightBg = await page.evaluate(() =>
        window.getComputedStyle(document.body).backgroundColor,
      );
      expect(lightBg).toBeTruthy();

      // Apply the production dark-mode toggle: next-themes sets class="dark"
      // on <html>, DataThemeBridge mirrors that into data-theme="skycot-dark".
      // CSS in globals.css matches on `.dark, [data-theme="skycot-dark"]`.
      await page.evaluate(() => {
        document.documentElement.classList.add("dark");
        document.documentElement.dataset.theme = "skycot-dark";
      });

      const darkBg = await page.evaluate(() =>
        window.getComputedStyle(document.body).backgroundColor,
      );
      expect(darkBg).not.toBe(lightBg);

      // Broad regression check: in dark mode no more than a small handful
      // of elements should retain a hard `rgb(255, 255, 255)` background
      // (a few are legitimate — gradient-band CTA buttons that sit on the
      // always-purple hero, hairline dividers). The pre-fix failure mode
      // produced 100+ of these.
      const whiteBackedCount = await page.evaluate(() => {
        const els = document.querySelectorAll("*");
        let count = 0;
        for (const el of els) {
          const bg = getComputedStyle(el).backgroundColor;
          if (bg === "rgb(255, 255, 255)") count += 1;
        }
        return count;
      });
      expect(whiteBackedCount).toBeLessThan(20);
    });
  }
});
