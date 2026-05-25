import { expect, test, type TestInfo } from "@playwright/test";

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

test.describe("onboarding keyboard affordances", () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test("welcome CTA is focused on load and Enter advances", async ({ page }, testInfo) => {
    await page.goto(appUrl(testInfo, "/onboarding/welcome"), { waitUntil: "domcontentloaded" });

    const cta = page.getByTestId("onboarding-welcome-continue");
    await expect(cta).toBeVisible();
    await expect(cta).toBeFocused();
    await expect
      .poll(() =>
        page.evaluate(() => document.activeElement?.textContent?.replace(/\s+/g, " ").trim()),
      )
      .toContain("Get started");

    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/onboarding\/workspace/);
  });

  test("pick-vertical Back CTA starts inside the 1280x800 viewport", async ({
    page,
  }, testInfo) => {
    await page.goto(appUrl(testInfo, "/onboarding/pick-vertical"), {
      waitUntil: "domcontentloaded",
    });

    const back = page.getByTestId("onboarding-vertical-back");
    await expect(back).toBeVisible();
    await expect(back).toBeInViewport({ ratio: 1 });
  });
});
