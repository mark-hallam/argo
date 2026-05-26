import { expect, test } from "@playwright/test";

const RUNS_ROUTE = "**/api/v1/verticals/*/workflows/*/runs**";

test("workflow detail skips run-history fetch while serving demo workflow data", async ({
  page,
}) => {
  const runHistoryCalls: string[] = [];

  await page.route(RUNS_ROUTE, async (route) => {
    runHistoryCalls.push(route.request().url());
    await route.abort();
  });

  await page.goto("/workflows/sales/qualify_lead");

  await expect(page.getByTestId("wfd-source-pill")).toHaveAttribute(
    "data-source",
    /mock-(offline|bypass)/,
  );
  await expect(page.getByTestId("wfd-demo-runs-empty")).toContainText(
    "Demo workflow · no run history",
  );

  await page.waitForTimeout(500);
  expect(runHistoryCalls).toEqual([]);
});
