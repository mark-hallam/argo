/**
 * Demo-mode wizard step 2 inputs must reach the in-app sidebar pill.
 *
 * Source: ux-audits/2026-05-27-2351 onboarding F3. Before this fix the
 * sidebar UserPill on every `(app)` route hardcoded
 * "Sam Harlow · Acmeco · Owner" via the `DEMO_SESSION` constant in
 * `web/lib/skycot/use-session.ts`. The wizard's step 2 still asked for
 * a workspace name and company name, fired a celebration toast on
 * submit, and stored the answers in the zustand draft — but the pill
 * read "Acmeco · Owner" the moment the user left the wizard, making
 * the wizard step camera-visible theatre.
 *
 * This spec walks the demo wizard with custom inputs and asserts the
 * post-wizard sidebar pill reflects the typed workspace name. It
 * `test.skip`s itself when `NEXT_PUBLIC_SKYCOT_DEMO_MODE` is unset
 * (same pattern as `onboarding-demo-mode-step-2-continue.spec.ts`)
 * because the demo branch is the only path that touches
 * `getDemoSession()` — outside demo mode the pill is JWT-derived and
 * has its own coverage.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

test("typed workspace name reaches the in-app sidebar pill in demo mode", async ({
  page,
}) => {
  await page.goto(appUrl("/onboarding/welcome"));
  await page.getByTestId("onboarding-welcome-continue").click();

  await expect(page).toHaveURL(/\/onboarding\/workspace/);
  await page.getByTestId("workspace-name").fill("Acme Growth");
  await page.getByTestId("company-name").fill("Acme Inc.");
  await page.getByTestId("onboarding-workspace-submit").click();

  await expect(page, "Step 2 Continue did not advance to connect-provider").toHaveURL(
    /\/onboarding\/connect-provider/,
    { timeout: 10_000 },
  );

  // Drop straight into an in-app route (the wizard chrome doesn't
  // mount the AppShell + UserPill); /team renders AppShell and is the
  // default first-hire landing.
  await page.goto(appUrl("/team"));
  const pill = page.getByTestId("appshell-user-workspace");
  await expect(pill).toBeVisible();
  await expect(pill).toHaveText("Acme Growth · Owner");
  // Regression guard: the old hardcoded label must not return.
  await expect(pill).not.toHaveText(/Acmeco/i);
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
