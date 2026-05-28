import { expect, test } from "@playwright/test";

/**
 * Worker detail 404 routing (F12 [P2] from
 * `scene-1-and-2-claim-1-and-2.md`).
 *
 * `<WorkerNotFound>` is the in-shell empty state for a worker id
 * that doesn't resolve. Before this brief, demo + auth-bypass mode
 * fell through to a default mock for any id outside the canonical
 * persona set — `/workers/<random-uuid>/overview` rendered the
 * SAMPLE Mariah card, so a curious investor pasting a URL got a
 * fabricated worker.
 *
 * Asserts that an unseeded UUID lands on the not-found shell —
 * neither a fabricated overview body nor the global error boundary.
 */
test("unknown worker id renders the in-shell not-found empty state", async ({
  page,
}) => {
  const workerId = "00000000-0000-0000-0000-000000000000";
  await page.goto(`/workers/${workerId}/overview`);

  const notFound = page.getByTestId("worker-not-found");
  await expect(notFound).toBeVisible();
  await expect(notFound).toContainText(workerId);

  await expect(page.getByTestId("worker-tab-panel-overview")).toHaveCount(0);
});

/**
 * Canonical persona ids must still render real overview content —
 * the routing change does not regress known ids.
 */
test("canonical persona id still renders the overview body", async ({
  page,
}) => {
  await page.goto("/workers/019e6785-928a-7270-a766-f945b6cd7161/overview");

  await expect(page.getByTestId("worker-tab-panel-overview")).toBeVisible();
  await expect(page.getByTestId("worker-not-found")).toHaveCount(0);
});
