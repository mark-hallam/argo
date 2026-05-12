import { expect, test } from "@playwright/test";

/**
 * E2-d — Hire flow Step 3 (Drafting transition).
 *
 * Walks Steps 1 → 2 → 3 with both Genesis endpoints stubbed (404):
 *  - `/api/v1/genesis/classify`  → Step 2 falls back to STUB_QUESTIONS.
 *  - `/api/v1/genesis/draft-worker` → Step 3 falls back to
 *    `stubWorkerDraft(...)`.
 *
 * The cosmetic timer is ~3.5s (5 stages × 700ms), so the redirect to
 * `/hire/step/4` should land well inside a 6-second window.
 */
test("Step 1 → 2 → 3 happy path with stub fallback advances to Step 4", async ({ page }) => {
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({ status: 404, contentType: "text/plain", body: "" }),
  );
  await page.route("**/api/v1/genesis/draft-worker", (route) =>
    route.fulfill({ status: 404, contentType: "text/plain", body: "" }),
  );

  await page.goto("/hire");
  await page
    .getByTestId("hire-step1-textarea")
    .fill("I sell hockey gear on Facebook Marketplace and get 30 messages a day.");
  await page.getByTestId("hire-shell-continue").click();

  await expect(page).toHaveURL(/\/hire\/step\/2$/);
  await page
    .getByTestId("hire-step2-answer-stub_action_mode-Reply on my behalf")
    .click();
  await page.getByTestId("hire-step2-answer-stub_autonomy-no").click();
  await page.getByTestId("hire-step2-answer-stub_name").fill("Mariah");
  await page.getByTestId("hire-shell-continue").click();

  await expect(page).toHaveURL(/\/hire\/step\/3$/);
  await expect(page.getByTestId("hire-step3-drafting")).toBeVisible();
  // Step 3 hides the HireShell footer (no Back / Continue).
  await expect(page.getByTestId("hire-shell-continue")).toHaveCount(0);
  await expect(page.getByTestId("hire-shell-back")).toHaveCount(0);

  // The cosmetic timer is ~3.5s; advance well inside the 30s wall.
  await expect(page).toHaveURL(/\/hire\/step\/4$/, { timeout: 6_000 });
});

test("Step 3 redirects to Step 2 when draft has no answers", async ({ page }) => {
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({ status: 404, body: "" }),
  );
  await page.goto("/hire/step/3");
  // No answers → bounces to /hire/step/2; that page in turn bounces
  // to /hire when there's no problem either.
  await expect(page).toHaveURL(/\/hire$/);
});
