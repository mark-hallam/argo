import { expect, test, type Page, type TestInfo } from "@playwright/test";

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

async function isInViewport(page: Page, selector: string) {
  return page.locator(selector).evaluate((node) => {
    const rect = node.getBoundingClientRect();
    return rect.bottom > 0 && rect.top < window.innerHeight;
  });
}

async function cardCenterOverlapArea(page: Page) {
  return page.evaluate(() => {
    const center = document.querySelector(".land-art-center");
    const cards = [...document.querySelectorAll(".land-art-bubble")];
    if (!center || cards.length === 0) return Number.POSITIVE_INFINITY;

    const centerRect = center.getBoundingClientRect();
    return cards.reduce((maxOverlap, card) => {
      const cardRect = card.getBoundingClientRect();
      const width = Math.max(
        0,
        Math.min(centerRect.right, cardRect.right) - Math.max(centerRect.left, cardRect.left),
      );
      const height = Math.max(
        0,
        Math.min(centerRect.bottom, cardRect.bottom) - Math.max(centerRect.top, cardRect.top),
      );
      return Math.max(maxOverlap, width * height);
    }, 0);
  });
}

test.describe("marketing nav anchors and landing hero", () => {
  test("marketing nav anchor links resolve to landing sections from compare", async ({ page }, testInfo) => {
    await page.goto(appUrl(testInfo, "/compare"), { waitUntil: "domcontentloaded" });

    await page.getByRole("link", { name: "How it works" }).click();
    await expect(page).toHaveURL(/\/landing#how$/);
    await expect(page.locator("#how")).toBeVisible();
    await expect.poll(() => isInViewport(page, "#how")).toBe(true);

    await page.goto(appUrl(testInfo, "/customers"), { waitUntil: "domcontentloaded" });
    await page.getByRole("link", { name: "Features" }).click();
    await expect(page).toHaveURL(/\/landing#features$/);
    await expect(page.locator("#features")).toBeVisible();
    await expect.poll(() => isInViewport(page, "#features")).toBe(true);
  });

  test("secondary hero CTA performs a real scroll action", async ({ page }, testInfo) => {
    await page.goto(appUrl(testInfo, "/landing"), { waitUntil: "domcontentloaded" });

    await expect(page.getByRole("link", { name: "Watch a 90-second demo" })).toHaveCount(0);
    await page.getByRole("link", { name: "See how it works" }).click();

    await expect(page).toHaveURL(/\/landing#how$/);
    await expect.poll(() => isInViewport(page, "#how")).toBe(true);
  });

  for (const width of [1280, 1440, 1920, 2560]) {
    test(`hero worker cards do not overlap the SKYCOT logo block at ${width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(appUrl(testInfo, "/landing"), { waitUntil: "domcontentloaded" });

      await expect(page.locator(".land-art-center")).toBeVisible();
      await expect.poll(() => cardCenterOverlapArea(page)).toBe(0);
    });
  }
});
