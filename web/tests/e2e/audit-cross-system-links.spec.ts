import { expect, test } from "@playwright/test";

/**
 * Audit drill-in cross-system jump links (E5-h) — mock-fallback smoke.
 *
 * The Jamal `_001` action in the mock dataset has a text-kind output body
 * containing a GitHub issue URL. Detection should wrap that URL with a
 * `<CrossSystemLink>` tagged `audit-output-cross-link-github`, and
 * clicking it should open a new browser tab targeting the GitHub URL.
 *
 * The webServer in `playwright.config.ts` boots `pnpm dev` with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` so the page hits the bundled
 * mock data deterministically. The previous fixture references
 * (`act_jamal_001`, `github.com/acmeco/ops/...`) predated the canonical
 * action-id shape + the Argo→Skycot rename.
 */
const JAMAL_ACTION_ID = "act_01J0JAMAL_001";
const JAMAL_GITHUB_URL = "https://github.com/skycot/ops/issues/2912";

test("renders cross-system jump link on drill-in and opens new tab", async ({
  page,
  context,
}) => {
  await context.route("https://github.com/**", (route) => route.abort());

  await page.goto(`/audit/action/${JAMAL_ACTION_ID}`);

  const link = page.getByTestId("audit-output-cross-link-github");
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute("href", JAMAL_GITHUB_URL);
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", /noopener/);

  // We aborted the github route up-top so the popup never actually
  // reaches github.com — its final URL settles on the Chromium error
  // page. We still want to assert that clicking the link DID open a
  // popup (i.e. target=_blank wired correctly + Cmd-click / fallback
  // click both produce a new tab), so look at the requested URL of
  // the first navigation rather than the post-abort settle URL.
  const requestPromise = context.waitForEvent("request", (req) =>
    req.url().includes("github.com/skycot/ops/issues/2912"),
  );
  const popupPromise = context.waitForEvent("page");
  await link.click({ modifiers: ["ControlOrMeta"] }).catch(() => link.click());
  const popup = await popupPromise;
  const request = await requestPromise;
  expect(request.url()).toContain("github.com/skycot/ops/issues/2912");
  await popup.close();
});
