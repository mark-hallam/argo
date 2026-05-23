import { expect, test } from "@playwright/test";

/**
 * /settings/connectors — opens the connect modal on a stub (api-key)
 * card and confirms the credential-entry form renders.
 *
 * The gateway list endpoint (`GET /api/v1/connectors`) returns 404
 * during v1 rollout; the client falls back to an empty list, so the
 * grid renders without a live gateway. This spec asserts only the
 * client-side modal contract — nothing depends on a real OAuth
 * provider being reachable.
 */
test("opens the credential modal on the Notion card and closes cleanly", async ({
  page,
}) => {
  await page.goto("/settings/connectors");

  await expect(page.getByTestId("connectors-page")).toBeVisible();
  await expect(page.getByTestId("connector-card-notion")).toBeVisible();

  await page.getByTestId("connector-connect-notion").click();

  const modal = page.getByTestId("connector-modal");
  await expect(modal).toBeVisible();
  await expect(modal).toHaveAttribute("data-connector-id", "notion");
  await expect(page.getByTestId("connector-modal-apikey-form")).toBeVisible();

  await page.getByTestId("connector-modal-close").click();
  await expect(modal).toHaveCount(0);
});
