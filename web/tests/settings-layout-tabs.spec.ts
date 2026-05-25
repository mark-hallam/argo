import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test("settings root renders workspace tab without coming soon badges", async ({
  page,
}) => {
  await page.goto(appUrl("/settings"));

  await expect(page.getByTestId("settings-tabs")).toBeVisible();
  await expect(page.getByTestId("settings-tab-workspace")).toHaveAttribute(
    "aria-current",
    "page",
  );
  await expect(page.getByTestId("settings-workspace-panel")).toBeVisible();
  await expect(page.locator("body")).not.toContainText("Coming soon");
});

test("settings security route uses shared tabs with active security", async ({ page }) => {
  await page.setViewportSize({ width: 1920, height: 1000 });
  await page.goto(appUrl("/settings/security"));

  const tabs = page.getByTestId("settings-tabs");
  const security = page.getByTestId("settings-tab-security");
  const danger = page.getByTestId("settings-danger-tab");

  await expect(tabs).toBeVisible();
  await expect(security).toHaveAttribute("aria-current", "page");
  await expect(danger).toBeVisible();
  await expect(page.locator("body")).not.toContainText("Coming soon");

  await expect(tabs).toHaveCSS("display", "flex");
  await expect
    .poll(
      async () =>
        danger.evaluate((el) => Number.parseFloat(getComputedStyle(el).marginLeft)),
      { message: "Danger zone tab should be pushed to the right edge" },
    )
    .toBeGreaterThan(0);
});

test("settings connectors route uses shared tabs with active connectors", async ({
  page,
}) => {
  await page.goto(appUrl("/settings/connectors"));

  const tabs = page.getByTestId("settings-tabs");
  const connectors = page.getByTestId("settings-tab-connectors");

  await expect(tabs).toBeVisible();
  await expect(connectors).toHaveAttribute("aria-current", "page");
  await expect(page.locator("body")).not.toContainText("Coming soon");
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
