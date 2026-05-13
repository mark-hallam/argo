import { expect, test } from "@playwright/test";

/**
 * Audit drill-in cross-system jump links (E5-h) — mock-fallback smoke.
 *
 * `act_jamal_001` in the mock dataset has a text-kind output body
 * containing a GitHub issue URL. Detection should wrap that URL with
 * a `<CrossSystemLink>` tagged `audit-output-cross-link-github`, and
 * clicking it should open a new browser tab targeting the GitHub
 * dashboard URL.
 *
 * The webServer in `playwright.config.ts` boots `pnpm dev` with
 * `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` so the page hits the bundled
 * mock data deterministically.
 */
test("renders cross-system jump link on drill-in and opens new tab", async ({
  page,
  context,
}) => {
  await context.route("https://github.com/**", (route) => route.abort());

  await page.goto("/audit/action/act_jamal_001");

  const link = page.getByTestId("audit-output-cross-link-github");
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute(
    "href",
    "https://github.com/acmeco/ops/issues/2912",
  );
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", /noopener/);

  const popupPromise = context.waitForEvent("page");
  await link.click({ modifiers: ["ControlOrMeta"] }).catch(() => link.click());
  const popup = await popupPromise;
  expect(popup.url()).toContain("github.com/acmeco/ops/issues/2912");
  await popup.close();
});
