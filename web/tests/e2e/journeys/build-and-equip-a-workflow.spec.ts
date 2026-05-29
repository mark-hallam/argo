import { expect, test } from "@playwright/test";

/**
 * JOURNEY — Build & equip (skills marketplace → workflow builder).
 *
 * The "platform builds workers" path (claim 3/4): browse the skill
 * marketplace, filter it, then start a workflow from a vertical and land
 * on the builder canvas. Chains the catalog → builder surfaces that the
 * atomic specs cover only in isolation.
 *
 * Local lane: auth-bypass + dead gateway → deterministic SEED_SKILLS /
 * SAMPLE_WORKFLOW_SPECS. Anchors are testids, never networkidle.
 */
test("skills marketplace → filter → workflow builder blank canvas", async ({ page }) => {
  // 1. Skills marketplace paints its catalog.
  await page.goto("/skills");
  await expect(page.getByTestId("skills-loading")).toBeHidden({ timeout: 5000 });
  const cards = page.getByTestId("skill-card");
  await expect(cards.first()).toBeVisible();
  const total = await cards.count();
  expect(total).toBeGreaterThan(0);

  // 2. The hero search narrows the catalog (signature marketplace UX).
  await page.getByTestId("skills-search").fill("zzzz-no-such-skill");
  await expect(cards).toHaveCount(0);
  await page.getByTestId("skills-search").fill("");
  await expect(cards.first()).toBeVisible();

  // 3. Cross to the workflow builder entry (the vertical picker).
  await page.goto("/workflows/build");
  await expect(page.getByRole("heading", { name: /blank canvas/i })).toBeVisible();

  // 4. Picking a vertical lands on the builder canvas for that vertical.
  await page.getByText("Sales", { exact: true }).first().click();
  await expect(page).toHaveURL(/\/workflows\/sales\//);
});
