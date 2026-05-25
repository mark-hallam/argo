import { expect, test } from "@playwright/test";

test("Project & Ops tab shows Peta lead and Patrick stacked tile", async ({
  page,
}, testInfo) => {
  await page.goto("/cohorts");

  const projectOpsTab = page.getByTestId("cohort-dept-tab-project-ops");
  await expect(projectOpsTab).toBeVisible();
  await projectOpsTab.click();

  const projectOps = page.getByTestId("cohort-mag-project-ops");
  await expect(projectOps).toBeVisible();
  const petaTile = page.getByTestId("cohort-card-project-ops::project-coordinator");
  const patrickTile = page.getByTestId("cohort-card-project-ops::status-reporter");
  await expect(petaTile).toHaveAttribute("data-cohort-tile", "lead");
  await expect(patrickTile).toHaveAttribute("data-cohort-tile", "stacked");
  await expect(projectOps).toContainText("Peta");
  await expect(projectOps).toContainText("Patrick");
  await expect(projectOps).toContainText("L3 · Status Reporter");
  await expect(projectOps).toContainText("Linear · GitHub · Slack");

  const png = await projectOps.screenshot({
    path: testInfo.outputPath("cohorts-project-ops-peta-patrick.png"),
  });
  expect(png.length).toBeGreaterThan(10_000);
});
