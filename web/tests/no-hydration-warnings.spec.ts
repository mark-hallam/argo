import { expect, test } from "@playwright/test";

const APP_SHELL_ROUTES = [
  "/",
  "/approvals",
  "/audit",
  "/team",
  "/workers",
  "/workflows",
  "/settings",
  "/settings/members",
] as const;

const HYDRATION_WARNING = /hydrat(?:ed|ion)|did not match|server rendered html|text content does not match/i;

test.describe("app shell hydration", () => {
  test.use({ colorScheme: "dark", viewport: { width: 1440, height: 900 } });

  for (const route of APP_SHELL_ROUTES) {
    test(`does not log hydration warnings on ${route}`, async ({ page }, testInfo) => {
      const baseURL =
        testInfo.project.use.baseURL ??
        process.env.ARGO_E2E_SPA_URL ??
        process.env.E2E_SPA_URL ??
        "http://localhost:3000";
      const hydrationMessages: string[] = [];
      page.on("console", (message) => {
        const text = message.text();
        if (HYDRATION_WARNING.test(text)) hydrationMessages.push(text);
      });

      await page.goto(new URL(route, baseURL).toString(), { waitUntil: "networkidle" });
      await page.waitForTimeout(250);

      expect(hydrationMessages).toEqual([]);
    });
  }
});
