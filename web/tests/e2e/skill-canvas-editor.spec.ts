import { expect, test } from "@playwright/test";

test("skill canvas edit mode can add and save under mock fallback", async ({ page }) => {
  await page.goto("/workers/wkr_01J0PRIYA/skills/skill_screen_applications/canvas");

  await page.getByTestId("mode-edit").click();
  await expect(page.getByTestId("skill-canvas-edit-mode")).toBeVisible();

  await page.getByTestId("palette-transform").click();
  await expect(page.getByTestId("edit-node-n_transform_10")).toBeVisible();

  await page.getByTestId("delete-node-n_transform_10").click();
  await expect(page.getByTestId("edit-node-n_transform_10")).toHaveCount(0);

  await page.getByTestId("policy-n_fetch_application").selectOption("ask");
  await page.getByTestId("save-ir").click();
  await expect(page.getByTestId("ab-scheduled")).toBeVisible();
});
