import { expect, test, type Page, type TestInfo } from "@playwright/test";

const APP_ROUTES = ["/approvals", "/audit", "/team", "/cohorts", "/settings"] as const;
const RAIL_SELECTOR = "#main .max-w-5xl, #main .max-w-6xl, #main .settings-shell";

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

async function widestRailWidth(page: Page) {
  return page.locator("#main").evaluate((main, selector) => {
    const widths = [...main.querySelectorAll(selector)]
      .map((node) => node.getBoundingClientRect().width)
      .filter((width) => width > 0);
    return Math.max(0, ...widths);
  }, RAIL_SELECTOR);
}

test.describe("app shell wide viewport content rail", () => {
  for (const width of [1920, 2560]) {
    test.describe(`${width}px viewport`, () => {
      test.use({ viewport: { width, height: width === 1920 ? 1080 : 1440 } });

      for (const route of APP_ROUTES) {
        test(`${route} expands the app rail`, async ({ page }, testInfo) => {
          await page.goto(appUrl(testInfo, route), { waitUntil: "domcontentloaded" });
          await expect(page.locator("#main")).toBeVisible();

          await expect.poll(() => widestRailWidth(page)).toBeGreaterThanOrEqual(1400);
        });
      }
    });
  }
});
