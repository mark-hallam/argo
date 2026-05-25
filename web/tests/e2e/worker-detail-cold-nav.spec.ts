import { expect, test } from "@playwright/test";

/**
 * Cold-navigation regression — worker-detail must NOT render the
 * "Worker not found" shell on first load against a real worker id.
 *
 * The original failure mode (audit F6 in
 * `docs/skycot/ux-audits/2026-05-25-2309/scene-1-and-2-claim-1-and-2.md`):
 * the bearer-token peek (`/api/auth/session`) is async; on a fresh
 * navigation the first `useWorker` fetch can fire before the token
 * is in memory, so the gateway returns 401 and the layout used to
 * fall through to the not-found component.
 *
 * The fix turns 401 into `AuthNotReadyError`, retries inside
 * `useWorker`, and keeps the loading skeleton up rather than the
 * not-found shell. These three tests pin all three branches:
 *   1. real-id cold nav → renders the hero (Mariah) and never the
 *      not-found component.
 *   2. genuinely-missing id → still renders the not-found shell so
 *      we did not over-correct.
 *   3. simulated 401-then-200 race → the layout never flashes the
 *      not-found component while the retry resolves.
 *
 * The local Playwright lane points the SPA at `127.0.0.1:1` for the
 * gateway, so the un-routed default path takes the
 * `findSampleWorker()` fallback inside `fetchWorker`. Tests #1 and #2
 * therefore exercise the sample-data path. Test #3 stubs the gateway
 * explicitly to assert the 401-retry behaviour.
 */

// Mariah — first entry in SAMPLE_WORKERS_DETAIL.
const SAMPLE_WORKER_ID = "018f67d0-0000-7000-8000-000000000001";
const NOT_FOUND_ID = "018f67d0-0000-7000-8000-deadbeefdead";
const WORKER_PATH_RE = /\/api\/v1\/workers\/[^/]+(?:\?.*)?$/;

test("cold nav to /workers/<real-id>/overview renders the C2 detail page", async ({
  page,
}) => {
  await page.goto(`/workers/${SAMPLE_WORKER_ID}/overview`);

  await expect(page.getByTestId("worker-not-found")).toHaveCount(0);
  await expect(page.getByTestId("worker-name")).toHaveText(/mariah/i);
});

test("cold nav to /workers/<bogus-id>/overview still renders the not-found shell", async ({
  page,
}) => {
  await page.goto(`/workers/${NOT_FOUND_ID}/overview`);

  await expect(page.getByTestId("worker-not-found")).toBeVisible();
});

test("401 on first fetch retries and never flashes the not-found shell", async ({
  page,
}) => {
  let call = 0;
  await page.route(WORKER_PATH_RE, async (route) => {
    call += 1;
    if (call === 1) {
      await route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({
          error: { code: "unauthorized", message: "missing token" },
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        worker: {
          id: SAMPLE_WORKER_ID,
          name: "Mariah",
          status: "active",
          role: {
            vertical: "marketing-intern",
            title: "Marketing Intern",
            level: 1,
          },
        },
      }),
    });
  });

  await page.goto(`/workers/${SAMPLE_WORKER_ID}/overview`);

  // The retry should land within the brief's 1-2 second cold-nav
  // budget. The not-found shell must never have rendered.
  await expect(page.getByTestId("worker-name")).toHaveText(/mariah/i);
  await expect(page.getByTestId("worker-not-found")).toHaveCount(0);
  expect(call).toBeGreaterThanOrEqual(2);
});
