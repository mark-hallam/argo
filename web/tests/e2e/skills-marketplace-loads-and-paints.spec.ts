import { expect, test } from "@playwright/test";

/**
 * Skills marketplace — first-paint smoke. Brief
 * `fix-skills-marketplace-perpetual-loading` requires that /skills
 * paints ≥6 skill cards within 2 seconds in demo mode and that the
 * "Loading skills from the marketplace…" copy disappears as soon as
 * the fetch resolves.
 *
 * The playwright webServer boots with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
 * (see playwright.config.ts) so the SPA short-circuits to the bundled
 * SEED_SKILLS catalog (`web/lib/api/skills.ts`). That catalog ships
 * with ≥6 entries including the Scene-4 narration anchors (approval
 * routing, invoice extraction, candidate screening). The same shape
 * the gateway-backed path produces once `argo demo seed` runs.
 */
test("paints ≥6 skill cards and clears the loading state", async ({ page }) => {
  await page.goto("/skills");

  // Loading copy must disappear (data resolves under bypass within ms,
  // but allow 2s headroom per acceptance criterion).
  await expect(page.getByTestId("skills-loading")).toBeHidden({ timeout: 2000 });

  const cards = page.getByTestId("skill-card");
  await expect(cards.first()).toBeVisible();
  const cardCount = await cards.count();
  expect(cardCount).toBeGreaterThanOrEqual(6);

  // Count pill reflects N skills · N shown shape; both numbers ≥6.
  const countPill = page.getByTestId("skills-count");
  await expect(countPill).toBeVisible();
  const pillText = (await countPill.textContent()) ?? "";
  const match = pillText.match(/(\d+)\s+skills\s+·\s+(\d+)\s+shown/);
  expect(match).not.toBeNull();
  if (match) {
    const total = Number.parseInt(match[1], 10);
    const shown = Number.parseInt(match[2], 10);
    expect(total).toBeGreaterThanOrEqual(6);
    expect(shown).toBeGreaterThanOrEqual(6);
  }
});

test("filter chip narrows the catalog without re-showing the spinner", async ({
  page,
}) => {
  await page.goto("/skills");
  await expect(page.getByTestId("skills-loading")).toBeHidden({ timeout: 2000 });

  const search = page.getByTestId("skills-search");
  await search.fill("approval");

  // Spinner must NOT reappear on text-input filter (search filters
  // are pure client-side over the cached list).
  await expect(page.getByTestId("skills-loading")).toBeHidden();
  await expect(page.getByTestId("skill-card").first()).toBeVisible();
});
