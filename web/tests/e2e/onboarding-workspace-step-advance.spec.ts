/**
 * Workspace step — manual advance after a fresh provision.
 *
 * Source: `fix-onboarding-workspace-step-celebration-hold-timer-jarring`
 * (onboarding F08, UX e2e audit 2026-05-28-0252).
 *
 * Before the fix, a fresh successful provision fired a celebration
 * toast and then `setTimeout(router.push, 1200)`-ed the user to
 * `/onboarding/connect-provider`. A user hovering Continue mid-toast
 * could click into the wrong page. This spec asserts:
 *
 * - The page does NOT navigate within ~800ms of a successful submit
 *   (well past the old 1200ms hold-minus-buffer; if a timer slipped
 *   back in, this would catch it).
 * - The Continue button is focused so Enter advances.
 * - Pressing Enter advances to /onboarding/connect-provider.
 *
 * Brief specified path `web/playwright/onboarding/...`; the project's
 * actual playwright testDir is `./tests/e2e`, so the spec lives here
 * (sensible deviation, noted in PR body).
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
  "demo mode short-circuits provision with created=false; this spec needs the gateway-intercept lane",
);

test("fresh provision does not auto-advance; Continue is focused and Enter advances", async ({
  page,
}) => {
  // Stub the workspace-step gateway calls so the page renders without
  // a real backend. `created: true` triggers the celebration path.
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
  await page.route("**/api/v1/onboarding/provision", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        tenant_id: "tenant-ws-step",
        workspace_id: "ws-ws-step",
        created: true,
      }),
    }),
  );

  await page.goto(appUrl("/onboarding/workspace"));
  await page.getByTestId("workspace-name").fill("Acme Growth");
  await page.getByTestId("company-name").fill("Acme Inc.");
  await page.getByTestId("onboarding-workspace-submit").click();

  // Window past the deleted 1200ms timer — URL must still be the
  // workspace step. If a setTimeout-driven push reappears this fails.
  await page.waitForTimeout(800);
  await expect(page).toHaveURL(/\/onboarding\/workspace/);

  // Continue button got focus + the manual-advance hint rendered.
  const continueBtn = page.getByTestId("onboarding-workspace-submit");
  await expect(continueBtn).toBeFocused();
  await expect(page.getByTestId("onboarding-workspace-press-enter-hint")).toBeVisible();

  // User presses Enter; advance fires.
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/onboarding\/connect-provider/, { timeout: 5_000 });
});
