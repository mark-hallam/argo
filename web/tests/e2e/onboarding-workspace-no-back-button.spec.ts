/**
 * Workspace step — no back button.
 *
 * Source: `fix-onboarding-wizard-back-button-step-2-jump-welcome`
 * (onboarding F8, UX audit 2026-05-26-2007).
 *
 * Step 2 (`/onboarding/workspace`) is the first substantive form in
 * the wizard. The pre-fix back-arrow routed to `/onboarding/welcome`
 * — the marketing landing — which erased the typed workspace + company
 * name and dumped the operator on a sales pitch. There is no
 * meaningful in-form "previous" state to navigate to, so the back
 * button is removed per the brief's Option A.
 *
 * Brief specified path `web/playwright/onboarding/...`; the project's
 * actual playwright testDir is `./tests/e2e`, so the spec lives here
 * (matches the sibling `onboarding-workspace-step-advance.spec.ts`
 * sensible deviation).
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1",
  "demo mode short-circuits the gateway; this spec exercises the route-intercept lane",
);

test("workspace step has no back-to-welcome button", async ({ page }) => {
  // Stub the workspace-step gateway calls so the page renders without
  // a real backend.
  await page.route("**/api/auth/session", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        gatewayToken: "stub",
        principal: "human:wsstep@example.test",
      }),
    }),
  );
  await page.route("**/api/v1/auth/csrf", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ token: "csrf-stub" }),
    }),
  );

  await page.goto(appUrl("/onboarding/workspace"));

  // Continue is present and primary.
  await expect(page.getByTestId("onboarding-workspace-submit")).toBeVisible();

  // Back button must NOT exist. Regression guard: if the back-arrow
  // returns it should re-trigger the F8 audit finding.
  await expect(page.getByTestId("onboarding-workspace-back")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^←\s*Back$/ })).toHaveCount(0);

  // Typing into the form and then a stray click-around must not be
  // able to navigate to the marketing /welcome page from this step.
  await page.getByTestId("workspace-name").fill("Acme Growth");
  await page.getByTestId("company-name").fill("Acme Inc.");
  await page.waitForTimeout(200);
  await expect(page).not.toHaveURL(/\/onboarding\/welcome/);
});
