import { expect, test } from "@playwright/test";

/**
 * Stream `web-workflow-detail-add-as-schedule-button` — E2E for the
 * `+ Add as Schedule` CTA + 3-step drawer on the workflow viewer.
 *
 * Local lane: gateway URL is unroutable, so the workflow viewer falls
 * back to the bundled `sample-workflows` row (`support/triage_ticket`)
 * and the worker list falls back to sample workers in the `support`
 * vertical (`Jamal`). The duty-commit POST is intercepted so the test
 * asserts the wire body shape + happy-path toast without needing the
 * gateway.
 */

const COMMIT_PATH_RE = /\/api\/v1\/workers\/[^/]+\/duties\/commit$/;

test("add-as-schedule: header CTA → 3-step drawer → POST → success toast", async ({
  page,
}) => {
  let commitCalls = 0;
  let lastBody: unknown = null;

  await page.route(COMMIT_PATH_RE, async (route) => {
    commitCalls += 1;
    lastBody = JSON.parse(route.request().postData() ?? "{}");
    await route.fulfill({
      status: 201,
      contentType: "application/json",
      body: JSON.stringify({
        duty: {
          id: "dty_e2e_1",
          label: "triage_ticket",
          schedule: "cron: 0 9 * * *",
          enabled: true,
        },
        nextFireAt: "2026-05-24T09:00:00Z",
      }),
    });
  });

  await page.goto("/workflows/support/triage_ticket");

  // Header CTA visible + opens drawer.
  const cta = page.getByTestId("wfd-add-as-schedule");
  await expect(cta).toBeVisible();
  await cta.click();

  const drawer = page.getByTestId("add-as-schedule-drawer");
  await expect(drawer).toBeVisible();
  await expect(page.getByTestId("add-as-schedule-title")).toContainText(
    "Add as Schedule",
  );

  // Step 1: pick Jamal (the lone support-vertical sample worker).
  const jamalRow = page.getByTestId("add-as-schedule-worker-wkr_01J0JAMAL");
  await expect(jamalRow).toBeVisible();
  await jamalRow.click();
  await page.getByTestId("add-as-schedule-next").click();

  // Step 2: default cadence preselected (daily-9am) → next.
  await expect(
    page.getByTestId("add-as-schedule-cadence-daily-9am"),
  ).toHaveAttribute("data-checked", "true");
  await page.getByTestId("add-as-schedule-next").click();

  // Step 3: confirm panel summarises workflow + worker + cadence.
  const confirm = page.getByTestId("add-as-schedule-confirm");
  await expect(confirm).toBeVisible();
  await expect(confirm).toContainText("triage_ticket");
  await expect(confirm).toContainText("Jamal");
  await expect(confirm).toContainText("Daily at 9:00 AM");

  // Submit → POST fires → success toast.
  await page.getByTestId("add-as-schedule-submit").click();
  await expect(page.getByText(/Schedule created/)).toBeVisible({
    timeout: 5_000,
  });

  // Drawer closes after success.
  await expect(drawer).toBeHidden({ timeout: 5_000 });

  // Wire-level assertions.
  expect(commitCalls).toBe(1);
  expect(lastBody).toMatchObject({
    proposal: {
      kind: "recurring",
      label: "triage_ticket",
      schedule: { kind: "cron", spec: "0 9 * * *" },
    },
  });
});

test("add-as-schedule: Escape closes the drawer", async ({ page }) => {
  await page.goto("/workflows/support/triage_ticket");
  await page.getByTestId("wfd-add-as-schedule").click();

  const drawer = page.getByTestId("add-as-schedule-drawer");
  await expect(drawer).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(drawer).toBeHidden({ timeout: 2_000 });
});
