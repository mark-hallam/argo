import { expect, test } from "@playwright/test";

/**
 * Regression pin for `fix-team-page-empty-state-despite-gateway-worker`.
 *
 * The defect: `/team` rendered the "Hire your first AI teammate"
 * empty-state grid even when the gateway had a roster, because the
 * page's render branch collapsed loading / error / silently-swallowed
 * fallbacks into the same `total === 0` empty case. That made Scene 1
 * of the v1.0 demo unrecordable — operator clicks the SKYCOT logo and
 * sees "0 working" instead of the C2 holographic trading-card deck.
 *
 * The Playwright webServer sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
 * and `NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:1`, so `fetchTeam()`
 * short-circuits to the bundled `SAMPLE_TEAM` (seven personas). That
 * gives us a deterministic non-empty roster without a live gateway,
 * which is sufficient to pin "when workers exist, the empty state
 * MUST NOT render".
 */
test("renders the trading-card deck — never the empty state — when workers exist", async ({
  page,
}) => {
  await page.goto("/team");

  // Sample personas now use UUID ids (`018f67d0-…`), not the legacy
  // `wkr_*` slugs the original fixture predated. Match the canonical
  // card id shape — `teammate-<uuid8>-…` — so the deck count is correct.
  const cards = page.getByTestId(/^teammate-[0-9a-f]{8}-/);
  await expect(cards.first()).toBeVisible();
  const count = await cards.count();
  expect(count).toBeGreaterThan(0);

  // The camera-visible defect: the "Hire your first AI teammate" empty
  // CTA grid must NEVER paint when at least one card is rendered.
  await expect(page.getByTestId("team-empty-state")).toHaveCount(0);

  // Subtitle reflects the loaded count (sanity-check the header
  // disagrees with the empty-state framing).
  await expect(page.getByTestId("team-subtitle")).toContainText(
    `${count} ${count === 1 ? "teammate" : "teammates"} working`,
  );
});

test("data-source attribute surfaces the non-gateway origin on the page header", async ({ page }) => {
  await page.goto("/team");

  // The visible MOCK-OFFLINE / MOCK-BYPASS chip was suppressed by
  // `fix-team-page-mock-offline-chip-suppress-during-recording` — it
  // leaked onto camera during the v1.0 demo recording, reading as
  // "broken software" to an investor. The diagnostic moved to a
  // `data-source` attribute on the page-header wrapper so DevTools
  // (and this regression pin) can still verify which path the page
  // is reading without showing the chip to a user.
  const header = page.getByTestId("team-page-header");
  await expect(header).toHaveAttribute("data-source", "mock-bypass");
  // The visible chip must NOT render with the default env profile.
  await expect(page.getByTestId("team-data-source-chip")).toHaveCount(0);
});
