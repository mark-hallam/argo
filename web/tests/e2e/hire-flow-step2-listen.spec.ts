import { expect, test } from "@playwright/test";

/**
 * E2-c — Hire flow Step 2 (Listen).
 *
 * Walks Step 1 → Step 2 with a forced gateway 404 on
 * `/api/v1/genesis/classify` so the test runs deterministically
 * without a live Genesis backend (the gateway endpoint is a sibling
 * brief: `mission-gateway-genesis-classify-endpoint`). The 404 trips
 * the client-side stub fallback path; the spec asserts the three
 * stub questions render, can be answered, and Continue advances the
 * flow to `/hire/step/3`.
 *
 * Status — runs once the repo's Playwright harness lands (same as
 * `hire-flow-shell.spec.ts`). Authored against the route + DOM
 * shape this brief locks.
 */
test("Step 1 → Step 2 happy path with stub fallback", async ({ page }) => {
  // Force the Genesis endpoint to 404 so the client falls back to
  // STUB_QUESTIONS deterministically.
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({
      status: 404,
      contentType: "text/plain",
      body: "",
    }),
  );

  await page.goto("/hire");
  const problem =
    "I sell hockey gear on Facebook Marketplace and get 30 messages a day.";
  await page.getByTestId("hire-step1-textarea").fill(problem);
  await page.getByTestId("hire-shell-continue").click();

  await expect(page).toHaveURL(/\/hire\/step\/2$/);
  await expect(page.getByTestId("hire-step2-listen")).toBeVisible();
  await expect(page.getByTestId("hire-step2-fallback-banner")).toBeVisible();
  await expect(page.getByTestId("hire-step2-question-stub_action_mode")).toBeVisible();
  await expect(page.getByTestId("hire-step2-question-stub_autonomy")).toBeVisible();
  await expect(page.getByTestId("hire-step2-question-stub_name")).toBeVisible();

  // Continue is disabled until all 3 stub questions are answered.
  await expect(page.getByTestId("hire-shell-continue")).toBeDisabled();

  await page
    .getByTestId("hire-step2-answer-stub_action_mode-Reply on my behalf")
    .click();
  await page.getByTestId("hire-step2-answer-stub_autonomy-no").click();
  await page.getByTestId("hire-step2-answer-stub_name").fill("Mariah");

  await expect(page.getByTestId("hire-shell-continue")).toBeEnabled();
  await page.getByTestId("hire-shell-continue").click();
  await expect(page).toHaveURL(/\/hire\/step\/3$/);
});

test("Step 2 redirects to Step 1 when draft has no problem", async ({ page }) => {
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({ status: 404, body: "" }),
  );
  await page.goto("/hire/step/2");
  // The page redirects to `/hire` immediately on hydrate.
  await expect(page).toHaveURL(/\/hire$/);
});
