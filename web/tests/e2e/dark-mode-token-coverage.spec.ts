import { expect, test } from "@playwright/test";

/**
 * Dark-mode token-coverage visual regression.
 *
 * Renders the highest-impact public routes that PR-B (the per-callsite
 * token sweep) migrated, toggles `data-theme="dark"` on `<html>`, and
 * asserts the page doesn't fall into the broken "white text on white
 * background" failure mode caught by the wave-9 dark-mode audit.
 *
 * Routes covered:
 *   /pricing  — pricing page (PR-B file 10)
 *   /compare  — competitor comparison (PR-B file 11)
 *   /docs     — public docs landing (PR-B file 12)
 *   /for/marketplace — vertical landing (uses VerticalLandingShell, PR-B file 13)
 *
 * The dark-theme block in `globals.css` already defines every token
 * (background, foreground, card, primary, gradient-end, …). The
 * regression check counts elements whose computed `background-color`
 * resolves to pure `rgb(255, 255, 255)` after the theme flip — a
 * handful are legitimate (logos, hairlines), but the broken-page
 * threshold (>20 elements) catches inline `#fafaff` / `#ffffff` that
 * didn't migrate to tokens.
 *
 * Skipped on the deployed lane: this spec asserts a local computed
 * style transition, not deployed-real network behavior, so it lives
 * outside the `deployed/` directory and only runs in the local lane.
 */

const PAGES = ["/pricing", "/compare", "/docs", "/for/marketplace"] as const;

test.describe("dark-mode token coverage", () => {
  for (const path of PAGES) {
    test(`${path} renders without white-on-white in dark mode`, async ({ page }) => {
      await page.goto(path);
      await page.waitForLoadState("domcontentloaded");

      // Snapshot a representative element's computed background in light mode.
      const lightBg = await page.evaluate(() =>
        window.getComputedStyle(document.body).backgroundColor,
      );
      expect(lightBg).toBeTruthy();

      // Flip the theme. The dark-theme block in globals.css overrides every
      // semantic token; any element using `var(--background)` etc. should
      // automatically pick up the dark value.
      await page.evaluate(() => {
        document.documentElement.dataset.theme = "dark";
      });

      // Body background must shift away from light. The exact value is
      // implementation-defined (oklch resolves to an rgb close to #14141e
      // on light-mode-default browsers), so we only assert it differs.
      const darkBg = await page.evaluate(() =>
        window.getComputedStyle(document.body).backgroundColor,
      );
      expect(darkBg).not.toBe(lightBg);

      // Broad regression check: in dark mode, no more than a small handful
      // of elements should retain a hard `rgb(255, 255, 255)` background
      // (we expect a few — logos, hairline dividers, badges). The wave-9
      // audit's failure mode produced 100+ of these.
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
