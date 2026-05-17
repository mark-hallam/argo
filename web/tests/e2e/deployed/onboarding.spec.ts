/**
 * Deployed-real onboarding E2E (audit finding #11, P1).
 *
 * Hits a live gateway/SPA/marketing trio rather than mock-fallback. The
 * full activation path:
 *
 *   marketing landing → /signup → magic-link sent → verify → 5-step
 *   wizard → first-hire submit → land on the worker profile.
 *
 * Run via:
 *   ARGO_E2E_TARGET=deploy \
 *   ARGO_E2E_GATEWAY_URL=https://api.skycot.com \
 *   ARGO_E2E_SPA_URL=https://app.skycot.com \
 *   ARGO_E2E_MARKETING_URL=https://skycot.com \
 *   ARGO_E2E_MAIL_CATCHER_URL=https://mailcatcher.example.com \
 *   pnpm test:e2e:deploy
 *
 * This spec INTENTIONALLY does no `page.route()` intercepts — the only
 * thing it stubs is "operator-resume" if the magic link doesn't arrive
 * in 60s. Everything else exercises the real backend.
 */

import { expect, test } from "@playwright/test";
import { resolveEnv, uniqueTestEmail } from "../fixtures/test-env";
import { DeployMailCatcher } from "./fixtures/mail-catcher-deploy";
import { deleteTestTenant } from "./fixtures/tenant-cleanup";

const env = resolveEnv();

test.describe("deployed-real onboarding", () => {
  test.skip(env.mode !== "deploy-smoke", "deployed lane only — set ARGO_E2E_TARGET=deploy");

  test("marketing → signup → magic-link → wizard → first hire", async ({ page }) => {
    const email = uniqueTestEmail();
    const mailCatcherUrl = requireEnv("ARGO_E2E_MAIL_CATCHER_URL");
    const catcher = new DeployMailCatcher(mailCatcherUrl);

    try {
      await page.goto(`${env.marketingUrl}/`);
      const cta = page.getByRole("link", { name: /hire your first worker/i }).first();
      await expect(cta, "marketing CTA not rendered").toBeVisible();

      await page.goto(`${env.spaUrl}/auth/signup`);
      await page.getByTestId("signup-email").fill(email);
      await page.getByTestId("signup-submit").click();
      await expect(page, "did not navigate to check-email").toHaveURL(
        /\/auth\/check-email/,
        { timeout: 15_000 },
      );

      const link = await catcher.waitForLink(email, env.magicLinkTimeoutMs);
      await page.goto(link.url);
      await expect(page, "verify did not redirect to onboarding/welcome").toHaveURL(
        /\/onboarding\/welcome/,
        { timeout: 15_000 },
      );

      await page.getByTestId("onboarding-welcome-continue").click();
      await expect(page).toHaveURL(/\/onboarding\/workspace/);
      await page.getByTestId("workspace-name").fill("E2E Smoke Workspace");
      await page.getByTestId("company-name").fill("E2E Smoke Co");
      await page.getByTestId("onboarding-workspace-submit").click();
      await expect(page).toHaveURL(/\/onboarding\/connect-provider/);

      const providerKey = process.env.ARGO_E2E_PROVIDER_KEY ?? "sk-ant-e2e-deploy-1234567890";
      await page.getByTestId("provider-key").fill(providerKey);
      await page.getByTestId("onboarding-provider-submit").click();
      await expect(page).toHaveURL(/\/onboarding\/pick-vertical/, { timeout: 15_000 });

      await page.getByTestId("vertical-marketing-intern").click();
      await expect(page).toHaveURL(/\/onboarding\/first-hire/);

      await page.getByTestId("first-hire-prompt").fill(
        "A marketing intern named Maya who drafts blog posts each Friday.",
      );
      await page.getByTestId("onboarding-first-hire-submit").click();

      await page.waitForURL(/\/(team|workers)\/[A-Za-z0-9\-_]+/, { timeout: 60_000 });
      await expect(
        page.getByText(/error/i),
        "worker page rendered an error overlay",
      ).toHaveCount(0);
    } finally {
      const result = await deleteTestTenant(
        env.gatewayUrl,
        email,
        process.env.ARGO_E2E_ADMIN_TOKEN,
      );
      if (!result.ok) {
        // Don't fail the test on cleanup miss — surface as a Playwright
        // annotation so the operator sees the leak without losing the
        // green-status signal on the actual product path.
        test.info().annotations.push({
          type: "cleanup-miss",
          description: `tenant ${email} not deleted: ${result.reason ?? "unknown"}`,
        });
      }
    }
  });
});

function requireEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`deployed lane requires env ${name}`);
  return v;
}
