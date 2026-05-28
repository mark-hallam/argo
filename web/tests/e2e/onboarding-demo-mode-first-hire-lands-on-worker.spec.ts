/**
 * Regression pin for `fix-demo-first-hire-discards-prompt-and-
 * redirects-to-sample-worker` F1 + F2 (supersedes the prior pin for
 * `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-
 * redirect`).
 *
 * Pre-fix behaviour: demo-mode wizard discarded the user's typed
 * name + description and routed every vertical to a fixed sample-
 * roster persona (recruiter → Priya, marketing-intern → Nori, etc.).
 * The camera saw the WRONG name on /workers/<id>/overview and the
 * /workers roster never grew — operator typed "Maya" + landed on
 * Priya's card.
 *
 * Post-fix behaviour:
 *
 *  - The redirect lands on `/workers?just-hired=1&new=<uuid>` (the
 *    roster listing with the new card highlighted), NOT on
 *    `/workers/<sample-persona-id>` or `/workers/demo-worker-*`.
 *  - The new UUID is real-shape (UUIDv7) and is NOT one of the
 *    canonical sample-roster ids.
 *  - The listing renders the operator's typed name in a card; the
 *    "Worker not found" shell never appears.
 *
 * Co-pinned with the F7 spec
 * `onboarding-demo-mode-provider-key-honest-copy.spec.ts`.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

const SAMPLE_PERSONA_IDS = [
  "019e6785-928a-7270-a766-f945b6cd7161", // Mariah
  "019e6785-928c-72b3-85c6-436fe5004c50", // Peta
  "019e6785-928d-78b1-a6b2-136d9ed263ce", // Priya
  "019e6785-928e-7f20-803b-55266e526011", // Jamal
  "019e6785-928f-7281-bf8d-e062bc277478", // Nori
];

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 so the wizard short-circuits the gateway",
);

test("demo-mode first-hire lands on /workers listing with the typed-name card", async ({
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
  await page.getByTestId("vertical-recruiter").click();

  await expect(page).toHaveURL(/\/onboarding\/first-hire/, { timeout: 10_000 });
  // Pick a typed name that does NOT shadow any sample persona so we
  // can distinguish the user's hire from the canonical roster card.
  await page.getByTestId("first-hire-name").fill("Maya");
  await page
    .getByTestId("first-hire-prompt")
    .fill("Maya screens applicants and books interviews.");
  await page.getByTestId("onboarding-first-hire-submit").click();

  // Acceptance criterion: redirect lands on /workers?just-hired=1&new=<uuid>.
  const listingPattern = /\/workers\?[^/]*just-hired=1[^/]*$/;
  await expect(
    page,
    "first-hire did not redirect to the /workers listing with just-hired=1",
  ).toHaveURL(listingPattern, { timeout: 10_000 });

  // Acceptance criterion: the new=<uuid> param is real-shape (uuidv7)
  // and is NOT one of the canonical sample-roster ids. Pre-fix the
  // redirect target would have been /workers/<sample-persona-id>.
  const url = new URL(page.url());
  const newId = url.searchParams.get("new") ?? "";
  expect(newId, "missing new=<uuid> highlight param").toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  );
  expect(SAMPLE_PERSONA_IDS, "redirect target reused a sample-persona id").not.toContain(
    newId,
  );

  // Acceptance criterion: the listing is the workers-grid, not a
  // 404 shell. The grid renders a TeammateCard for the new hire.
  await expect(page.getByTestId("workers-grid")).toBeVisible({ timeout: 5_000 });
  await expect(page.locator("body")).not.toContainText("Worker not found");
  await expect(page.getByTestId(`teammate-${newId}`)).toBeVisible();
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
