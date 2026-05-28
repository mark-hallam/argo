import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test("settings/inbound-channels lists SMS / Discord / Voice with webhook URLs and copy buttons", async ({
  page,
}) => {
  await page.goto(appUrl("/settings/inbound-channels"));

  await expect(page.getByTestId("inbound-channels-page")).toBeVisible();

  for (const kind of ["sms", "discord", "voice"] as const) {
    const card = page.getByTestId(`inbound-card-${kind}`);
    await expect(card).toBeVisible();

    const url = page.getByTestId(`inbound-url-${kind}`);
    await expect(url).toBeVisible();
    // Each gateway path the operator pastes into the upstream console.
    const expectedPath =
      kind === "sms"
        ? "/twilio/sms/decision"
        : kind === "discord"
          ? "/channels/discord/decision"
          : "/twilio/voice/decision";
    await expect(url).toContainText(expectedPath);

    await expect(page.getByTestId(`inbound-copy-${kind}`)).toBeVisible();
    await expect(page.getByTestId(`inbound-test-${kind}`)).toBeVisible();
  }
});

test("settings/inbound-channels test button opens an explainer modal", async ({ page }) => {
  await page.goto(appUrl("/settings/inbound-channels"));
  await page.getByTestId("inbound-test-sms").click();
  const modal = page.getByTestId("inbound-test-modal");
  await expect(modal).toBeVisible();
  await expect(modal).toContainText("Send a test to SMS");
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
