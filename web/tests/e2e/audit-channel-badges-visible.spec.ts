import { expect, test } from "@playwright/test";

/**
 * Audit feed — channel-badge visibility (Claim 6 surface).
 *
 * Pre-fix (F12+F13), `/audit` rows showed no channel info at the
 * row level and the SPA's `ChannelKind` union excluded `discord`
 * entirely — so a Discord audit row rendered with the unknown-channel
 * fallback (no badge). This spec pins the post-fix surface:
 *
 * 1. The audit list paints multiple distinct channel badges across
 *    its visible rows (Claim 6 vocabulary: "voice, SMS, Slack,
 *    Discord").
 * 2. At least one badge is the Discord variant (the new tile that
 *    closes F12 — Discord referenced nowhere visible in the SPA).
 *
 * The webServer in `playwright.config.ts` boots `pnpm dev` with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, so the page hits the
 * bundled `SAMPLE_AUDIT_ROWS` mock dataset — which now includes
 * Discord seeds.
 */
test("audit feed shows multiple distinct channel badges including Discord", async ({
  page,
}) => {
  await page.goto("/audit");

  // Wait for at least one audit row to hydrate before counting badges.
  await expect(page.locator('[data-testid^="audit-row-"]').first()).toBeVisible();

  const badges = page.locator('[data-testid^="audit-row-channel-"]');
  // Collect distinct `data-kind` values across the painted badges.
  const kinds = await badges.evaluateAll((els) =>
    Array.from(new Set(els.map((el) => el.getAttribute("data-kind") ?? ""))),
  );

  expect(kinds.length).toBeGreaterThanOrEqual(3);
  expect(kinds).toContain("discord");
});
