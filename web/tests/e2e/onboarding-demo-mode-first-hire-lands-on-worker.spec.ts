/**
 * Regression pin for `fix-onboarding-demo-mode-fake-validation-and-
 * synthetic-worker-redirect` F1 [P0].
 *
 * Before the fix, the demo-mode wizard ended Scene 3 with the
 * **WorkerNotFound** error shell because `buildDemoFirstHireResult`
 * minted a synthetic id of the shape `demo-worker-${vertical}-${ts}`
 * that matched no gateway worker and no sample-roster row. The brief
 * dropped the synthetic-id design and routes through the canonical
 * sample-worker UUIDs (Mariah `…0001`, Priya `…0002`, Jamal `…0003`,
 * Nori `…0004`), so the destination `/workers/<id>` page renders a
 * real C2 personality card via `findSampleWorker`'s fallback.
 *
 * This spec walks all five steps in demo mode, picks a vertical with
 * a clear persona match (recruiter → Priya), clicks Hire them, and
 * asserts:
 *
 *  - The destination URL contains a real UUID (`/workers/[a-f0-9-]{36}`),
 *    NOT `/workers/demo-worker-*`.
 *  - The page body does NOT contain "Worker not found".
 *  - The page DOES render the worker's name as a heading.
 *
 * Co-pinned with the F2 spec
 * `onboarding-demo-mode-provider-key-honest-copy.spec.ts`.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

test("demo-mode first-hire lands on a real-rendered worker, not WorkerNotFound", async ({
  page,
}) => {
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
  // Recruiter → Priya (`019e6785-928d-78b1-a6b2-136d9ed263ce`). Any of
  // the four matched verticals would do; recruiter is the one with the
  // tightest persona-text match for the assertion below.
  await page.getByTestId("vertical-recruiter").click();

  await expect(page).toHaveURL(/\/onboarding\/first-hire/, { timeout: 10_000 });
  await page
    .getByTestId("first-hire-prompt")
    .fill("A recruiter named Priya who screens applicants and schedules interviews.");
  await page.getByTestId("onboarding-first-hire-submit").click();

  // Acceptance criterion: redirect lands on /workers/<uuid>, not
  // /workers/demo-worker-*. The destination layout chooses its own
  // sub-tab (default /overview) so we match either /workers/<uuid>
  // or /workers/<uuid>/overview.
  const uuidPattern =
    /\/workers\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(?:\/.*)?$/;
  await expect(page, "first-hire did not redirect to a real worker UUID").toHaveURL(
    uuidPattern,
    { timeout: 10_000 },
  );
  expect(page.url(), "first-hire redirected to a synthetic demo-worker-* id").not.toMatch(
    /\/workers\/demo-worker-/,
  );

  // Acceptance criterion: the destination is NOT the WorkerNotFound
  // error shell. The shell's heading is the literal string
  // "Worker not found"; if it appears anywhere the regression is back.
  await expect(page.locator("body")).not.toContainText("Worker not found");

  // Acceptance criterion: the page renders the worker's name as a
  // heading. Priya is the recruiter vertical default; any persona
  // name landing in <h1> proves the C2 card mounted.
  await expect(page.locator("h1").first()).toContainText(/Priya|Mariah|Jamal|Nori/);
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
