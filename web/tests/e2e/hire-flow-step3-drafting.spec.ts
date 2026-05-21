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

test("Step 3 surfaces the GenesisFallbackBanner when draft-worker 500s", async ({
  page,
}) => {
  // Classify still falls through to the stub so Step 2 advances; the
  // draft-worker endpoint returns a 500 to exercise the banner's
  // 5xx-error branch (mutation throws → hook synthesises
  // source === "stub").
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({ status: 404, contentType: "text/plain", body: "" }),
  );
  await page.route("**/api/v1/genesis/draft-worker", (route) =>
    route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({ error: "internal" }),
    }),
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
  // The banner surfaces once the mutation lands its error (a single
  // tick after mount). The plain-language "Generic questions only"
  // copy is load-bearing — Playwright treats it as a visible string.
  const banner = page.getByTestId("hire-step3-fallback-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText(/generic questions only/i);
  // Retry button is rendered and reachable; clicking it does not
  // throw. The mutation re-fires against the same mocked 500 so the
  // banner stays — we just assert the click is wired.
  const retry = page.getByTestId("hire-step3-fallback-retry");
  await expect(retry).toBeVisible();
  await expect(retry).toBeEnabled();
  await retry.click();
  await expect(banner).toBeVisible();
});
