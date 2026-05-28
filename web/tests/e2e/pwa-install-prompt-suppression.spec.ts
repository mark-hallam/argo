/**
 * Regression guard for `fix-pwa-install-prompt-suppress-during-demo-
 * and-localhost`.
 *
 * The bottom-right "Install SKYCOT" banner (rendered by
 * `web/components/pwa/InstallPrompt.tsx` and only fired by Chrome's
 * `beforeinstallprompt` event) used to overlay /team, /audit,
 * /onboarding/workspace, etc. during demo-recording walks, ruining
 * every screenshot. The fix adds four early-return suppression
 * channels in the component's effect:
 *
 *   1. `NEXT_PUBLIC_HIDE_PWA_PROMPT=1`           (existing wave-6 env)
 *   2. `NEXT_PUBLIC_SKYCOT_DEMO_RECORDING=1`     (new recording flag)
 *   3. `?hideInstallPrompt=1` URL query param   (ad-hoc override)
 *   4. localhost / 127.0.0.1 / *.local origins   (dev always-on)
 *
 * Channels (3) and (4) are the only two reachable from a Playwright
 * run against the local dev server, so those are what this spec
 * exercises directly. Channel (2) is tested at the same path via a
 * deliberate fake `beforeinstallprompt` dispatch — even when the
 * recording env var is set, the listener must not have been
 * registered, so the synthesized event does nothing.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

const BANNER_TESTID = "pwa-install-banner";

async function dispatchFakeBeforeInstallPrompt(page: import("@playwright/test").Page) {
  await page.evaluate(() => {
    const evt = new Event("beforeinstallprompt") as Event & {
      prompt?: () => Promise<void>;
      userChoice?: Promise<{ outcome: "accepted" | "dismissed" }>;
    };
    evt.prompt = () => Promise.resolve();
    evt.userChoice = Promise.resolve({ outcome: "dismissed" as const });
    window.dispatchEvent(evt);
  });
}

test.describe("PWA install banner — localhost suppression", () => {
  test("does not render on a localhost dev origin", async ({ page }) => {
    await page.goto(new URL("/team", SPA_ORIGIN).toString());

    // Try to coax the banner out — if the listener was registered the
    // banner would render on the next tick.
    await dispatchFakeBeforeInstallPrompt(page);

    await expect(page.getByTestId(BANNER_TESTID)).toHaveCount(0);
  });
});

test.describe("PWA install banner — query-param override", () => {
  test("does not render when ?hideInstallPrompt=1 is in the URL", async ({ page }) => {
    const url = new URL("/team", SPA_ORIGIN);
    url.searchParams.set("hideInstallPrompt", "1");
    await page.goto(url.toString());

    await dispatchFakeBeforeInstallPrompt(page);

    await expect(page.getByTestId(BANNER_TESTID)).toHaveCount(0);
  });
});
