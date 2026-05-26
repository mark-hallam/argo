/**
 * Regression pin for `fix-onboarding-demo-mode-fake-validation-and-
 * synthetic-worker-redirect` F2 [P0].
 *
 * Before the fix, the demo-mode connect-provider step advertised
 * **"$0.001 test call"** in the hero subtitle and rendered
 * **"Key validated. Forwarding…"** in the success banner without any
 * network round-trip. The camera saw a lie live on screen, in
 * violation of the operator-locked NO-THEATER rule
 * (`feedback_no_theater_in_demo`).
 *
 * The fix swaps the hero subtitle and success banner whenever demo
 * mode is on (or whenever the validation result carries
 * `mockProvider: true`). This spec asserts:
 *
 *  - The page content does NOT contain "$0.001" (the false-cost lie).
 *  - The hero subtitle DOES contain "Mock provider" copy.
 *  - The validate click still advances to /onboarding/pick-vertical
 *    (demo short-circuit still works end-to-end).
 *
 * The assertion lands on the hero subtitle rather than the success
 * banner because the banner is rendered for ~one React tick before
 * `router.push` navigates away — too narrow a window for stable
 * Playwright polling. The brief's acceptance is satisfied either way
 * ("the success banner does NOT contain `$0.001` OR contains the
 * string `Mock provider`"); the hero subtitle is the more stable
 * surface and carries the same honest copy.
 *
 * Co-pinned with the F1 spec
 * `onboarding-demo-mode-first-hire-lands-on-worker.spec.ts`.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

test("demo-mode provider-key copy is honest mock-provider, not '$0.001 test call'", async ({
  page,
}) => {
  await page.goto(appUrl("/onboarding/connect-provider"));
  await expect(page).toHaveURL(/\/onboarding\/connect-provider/);

  // Acceptance criterion: the false-cost lie is nowhere on the page.
  // Before the fix, the hero subtitle read "A $0.001 test call
  // confirms the key works" in demo mode.
  await expect(page.locator("body")).not.toContainText("$0.001");

  // Acceptance criterion: the page advertises the mock provider
  // honestly. The hero subtitle now reads "Demo mode — workers run
  // with a mock provider (simulated responses)."
  await expect(page.locator("body")).toContainText(/mock provider/i);

  // End-to-end demo path still works: type any sk-ant- key, submit,
  // and the wizard advances to step 4 (pick-vertical). This pins the
  // mockProvider short-circuit returns { validated: true } rather
  // than throwing.
  await page.getByTestId("provider-key").fill("sk-ant-anything-1234567890");
  await page.getByTestId("onboarding-provider-submit").click();
  await expect(page).toHaveURL(/\/onboarding\/pick-vertical/, { timeout: 10_000 });
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
