import { expect, test } from "@playwright/test";

test("orchestration queue renders mock runs and filters by worker", async ({ page }) => {
  await page.route("**/api/auth/session", async (route) => {
    await route.fulfill({ json: { gatewayToken: "test-token" } });
  });
  await page.route("http://127.0.0.1:1/api/v1/orchestration/runs/**", async (route) => {
    await route.fulfill({ status: 202, json: { ok: true } });
  });

  await page.goto("/orchestration");

  await expect(page.getByTestId("orchestration-page")).toBeVisible();
  await expect(page.getByTestId("orchestration-table")).toBeVisible();
  const initialRows = await page.getByTestId("orchestration-row").count();
  expect(initialRows).toBeGreaterThanOrEqual(50);

  await page.getByLabel("Worker").selectOption("Maya");
  const filteredRows = await page.getByTestId("orchestration-row").count();
  expect(filteredRows).toBeGreaterThan(0);
  expect(filteredRows).toBeLessThan(initialRows);

  await page.getByLabel("Cancel run").first().click();
  await expect(page.getByText("Run cancelled")).toBeVisible();
});
