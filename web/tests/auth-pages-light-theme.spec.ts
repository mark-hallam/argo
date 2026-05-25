import { expect, test, type Page, type TestInfo } from "@playwright/test";

const AUTH_ROUTES = ["/auth/signup", "/auth/signin", "/auth/verify"] as const;

const DARK_THEME_BOOTSTRAP = () => {
  window.localStorage.setItem("theme", "dark");
  document.documentElement.classList.add("dark");
  document.documentElement.setAttribute("data-theme", "skycot-dark");
};

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

async function htmlThemeState(page: Page) {
  return page.evaluate(() => ({
    className: document.documentElement.className,
    dataTheme: document.documentElement.getAttribute("data-theme"),
    colorScheme: document.documentElement.style.colorScheme,
  }));
}

test.describe("auth pages light theme lock", () => {
  test.use({ colorScheme: "dark", viewport: { width: 1440, height: 900 } });

  for (const route of AUTH_ROUTES) {
    test(`${route} renders the locked light editorial shell`, async ({
      page,
    }, testInfo) => {
      const hydrationMessages: string[] = [];
      page.on("console", (message) => {
        const text = message.text();
        if (/hydrat(?:ed|ion)|did not match|server rendered html/i.test(text)) {
          hydrationMessages.push(text);
        }
      });

      await page.addInitScript(DARK_THEME_BOOTSTRAP);
      await page.goto(appUrl(testInfo, route), { waitUntil: "domcontentloaded" });

      const shell = page.getByTestId("auth-editorial");
      await expect(shell).toBeVisible();

      await expect
        .poll(async () => shell.evaluate((node) => getComputedStyle(node).backgroundColor))
        .toBe("rgb(250, 250, 255)");

      await expect.poll(() => htmlThemeState(page)).toMatchObject({
        dataTheme: "skycot-light",
        colorScheme: "light",
      });

      const state = await htmlThemeState(page);
      expect(state.className).toContain("light");
      expect(state.className).not.toContain("dark");

      await expect
        .poll(async () =>
          shell.evaluate((node) => {
            const style = getComputedStyle(node);
            const hero = node.querySelector(".hero");
            const sub = node.querySelector(".sub");
            const heroStyle = hero ? getComputedStyle(hero) : null;
            const subStyle = sub ? getComputedStyle(sub) : null;
            return {
              dataTheme: node.getAttribute("data-theme"),
              backgroundColor: style.backgroundColor,
              color: style.color,
              heroFontFamily: heroStyle?.fontFamily,
              subFontWeight: subStyle?.fontWeight,
            };
          }),
        )
        .toMatchObject({
          dataTheme: "skycot-light",
          backgroundColor: "rgb(250, 250, 255)",
          color: "rgb(30, 27, 75)",
          subFontWeight: "300",
        });

      const typography = await shell.evaluate((node) => {
        const hero = node.querySelector(".hero");
        const heroStyle = hero ? getComputedStyle(hero) : null;
        return {
          heroFontFamily: heroStyle?.fontFamily,
          heroLetterSpacing: heroStyle?.letterSpacing,
        };
      });

      expect(typography.heroFontFamily).toMatch(/Audiowide|__Audiowide/);
      expect(["0px", "normal"]).toContain(typography.heroLetterSpacing);
      expect(hydrationMessages).toEqual([]);
    });
  }
});
