import { expect, test } from "@playwright/test";

/**
 * Smoke tests for the shared wizard shell (`WizardEditorialShell`,
 * the F1 editorial-center locked 2026-05-19) as hosted by the hire
 * flow. The shell renders Step 1 chrome, a per-step progressbar, and
 * Back/Continue buttons in its CTA row.
 *
 * Shell-level concerns only — the step bodies (Step1Land, Step2Listen,
 * Step3Drafting, Step4Introduce, Step5Connect) are covered by their
 * own spec files. The end-to-end happy-path walk lives in
 * `hire-flow-step4-introduce.spec.ts` which already drives Step 1 → 5
 * with the genesis endpoints stubbed.
 *
 * The /hire/done landing is no longer an auto-redirect overlay; the
 * brand-loud payoff page IS the destination (per
 * web-hire-done-celebration-brandloud), so this spec asserts the
 * payoff renders, not a redirect to `/`.
 */
test("Step 1 renders the shared shell with progressbar and Step1 textarea", async ({
  page,
}) => {
  await page.goto("/hire");
  await expect(page.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "1");
  await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();
  await expect(page.getByTestId("hire-step1-textarea")).toBeVisible();
});

test("/hire/done renders the brand-loud payoff page (no auto-redirect)", async ({
  page,
}) => {
  await page.goto("/hire/done/stub?name=Robin");
  // The page IS the payoff — no auto-redirect away. The welcome card
  // and at least one CTA paint.
  await expect(page.getByTestId("hire-done-welcome-card")).toBeVisible();
  await expect(page).toHaveURL(/\/hire\/done\/stub/);
});

test("Back button is absent on step 1 (no prior step to return to)", async ({
  page,
}) => {
  await page.goto("/hire");
  // Step 1 doesn't wire `onBack`, so the shell omits the Back button
  // entirely (it isn't "disabled" — it isn't in the DOM).
  await expect(page.getByTestId("wiz-shell-back")).toHaveCount(0);
});

test("invalid step segment returns 404 within the hire route group", async ({ page }) => {
  const response = await page.goto("/hire/step/99");
  expect(response?.status()).toBe(404);
});
