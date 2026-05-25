import { expect, test } from "@playwright/test";

test("cohorts renders a D3 tile and the create destination is live", async ({
  page,
}) => {
  await page.goto(appUrl("/cohorts"));

  await expect(page.getByTestId("cohorts-page")).toBeVisible();
  await expect(page.locator("[data-cohort-tile]").first()).toBeVisible();

  const cta = page.getByTestId("cohorts-create-cta");
  if ((await cta.count()) > 0) {
    await expect(cta).toHaveAttribute("href", "/cohorts/new");
    await cta.click();
  } else {
    const response = await page.goto(appUrl("/cohorts/new"));
    expect(response?.status()).toBeLessThan(400);
  }

  await expect(page).toHaveURL(/\/cohorts\/new$/);
  await expect(page.getByTestId("cohort-create-page")).toBeVisible();
  await expect(page.getByTestId("cohort-create-form")).toBeVisible();
});

function appUrl(pathname: string): string {
  const base =
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(pathname, base).toString();
}
