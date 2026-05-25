import { expect, test } from "@playwright/test";

async function colorMode(page: import("@playwright/test").Page) {
  return page.evaluate(() => ({
    classes: Array.from(document.documentElement.classList),
    dataTheme: document.documentElement.getAttribute("data-theme"),
    storedTheme: window.localStorage.getItem("theme"),
    label: document
      .querySelector('[data-testid="appshell-theme-toggle"]')
      ?.getAttribute("aria-label"),
  }));
}

async function expectColorMode(
  page: import("@playwright/test").Page,
  theme: "light" | "dark",
) {
  await expect
    .poll(async () => {
      const mode = await colorMode(page);
      return (
        mode.classes.includes(theme) &&
        !mode.classes.includes(theme === "dark" ? "light" : "dark") &&
        mode.dataTheme === `skycot-${theme}` &&
        mode.storedTheme === theme &&
        mode.label === `Switch to ${theme === "dark" ? "light" : "dark"} theme`
      );
    })
    .toBe(true);
  const mode = await colorMode(page);
  expect(mode.classes).toContain(theme);
  expect(mode.classes).not.toContain(theme === "dark" ? "light" : "dark");
  expect(mode.dataTheme).toBe(`skycot-${theme}`);
  expect(mode.storedTheme).toBe(theme);
  expect(mode.label).toBe(`Switch to ${theme === "dark" ? "light" : "dark"} theme`);
}

test.describe("/approvals theme toggle", () => {
  test.use({ colorScheme: "dark", viewport: { width: 1440, height: 900 } });

  test("flips html class, storage, aria label, and survives reload", async ({
    page,
  }, testInfo) => {
    const baseURL =
      testInfo.project.use.baseURL ??
      process.env.ARGO_E2E_SPA_URL ??
      process.env.E2E_SPA_URL ??
      "http://localhost:3000";
    await page.goto(new URL("/approvals", baseURL).toString());
    const toggle = page.getByTestId("appshell-theme-toggle");
    await expect(toggle).toHaveAttribute("aria-label", "Switch to light theme");

    await toggle.click();
    await expectColorMode(page, "light");

    await page.reload();
    await expectColorMode(page, "light");

    await toggle.click();
    await expectColorMode(page, "dark");
  });
});
