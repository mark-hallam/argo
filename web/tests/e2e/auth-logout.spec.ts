import { expect, test } from "@playwright/test";

/**
 * Logout-button wire (Stream `web-auth-logout-button-wire`).
 *
 * The sign-out control lives in the `AppShell` workspace footer
 * popover. Click → `POST /api/v1/auth/logout` → on 2xx clear the
 * NextAuth session and redirect to `/auth/signin`. On 401 (cookie
 * already revoked) we treat it as success. On 5xx we toast an error
 * and keep the user where they are so they can retry.
 *
 * The local Playwright lane points `NEXT_PUBLIC_GATEWAY_URL` at
 * `http://127.0.0.1:1`, so every assertion goes through `page.route`
 * interception — same pattern as `workflow-save-real.spec.ts`.
 */

const LOGOUT_URL = /\/api\/v1\/auth\/logout$/;

async function openUserMenu(page: import("@playwright/test").Page) {
  const trigger = page.getByTestId("appshell-user-menu-trigger");
  await expect(trigger).toBeVisible();
  await trigger.click();
  await expect(page.getByTestId("appshell-sign-out")).toBeVisible();
}

test("happy path: sign out POSTs to gateway and redirects to /auth/signin", async ({
  page,
}) => {
  let logoutHits = 0;
  await page.route(LOGOUT_URL, async (route) => {
    logoutHits += 1;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      headers: {
        "Set-Cookie": "__Host-session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax",
      },
      body: JSON.stringify({ status: "logged_out", had_jti: true }),
    });
  });

  await page.goto("/");
  await openUserMenu(page);
  await page.getByTestId("appshell-sign-out").click();

  await expect(page).toHaveURL(/\/auth\/signin/, { timeout: 5_000 });
  expect(logoutHits).toBe(1);
});

test("401 from gateway (cookie already revoked) is treated as success", async ({
  page,
}) => {
  await page.route(LOGOUT_URL, async (route) => {
    await route.fulfill({
      status: 401,
      contentType: "application/json",
      body: JSON.stringify({ error: "missing_cookie" }),
    });
  });

  await page.goto("/");
  await openUserMenu(page);
  await page.getByTestId("appshell-sign-out").click();

  await expect(page).toHaveURL(/\/auth\/signin/, { timeout: 5_000 });
});

test("5xx surfaces an error toast and keeps the user signed in", async ({
  page,
}) => {
  await page.route(LOGOUT_URL, async (route) => {
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({ error: "boom" }),
    });
  });

  await page.goto("/");
  await openUserMenu(page);
  await page.getByTestId("appshell-sign-out").click();

  // Toast surfaces and we stay on the (app) surface, not /auth/signin.
  await expect(
    page.getByText(/Couldn't sign out — please refresh and try again/i),
  ).toBeVisible({ timeout: 5_000 });
  await expect(page).not.toHaveURL(/\/auth\/signin/);
});
