/**
 * Full new-user activation E2E (mission-onboarding-e2e-integration-test).
 *
 * Walks the 10-step viral path: marketing → signup → magic-link verify
 * → 5-step onboarding wizard → first-hire submit → land on worker.
 *
 * Run modes (`ARGO_E2E_TARGET`):
 * - unset / `local`: Next.js servers spun up by playwright.config.ts;
 *   backend calls are intercepted via `page.route()`; MailCatcher is
 *   in-memory.
 * - `deploy`: targets `skycot.com` + `app.skycot.com` + `api.skycot.com`;
 *   real gateway, real Postmark/SendGrid; MailCatcher polls an inbox
 *   webhook (falls back to operator-resume if nothing arrives in 60s).
 *
 * Tenant provisioning + worker landing (steps 9/10) are gated on
 * `E2E_TENANT_PROVISION_READY=1` while `claude-tenant-provisioning-endpoint`
 * is BLOCKED. See brief Hidden constraints.
 */

import { expect, test } from "@playwright/test";
import { MailCatcher } from "./fixtures/mail-catcher";
import { resolveEnv, uniqueTestEmail } from "./fixtures/test-env";

const env = resolveEnv();

test.describe("new-user activation path", () => {
  test("walks marketing → signup → magic-link → 5-step wizard → first hire", async ({ page }) => {
    const email = uniqueTestEmail();
    const catcher = new MailCatcher(env.spaUrl);

    // Step intercepts (local mode only). Order matters: install before
    // any page.goto() so the first request is captured.
    if (env.mode === "local") {
      await catcher.installLocalIntercepts(page);
      await installVerifyIntercept(page);
      await installOnboardingIntercepts(page);
    }

    // Step 1: marketing landing renders + CTA visible.
    await page.goto(`${env.marketingUrl}/`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const cta = page.getByRole("link", { name: /hire your first worker/i }).first();
    await expect(cta, "Step 1: hero CTA not rendered on marketing landing").toBeVisible();
    const ctaHref = await cta.getAttribute("href");
    expect(ctaHref, "Step 2: CTA missing href").toBeTruthy();
    // Sensible deviation: marketing currently points at `${APP_URL}/signup`,
    // not `/auth/signup`. Accept either shape until the route shim lands.
    expect(ctaHref).toMatch(/\/signup$/);

    // Step 3: jump straight to /auth/signup (see header note).
    await page.goto(`${env.spaUrl}/auth/signup`);
    await page.getByTestId("signup-email").fill(email);
    await page.getByTestId("signup-submit").click();
    await expect(page, "Step 3: did not navigate to check-email").toHaveURL(/\/auth\/check-email/);

    // Step 4: intercept magic link.
    const link = await catcher.waitForLink(email, env.magicLinkTimeoutMs);

    // Step 5: visit verify URL; expect onboarding/welcome.
    await page.goto(link.url);
    await expect(page, "Step 5: verify did not redirect to onboarding/welcome").toHaveURL(
      /\/onboarding\/welcome/,
      { timeout: 10_000 },
    );

    // Step 6: continue → workspace → fill + submit.
    await page.getByTestId("onboarding-welcome-continue").click();
    await expect(page).toHaveURL(/\/onboarding\/workspace/);
    await page.getByTestId("workspace-name").fill("Acme Growth");
    await page.getByTestId("company-name").fill("Acme Co");
    await page.getByTestId("onboarding-workspace-submit").click();
    await expect(page, "Step 6: workspace submit did not advance").toHaveURL(
      /\/onboarding\/connect-provider/,
    );

    // Step 7: paste stub provider key → next.
    await page.getByTestId("provider-key").fill("sk-ant-e2e-stub-1234567890");
    await page.getByTestId("onboarding-provider-submit").click();
    await expect(page, "Step 7: provider validate did not advance").toHaveURL(
      /\/onboarding\/pick-vertical/,
    );

    // Step 8: select marketing-intern template.
    await page.getByTestId("vertical-marketing-intern").click();
    await expect(page, "Step 8: vertical pick did not advance").toHaveURL(
      /\/onboarding\/first-hire/,
    );

    if (!env.tenantProvisionReady) {
      test.info().annotations.push({
        type: "skip",
        description:
          "Steps 9-10 gated on E2E_TENANT_PROVISION_READY=1 — depends on claude-tenant-provisioning-endpoint MERGED.",
      });
      return;
    }

    // Step 9: fill description → submit → wait for worker creation.
    await page.getByTestId("first-hire-prompt").fill(
      "A marketing intern named Maya who drafts blog posts each Friday.",
    );
    await page.getByTestId("onboarding-first-hire-submit").click();

    // Step 10: land on the worker profile (URL contains worker_id) and
    // assert the route returned 200 (no error overlay).
    await page.waitForURL(/\/(team|workers)\/[A-Za-z0-9\-_]+/, { timeout: 30_000 });
    await expect(
      page.getByText(/error/i),
      "Step 10: worker page rendered an error overlay",
    ).toHaveCount(0);
  });
});

async function installVerifyIntercept(page: import("@playwright/test").Page): Promise<void> {
  await page.route("**/api/auth/verify", async (route) => {
    const body = route.request().postDataJSON() as { token?: string };
    const token = body?.token ?? "";
    if (!token.startsWith("e2e-token-")) {
      await route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "invalid_token", message: "rejected by stub" } }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      headers: { "Set-Cookie": "skycot_session=stub-jwt; Path=/; HttpOnly; SameSite=Lax" },
      body: JSON.stringify({ ok: true, needs_onboarding: true, tenant_id: null }),
    });
  });
}

async function installOnboardingIntercepts(page: import("@playwright/test").Page): Promise<void> {
  await page.route("**/api/v1/onboarding/provision", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ tenant_id: "tenant-e2e", workspace_id: "ws-e2e" }),
    }),
  );
  await page.route("**/api/v1/onboarding/provider-key", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ validated: true, test_cost: 0.001 }),
    }),
  );
  await page.route("**/api/v1/onboarding/install-vertical", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ installed: true }),
    }),
  );
  await page.route("**/api/v1/onboarding/first-hire", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ worker_id: "worker-e2e", worker_name: "Maya", pending: false }),
    }),
  );
}
