import { expect, test, type Page } from "@playwright/test";

/**
 * Regression for the F5 [P0] demo-blocker: the "Or try a demo workspace
 * · no email required" secondary CTA on `/auth/signup` used to be
 * `disabled` until the email field was non-empty, AND its handler
 * short-circuited with "Enter an email to continue." when the field was
 * blank. Both contradicted the visible copy.
 *
 * The fix mints a synthetic `demo+<ts>@skycot.test` address when the
 * field is blank and drops the `disabled` gate on the CTA. This spec
 * proves both: the button is clickable cold, and a blank-field click
 * lands the user on `/onboarding/welcome` with no error banner.
 */

const DEMO_PRINCIPAL = "human:demo+stub@skycot.test";
const DEMO_SESSION_TOKEN = "demo.header.payload";
const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1",
  "the secondary CTA only renders when NEXT_PUBLIC_ARGO_DEMO_MODE is unset",
);

test("demo CTA with blank email lands on onboarding welcome with no error", async ({
  page,
}) => {
  const sentEmails = await installDemoMintAuth(page);

  await page.goto(appUrl("/auth/signup"));

  const demoButton = page.getByTestId("signup-demo-mint");
  await expect(demoButton).toBeVisible();
  await expect(demoButton).toBeEnabled();
  await expect(demoButton).toContainText(
    "Or try a demo workspace · no email required",
  );

  await Promise.all([
    page.waitForURL(appUrl("/onboarding/welcome"), { timeout: 10_000 }),
    demoButton.click(),
  ]);

  await expect(page.locator("#signup-error")).toHaveCount(0);
  expect(sentEmails.length).toBeGreaterThan(0);
  expect(sentEmails[0]).toMatch(/^demo\+\d+@skycot\.test$/);
});

test("typed email is preserved through the demo CTA path", async ({ page }) => {
  const sentEmails = await installDemoMintAuth(page);

  await page.goto(appUrl("/auth/signup"));
  await page.getByTestId("signup-email").fill("real.user@example.com");

  await Promise.all([
    page.waitForURL(appUrl("/onboarding/welcome"), { timeout: 10_000 }),
    page.getByTestId("signup-demo-mint").click(),
  ]);

  expect(sentEmails).toContain("real.user@example.com");
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}

async function installDemoMintAuth(page: Page): Promise<string[]> {
  const sentEmails: string[] = [];
  await page.route("**/api/auth/demo-mint", async (route) => {
    try {
      const body = JSON.parse(route.request().postData() ?? "{}") as {
        email?: unknown;
      };
      if (typeof body.email === "string") sentEmails.push(body.email);
    } catch {
      // Ignore malformed JSON; assertion on sentEmails length will catch it.
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        ok: true,
        principal: DEMO_PRINCIPAL,
        session_token: DEMO_SESSION_TOKEN,
        needs_onboarding: true,
      }),
    });
  });

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
        user: { email: "demo@skycot.test" },
        gatewayToken: DEMO_SESSION_TOKEN,
        principal: DEMO_PRINCIPAL,
        expires: "2099-01-01T00:00:00.000Z",
      }),
    }),
  );

  return sentEmails;
}
