import { expect, test } from "@playwright/test";

/**
 * E2-e — Hire flow Step 4 (Introduce).
 *
 * Walks Steps 1 → 2 → 3 → 4 with all three Genesis-track endpoints
 * stubbed out (404):
 *  - `/api/v1/genesis/classify`     → Step 2 falls back to STUB_QUESTIONS.
 *  - `/api/v1/genesis/draft-worker` → Step 3 falls back to `stubWorkerDraft`.
 *  - `/api/v1/workers`              → Step 4 falls back to `stub-<uuid>`.
 *
 * After "Hire Mariah" the user should land on `/hire/step/5` (the
 * Connect placeholder until E2-g lands).
 */
test("Step 1 → 2 → 3 → 4 reaches Step 5 via stub fallback", async ({ page }) => {
  await page.route("**/api/v1/genesis/classify", (route) =>
    route.fulfill({ status: 404, contentType: "text/plain", body: "" }),
  );
  await page.route("**/api/v1/genesis/draft-worker", (route) =>
    route.fulfill({ status: 404, contentType: "text/plain", body: "" }),
  );
  await page.route("**/api/v1/workers", (route) =>
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
  // Step 3's cosmetic timer is ~3.5s; allow well inside the 30s wall.
  await expect(page).toHaveURL(/\/hire\/step\/4$/, { timeout: 6_000 });

  // Step 4 invariants.
  await expect(page.getByTestId("hire-step4-introduce")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /Meet your new teammate\./i }),
  ).toBeVisible();
  await expect(page.getByTestId("hire-step4-worker-card")).toBeVisible();
  await expect(page.getByTestId("hire-step4-avatar")).toContainText("M");

  // Hire commit → optimistic advance to Step 5.
  await page.getByTestId("hire-step4-hire").click();
  await expect(page).toHaveURL(/\/hire\/step\/5$/, { timeout: 6_000 });
});

test("Step 4 redirects to Step 3 when no workerDraft is in state", async ({
  page,
}) => {
  // Land directly on /hire/step/4 with no prior state → bounces back
  // toward Step 3 (which in turn bounces toward Step 2 → /hire when
  // its own preconditions are missing).
  await page.goto("/hire/step/4");
  await expect(page).toHaveURL(/\/hire(\/step\/[123])?$|\/hire$/, {
    timeout: 3_000,
  });
});
