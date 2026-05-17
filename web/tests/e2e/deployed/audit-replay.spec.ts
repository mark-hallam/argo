/**
 * Deployed-real audit chain verification (audit finding #11, P1).
 *
 * After a deployed activation spec runs, the gateway should have an
 * intact hash-chained audit log covering the new tenant's actions.
 * This spec exercises the operator-facing `verify-chain` surface
 * exposed by `argo audit verify-chain --json` (proxied through the
 * gateway as `/admin/e2e/audit/verify-chain` for deployed runs) and
 * asserts a clean response.
 */

import { expect, test } from "@playwright/test";
import { resolveEnv, uniqueTestEmail } from "../fixtures/test-env";
import { DeployMailCatcher } from "./fixtures/mail-catcher-deploy";
import { deleteTestTenant } from "./fixtures/tenant-cleanup";

const env = resolveEnv();

test.describe("deployed-real audit replay", () => {
  test.skip(env.mode !== "deploy-smoke", "deployed lane only — set ARGO_E2E_TARGET=deploy");

  test("verify-chain returns clean after a real-tenant activation", async ({ page, request }) => {
    const email = uniqueTestEmail();
    const adminToken = requireEnv("ARGO_E2E_ADMIN_TOKEN");
    const catcher = new DeployMailCatcher(requireEnv("ARGO_E2E_MAIL_CATCHER_URL"));

    try {
      await page.goto(`${env.spaUrl}/auth/signup`);
      await page.getByTestId("signup-email").fill(email);
      await page.getByTestId("signup-submit").click();
      const link = await catcher.waitForLink(email, env.magicLinkTimeoutMs);
      await page.goto(link.url);
      await expect(page).toHaveURL(/\/onboarding\/welcome/, { timeout: 15_000 });

      const verifyRes = await request.get(
        `${env.gatewayUrl}/admin/e2e/audit/verify-chain?tenant_email=${encodeURIComponent(email)}`,
        { headers: { authorization: `Bearer ${adminToken}` } },
      );
      expect(verifyRes.ok(), "verify-chain endpoint not available").toBeTruthy();
      const verdict = (await verifyRes.json()) as {
        verified: boolean;
        head_seq: number;
        broken_at?: number;
      };
      expect(verdict.verified, `audit chain broken at seq ${verdict.broken_at ?? "?"}`).toBe(true);
      expect(verdict.head_seq, "audit chain empty for this tenant").toBeGreaterThan(0);
    } finally {
      const result = await deleteTestTenant(env.gatewayUrl, email, adminToken);
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
