/**
 * Deployed-real approval round-trip (audit finding #11, P1).
 *
 *   tenant exists → worker emits an ActionAsked → approvals inbox
 *   shows the card → operator clicks Approve → backend transitions to
 *   ActionExecuted → inbox card disappears.
 *
 * The "operator" here is the test itself — we sign in as the tenant
 * owner created in pre-flight, find the seeded approval (or trigger
 * one via the worker's send-message tool), click Approve, and assert
 * the gateway's audit log records the round-trip.
 *
 * Operators run this as part of `pnpm test:e2e:deploy` after a fly
 * deploy. RED runs fail `deploy-smoke.yml` but do not block PR merges.
 */

import { expect, test } from "@playwright/test";
import { resolveEnv, uniqueTestEmail } from "../fixtures/test-env";
import { DeployMailCatcher } from "./fixtures/mail-catcher-deploy";
import { deleteTestTenant } from "./fixtures/tenant-cleanup";

const env = resolveEnv();

test.describe("deployed-real approval round-trip", () => {
  test.skip(env.mode !== "deploy-smoke", "deployed lane only — set ARGO_E2E_TARGET=deploy");

  test("ApprovalAsked → operator Approve → ActionExecuted", async ({ page, request }) => {
    const email = uniqueTestEmail();
    const mailCatcherUrl = requireEnv("ARGO_E2E_MAIL_CATCHER_URL");
    const catcher = new DeployMailCatcher(mailCatcherUrl);

    try {
      await page.goto(`${env.spaUrl}/auth/signup`);
      await page.getByTestId("signup-email").fill(email);
      await page.getByTestId("signup-submit").click();
      const link = await catcher.waitForLink(email, env.magicLinkTimeoutMs);
      await page.goto(link.url);
      await expect(page).toHaveURL(/\/onboarding\/welcome/, { timeout: 15_000 });

      // Seed an approval via the gateway's e2e helper rather than the
      // wizard — keeps this spec focused on the approve round-trip and
      // independent of onboarding flakiness.
      const seedRes = await request.post(`${env.gatewayUrl}/admin/e2e/seed-approval`, {
        headers: { authorization: `Bearer ${requireEnv("ARGO_E2E_ADMIN_TOKEN")}` },
        data: { tenant_email: email, verb: "send the listing email" },
      });
      expect(seedRes.ok(), "seed-approval helper not available on this deploy").toBeTruthy();
      const seed = (await seedRes.json()) as { approval_id: string };

      await page.goto(`${env.spaUrl}/`);
      const card = page
        .getByTestId("approval-card")
        .filter({ has: page.getByText(seed.approval_id, { exact: false }) })
        .first();
      await expect(card, "seeded approval not rendered").toBeVisible({ timeout: 15_000 });
      await card.getByTestId("approval-approve").click();
      await expect(card).toHaveCount(0, { timeout: 15_000 });

      const auditRes = await request.get(
        `${env.gatewayUrl}/admin/e2e/approvals/${encodeURIComponent(seed.approval_id)}`,
        { headers: { authorization: `Bearer ${requireEnv("ARGO_E2E_ADMIN_TOKEN")}` } },
      );
      expect(auditRes.ok(), "audit lookup endpoint not available on this deploy").toBeTruthy();
      const audit = (await auditRes.json()) as { state: string };
      expect(audit.state, "approval did not transition past Asked").toMatch(/Executed|Approved/);
    } finally {
      const result = await deleteTestTenant(
        env.gatewayUrl,
        email,
        process.env.ARGO_E2E_ADMIN_TOKEN,
      );
      if (!result.ok) {
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
