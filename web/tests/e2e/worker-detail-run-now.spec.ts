import { expect, test } from "@playwright/test";

/**
 * Worker-detail Run-now affordance (brief
 * `fix-worker-detail-run-now-action-and-endpoint`).
 *
 * Demo runbook Scene 2 needs an operator-facing "click → run → audit
 * drill-in" path on `/workers/<id>/overview`. Before this brief, no
 * Run-now button existed on the worker surface at all.
 *
 * The local Playwright lane points the SPA at `127.0.0.1:1` so a real
 * gateway call would fail and the SPA would fall back to mocked
 * sample data. This spec intercepts the gateway run endpoint
 * `/api/v1/workers/<id>/run` directly with `page.route()` (the SPA now
 * calls it via the shared authenticated `apiPost` client, the same path
 * `hireWorker` uses — not the old `/api/workers/<id>/run` Next proxy
 * handler) so the test exercises:
 *   1. The button is rendered above the Responsibilities card.
 *   2. Clicking it fires a POST to the proxy with the documented body.
 *   3. The success toast renders with "View in audit" affordance.
 *   4. The affordance deep-links to `/audit/action/<action_id>`.
 */

// Mariah — first entry in SAMPLE_WORKERS_DETAIL.
const SAMPLE_WORKER_ID = "018f67d0-0000-7000-8000-000000000001";
const FAKE_ACTION_ID = "deadbeef000000000000000000000001";

test("worker overview Run-now button POSTs and toast deep-links to /audit", async ({
  page,
}) => {
  let postBody: string | null = null;
  await page.route(`**/api/v1/workers/${SAMPLE_WORKER_ID}/run`, async (route) => {
    postBody = route.request().postData();
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        action_id: FAKE_ACTION_ID,
        audit_id: FAKE_ACTION_ID,
        trigger_id: "trig-test-01",
        queued_at: "2026-05-27T04:15:00Z",
      }),
    });
  });

  await page.goto(`/workers/${SAMPLE_WORKER_ID}/overview`);

  const runNow = page.getByTestId("worker-overview-run-now");
  await expect(runNow).toBeVisible();

  await runNow.click();

  await expect(page.getByText(/is running now/i)).toBeVisible();
  expect(postBody).not.toBeNull();
  expect(JSON.parse(postBody!)).toMatchObject({
    trigger_source: "manual_run_now",
  });

  await page.getByRole("button", { name: /view in audit/i }).click();
  await expect(page).toHaveURL(
    new RegExp(`/audit/action/${FAKE_ACTION_ID}$`),
  );
});

test("worker overview Run-now button shows error toast on proxy failure", async ({
  page,
}) => {
  await page.route(`**/api/v1/workers/${SAMPLE_WORKER_ID}/run`, async (route) => {
    await route.fulfill({
      status: 502,
      contentType: "application/json",
      body: JSON.stringify({
        error: {
          code: "gateway_unreachable",
          message: "Couldn't reach the gateway.",
        },
      }),
    });
  });

  await page.goto(`/workers/${SAMPLE_WORKER_ID}/overview`);
  await page.getByTestId("worker-overview-run-now").click();

  await expect(page.getByText(/couldn't trigger a run/i)).toBeVisible();
});
