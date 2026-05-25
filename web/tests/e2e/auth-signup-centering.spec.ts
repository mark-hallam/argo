import { expect, test } from "@playwright/test";

async function stageMetrics(page: import("@playwright/test").Page) {
  const box = await page.getByTestId("auth-stage").boundingBox();
  expect(box).not.toBeNull();
  return box!;
}

async function layoutCenterX(page: import("@playwright/test").Page) {
  return page.evaluate(() => document.documentElement.clientWidth / 2);
}

test.describe("/auth/signup editorial centering", () => {
  test("keeps the signup stage centered at desktop, tablet, and mobile widths", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1600, height: 900 });
    await page.goto("/auth/signup");
    await expect(page.getByRole("heading", { name: /hire your first ai teammate/i }))
      .toBeVisible();

    const desktop = await stageMetrics(page);
    expect(Math.round(desktop.width)).toBe(720);
    expect(Math.abs(desktop.x + desktop.width / 2 - (await layoutCenterX(page))))
      .toBeLessThanOrEqual(1);

    const form = await page.getByTestId("signup-form").boundingBox();
    expect(form).not.toBeNull();
    expect(Math.round(form!.width)).toBeLessThanOrEqual(460);

    await page.setViewportSize({ width: 768, height: 900 });
    const tablet = await stageMetrics(page);
    expect(Math.round(tablet.width)).toBe(720);
    expect(Math.abs(tablet.x + tablet.width / 2 - (await layoutCenterX(page))))
      .toBeLessThanOrEqual(1);

    await page.setViewportSize({ width: 375, height: 812 });
    const mobile = await stageMetrics(page);
    expect(Math.round(mobile.width)).toBe(375);
    expect(Math.abs(mobile.x + mobile.width / 2 - (await layoutCenterX(page))))
      .toBeLessThanOrEqual(1);
  });
});
