/**
 * Demo-mode wizard walk (F5 / F5a / F5b from
 * docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md).
 *
 * Before this brief shipped, clicking Continue on Step 2
 * (`/onboarding/workspace`) in demo mode rendered, in red, inside the
 * form card:
 *   "X-Argo-Principal header must carry a verified human:<email> value"
 *
 * Two failures:
 *   1. The provision call wasn't short-circuited in demo mode, so the
 *      wizard couldn't advance without a real signed-in principal.
 *   2. The gateway's error envelope was rendered verbatim, leaking the
 *      `X-Argo-Principal` internal header name to the investor.
 *
 * This spec walks the full wizard in demo mode and asserts:
 *   - Continue on Step 2 advances to `/onboarding/connect-provider`
 *     without any red error banner.
 *   - No element on the page contains the substring `X-Argo-Principal`
 *     at any point during the walk.
 *
 * Requires the dev server to be running with
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` — that env var is build-time inlined
 * by Next, so it has to be set when `pnpm dev` starts. The spec
 * `test.skip`s itself when the var isn't set (matches the pattern from
 * `signup-demo-mint-routes-to-welcome.spec.ts`).
 */

import { expect, test, type Page } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

const PROTOCOL_LEAK_SUBSTRING = "X-Argo-Principal";

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

test("demo-mode wizard walks Step 2 → Step 3 without leaking X-Argo-Principal", async ({
  page,
}) => {
  // Tripwire: if the demo-mode branch ever regresses and the SPA falls
  // back to the gateway, the leak banner contains "X-Argo-Principal".
  // Fail the test the moment any DOM mutation produces that substring.
  const leakDetected: string[] = [];
  page.on("framenavigated", () => {
    void page
      .content()
      .then((html) => {
        if (html.includes(PROTOCOL_LEAK_SUBSTRING)) {
          leakDetected.push(page.url());
        }
      })
      .catch(() => {});
  });

  await page.goto(appUrl("/onboarding/welcome"));
  await page.getByTestId("onboarding-welcome-continue").click();

  await expect(page).toHaveURL(/\/onboarding\/workspace/);
  await page.getByTestId("workspace-name").fill("Acmeco Growth");
  await page.getByTestId("company-name").fill("Acmeco Inc.");
  await page.getByTestId("onboarding-workspace-submit").click();

  // The acceptance criterion: Step 2 Continue advances to Step 3.
  await expect(page, "Step 2 Continue did not advance to connect-provider").toHaveURL(
    /\/onboarding\/connect-provider/,
    { timeout: 10_000 },
  );

  // The acceptance criterion: no red error banner is visible. The
  // workspace page renders its error inside `[data-testid="onboarding-workspace-error"]`;
  // it should never appear in the successful demo walk.
  await expect(page.getByTestId("onboarding-workspace-error")).toHaveCount(0);

  // The acceptance criterion: no element on the page contains the
  // verbatim header name. Use locator + count rather than visibility so
  // we also catch hidden / aria-only leaks.
  await expectNoProtocolLeak(page);

  // Walk one more step to make sure provider-key bypass also works.
  await page.getByTestId("provider-key").fill("sk-ant-demo-stub-1234567890");
  await page.getByTestId("onboarding-provider-submit").click();
  await expect(page, "Step 3 Validate did not advance to pick-vertical").toHaveURL(
    /\/onboarding\/pick-vertical/,
    { timeout: 10_000 },
  );
  await expectNoProtocolLeak(page);

  expect(leakDetected, `X-Argo-Principal observed at: ${leakDetected.join(", ")}`).toEqual(
    [],
  );
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}

async function expectNoProtocolLeak(page: Page): Promise<void> {
  const html = await page.content();
  expect(
    html.includes(PROTOCOL_LEAK_SUBSTRING),
    `page leaked internal protocol detail "${PROTOCOL_LEAK_SUBSTRING}"`,
  ).toBe(false);
}
