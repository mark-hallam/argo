import { expect, test, type Page } from "@playwright/test";

const DEMO_EMAIL = "demo@skycot.com";
const DEMO_PRINCIPAL = "human:demo@skycot.com";
const DEMO_SESSION_TOKEN = "demo.header.payload";
const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_ARGO_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_ARGO_DEMO_MODE=1 so /auth/signup renders the demo-mint lane",
);

test("demo mint signup lands on onboarding welcome without flashing team", async ({
  page,
}) => {
  const pathnames = trackMainFramePathnames(page);
  await installDemoMintAuth(page);

  await page.goto(appUrl("/auth/signup"));
  await expect(page.getByTestId("demo-mode-banner")).toBeVisible();
  await expect(page.getByTestId("signup-submit")).toContainText(
    "Enter demo workspace",
  );

  await page.getByTestId("signup-email").fill(DEMO_EMAIL);
  await Promise.all([
    page.waitForURL(appUrl("/onboarding/welcome"), { timeout: 10_000 }),
    page.getByTestId("signup-submit").click(),
  ]);

  const stepper = page.getByRole("progressbar", { name: /step 1 of 5/i });
  await expect(stepper).toBeVisible();
  await expect(stepper).toContainText(/STEP\s*01\s*OF\s*05\s*WELCOME/);
  await expect(page.getByTestId("onboarding-welcome")).toBeVisible();

  expect(pathnames, `visited pathnames: ${pathnames.join(" -> ")}`).not.toContain(
    "/team",
  );
});

function trackMainFramePathnames(page: Page): string[] {
  const pathnames: string[] = [];
  page.on("framenavigated", (frame) => {
    if (frame !== page.mainFrame()) return;
    try {
      pathnames.push(new URL(frame.url()).pathname);
    } catch {
      // Ignore about:blank and other non-URL frame states.
    }
  });
  return pathnames;
}

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}

async function installDemoMintAuth(page: Page): Promise<void> {
  await page.route("**/api/auth/demo-mint", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        ok: true,
        principal: DEMO_PRINCIPAL,
        session_token: DEMO_SESSION_TOKEN,
        needs_onboarding: true,
      }),
    }),
  );

  await page.route("**/api/auth/csrf", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ csrfToken: "csrf-demo-mint" }),
    }),
  );

  await page.route("**/api/auth/providers", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        "gateway-bearer": {
          id: "gateway-bearer",
          name: "Gateway bearer",
          type: "credentials",
          signinUrl: appUrl("/api/auth/signin/gateway-bearer"),
          callbackUrl: appUrl("/api/auth/callback/gateway-bearer"),
        },
      }),
    }),
  );

  await page.route("**/api/auth/callback/gateway-bearer**", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        ok: true,
        status: 200,
        error: null,
        code: null,
        url: appUrl("/auth/signup"),
      }),
    }),
  );

  await page.route("**/api/auth/session", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        user: { email: DEMO_EMAIL },
        gatewayToken: DEMO_SESSION_TOKEN,
        principal: DEMO_PRINCIPAL,
        expires: "2099-01-01T00:00:00.000Z",
      }),
    }),
  );
}
