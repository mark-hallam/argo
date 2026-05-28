import { expect, test, type Page } from "@playwright/test";

/**
 * Whole-site F07 fix verification — when the in-app theme preference
 * is "dark", marketing pages (/landing, /pricing, /customers) should
 * see `html.dark` set on first paint instead of the marketing-default
 * light. The bridge is the `skycot-color-mode` cookie that
 * ThemeToggle writes alongside next-themes' localStorage.
 *
 * The toggle itself is asserted at the unit-test level in
 * web/components/marketing/__tests__/MarketingNav.test.tsx; this spec
 * exercises the SSR side: a request carrying the cookie must render
 * `<html class="dark ...">` directly, with no light→dark flicker.
 */

function spaUrl(testInfo: import("@playwright/test").TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

async function htmlClass(page: Page): Promise<string> {
  return page.evaluate(() => document.documentElement.className);
}

test.describe("marketing theme cross-section (whole-site F07)", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("with cookie=dark, /landing SSR renders html.dark on first paint", async ({
    context,
    page,
  }, testInfo) => {
    const base = new URL(spaUrl(testInfo, "/landing"));
    await context.addCookies([
      {
        name: "skycot-color-mode",
        value: "dark",
        domain: base.hostname,
        path: "/",
        sameSite: "Lax",
      },
    ]);

    await page.goto(base.toString(), { waitUntil: "domcontentloaded" });

    const cls = await htmlClass(page);
    expect(cls).toContain("dark");
    expect(cls).not.toMatch(/\blight\b/);
  });

  test("with no cookie, /landing first paint stays on the marketing-default (no dark class)", async ({
    page,
  }, testInfo) => {
    await page.goto(spaUrl(testInfo, "/landing"), { waitUntil: "domcontentloaded" });
    const cls = await htmlClass(page);
    expect(cls).not.toMatch(/\bdark\b/);
  });

  test("toggling theme in-app writes a cookie that survives /landing navigation", async ({
    page,
  }, testInfo) => {
    // Start in-app, flip to dark via the AppShell toggle, then navigate to
    // /landing and confirm the SSR class on /landing also has `dark`.
    await page.goto(spaUrl(testInfo, "/approvals"), { waitUntil: "domcontentloaded" });
    const toggle = page.getByTestId("appshell-theme-toggle").first();
    await toggle.click();
    await expect
      .poll(() => page.evaluate(() => document.cookie))
      .toMatch(/skycot-color-mode=dark/);

    await page.goto(spaUrl(testInfo, "/landing"), { waitUntil: "domcontentloaded" });
    const cls = await htmlClass(page);
    expect(cls).toContain("dark");
  });
});
