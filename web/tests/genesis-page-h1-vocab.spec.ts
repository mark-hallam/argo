import { expect, test } from "@playwright/test";

// Pins the /genesis surface vocabulary so the page does not regress
// into hire-wizard framing. Genesis is the AI Teammate Onboarding
// Studio (operator-locked memory `feedback_genesis_is_ai_teammate_onboarding_studio`):
// voice/text role + SOP/wiki/Slack uploads + access controls +
// comprehension verification. The `/onboarding/first-hire` wizard is
// the hire flow; /genesis must not duplicate its voice.
test("/genesis H1 frames the page as an onboarding studio, not a hire wizard", async ({
  page,
}) => {
  await page.goto(appUrl("/genesis"));

  await expect(page.getByTestId("genesis-page")).toBeVisible();

  const h1 = page.locator("h1").first();
  await expect(h1).toBeVisible();
  const h1Text = (await h1.innerText()).trim();
  expect(h1Text.toLowerCase()).not.toContain("hire your next teammate");
  expect(h1Text.toLowerCase()).toContain("onboard");
  expect(h1Text.toLowerCase()).toContain("teammate");
});

function appUrl(pathname: string): string {
  const base =
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(pathname, base).toString();
}
