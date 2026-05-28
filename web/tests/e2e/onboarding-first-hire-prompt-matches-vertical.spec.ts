/**
 * Regression pin for `fix-onboarding-first-hire-example-prompt-by-vertical` F12 [P2].
 *
 * Before the fix, step 5 hardcoded a single placeholder ("A marketing
 * intern named Maya…") regardless of which template the operator had
 * picked in step 4. Combined with Genesis dispatching on archetype
 * words in the prompt text, the result was: pick GTM Engineer → type
 * the placeholder verbatim → get a Marketing Intern plan.
 *
 * Fix: `web/lib/onboarding/example-prompts.ts` keys the placeholder on
 * `draft.verticalId`. This spec walks the wizard for three different
 * verticals and asserts the textarea placeholder mentions the
 * picked-vertical persona (Mariah for GTM, Maya for marketing intern,
 * Priya for recruiter — names match the example-prompts lookup table).
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

interface VerticalCase {
  testId: string;
  expectedNameInPlaceholder: RegExp;
}

const CASES: ReadonlyArray<VerticalCase> = [
  { testId: "vertical-gtm-engineer", expectedNameInPlaceholder: /Mariah|GTM engineer/i },
  {
    testId: "vertical-marketing-intern",
    expectedNameInPlaceholder: /Maya|marketing intern/i,
  },
  { testId: "vertical-recruiter", expectedNameInPlaceholder: /Rachel|recruiter/i },
];

for (const { testId, expectedNameInPlaceholder } of CASES) {
  test(`first-hire placeholder matches picked vertical (${testId})`, async ({ page }) => {
    await page.goto(appUrl("/onboarding/welcome"));
    await page.getByTestId("onboarding-welcome-continue").click();

    await expect(page).toHaveURL(/\/onboarding\/workspace/);
    await page.getByTestId("workspace-name").fill("Acmeco Growth");
    await page.getByTestId("company-name").fill("Acmeco Inc.");
    await page.getByTestId("onboarding-workspace-submit").click();

    await expect(page).toHaveURL(/\/onboarding\/connect-provider/);
    await page.getByTestId("provider-key").fill("sk-ant-demo-stub-1234567890");
    await page.getByTestId("onboarding-provider-submit").click();

    await expect(page).toHaveURL(/\/onboarding\/pick-vertical/, { timeout: 10_000 });
    await page.getByTestId(testId).click();

    await expect(page).toHaveURL(/\/onboarding\/first-hire/, { timeout: 10_000 });

    const placeholder = await page
      .getByTestId("first-hire-prompt")
      .getAttribute("placeholder");
    expect(placeholder, "placeholder should reflect the picked vertical").not.toBeNull();
    expect(placeholder ?? "").toMatch(expectedNameInPlaceholder);
  });
}

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
