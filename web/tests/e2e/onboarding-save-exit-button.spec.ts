/**
 * Onboarding "Exit setup" top-bar button — safety net.
 *
 * Source: `fix-onboarding-save-and-exit-button-safety` (UX-audit
 * onboarding-hire-an-ai-teammate F10, P2).
 *
 * Before the fix, the top-bar "SAVE & EXIT" button was a plain
 * `<button>` with no `aria-haspopup`, no confirmation step, and an
 * undocumented `/` destination. A demo-recording click could silently
 * blow the operator out of the wizard mid-take. This spec asserts:
 *
 * - Label reads "Exit setup" (not "SAVE & EXIT").
 * - `aria-haspopup="dialog"` is set so screen readers announce the
 *   confirm popup.
 * - Clicking opens an `alertdialog` with both Stay and Save & exit
 *   buttons; clicking Stay keeps the URL on the wizard step.
 * - Clicking Save & exit on the unprovisioned welcome step routes to
 *   `/landing` (public home — the operator hasn't tenanted yet).
 * - Clicking Save & exit after a successful workspace provision routes
 *   to `/workers` (the dashboard for the now-tenanted operator).
 * - The wizard draft survives in localStorage so revisiting the wizard
 *   resumes mid-flow rather than starting over.
 *
 * Brief specified path `web/playwright/onboarding/...`; the project's
 * actual playwright testDir is `./tests/e2e`, so the spec lives here
 * (sensible deviation noted in PR body).
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}

test.describe("Onboarding top-bar Exit setup button", () => {
  test("Welcome step exit-confirm: label/ARIA, cancel stays, confirm → /landing", async ({
    page,
  }) => {
    // Welcome step doesn't hit the gateway, but the AppShell may. Stub
    // auth so the page renders deterministically.
    await page.route("**/api/auth/session", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          gatewayToken: "stub",
          principal: "human:exit@example.test",
        }),
      }),
    );

    // Routing `/landing` is the public marketing home — intercept so the
    // playwright nav doesn't require a real marketing-site backend.
    await page.route("**/landing", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "text/html",
        body: "<!doctype html><title>landing</title>",
      }),
    );

    await page.goto(appUrl("/onboarding/welcome"));

    const exitBtn = page.getByTestId("wiz-shell-exit");
    await expect(exitBtn).toHaveText("Exit setup");
    await expect(exitBtn).toHaveAttribute("aria-haspopup", "dialog");

    // Clicking opens the confirm dialog rather than navigating away.
    await exitBtn.click();
    const dialog = page.getByTestId("alert-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Exit setup and save your progress?");

    // Cancel keeps the URL on the wizard step.
    await page.getByTestId("alert-dialog-cancel").click();
    await expect(dialog).not.toBeVisible();
    await expect(page).toHaveURL(/\/onboarding\/welcome/);

    // Re-open and confirm → routes to /landing because the workspace
    // hasn't been provisioned yet (no tenantId in draft).
    await exitBtn.click();
    await page.getByTestId("alert-dialog-confirm").click();
    await expect(page).toHaveURL(/\/landing$/, { timeout: 5_000 });
  });

  test("After a workspace provision the Exit setup destination becomes /workers", async ({
    page,
  }) => {
    await page.route("**/api/auth/session", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          gatewayToken: "stub",
          principal: "human:exitprov@example.test",
        }),
      }),
    );
    await page.route("**/api/v1/auth/csrf", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ token: "csrf-stub" }),
      }),
    );
    await page.route("**/api/v1/onboarding/provision", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tenant_id: "tenant-exit",
          workspace_id: "ws-exit",
          created: true,
        }),
      }),
    );
    await page.route("**/workers", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "text/html",
        body: "<!doctype html><title>workers</title>",
      }),
    );

    await page.goto(appUrl("/onboarding/workspace"));
    await page.getByTestId("workspace-name").fill("Exit Corp");
    await page.getByTestId("company-name").fill("Exit Inc.");
    await page.getByTestId("onboarding-workspace-submit").click();

    // The provision succeeded — draft now carries tenantId+workspaceId
    // so the Exit setup destination should flip from /landing to
    // /workers. Trigger the exit from the workspace step (we haven't
    // navigated away yet).
    await page.getByTestId("wiz-shell-exit").click();
    await expect(page.getByTestId("alert-dialog")).toBeVisible();
    await page.getByTestId("alert-dialog-confirm").click();
    await expect(page).toHaveURL(/\/workers$/, { timeout: 5_000 });
  });

  test("Draft survives an Exit setup round-trip", async ({ page }) => {
    await page.route("**/api/auth/session", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          gatewayToken: "stub",
          principal: "human:draft@example.test",
        }),
      }),
    );
    await page.route("**/landing", async (route) =>
      route.fulfill({
        status: 200,
        contentType: "text/html",
        body: "<!doctype html><title>landing</title>",
      }),
    );

    await page.goto(appUrl("/onboarding/workspace"));
    await page.getByTestId("workspace-name").fill("Resume Co");

    // Exit through the confirm modal.
    await page.getByTestId("wiz-shell-exit").click();
    await page.getByTestId("alert-dialog-confirm").click();
    await expect(page).toHaveURL(/\/landing$/, { timeout: 5_000 });

    // Open the wizard step again — the draft should rehydrate from
    // localStorage and the workspace name field should be pre-filled.
    await page.goto(appUrl("/onboarding/workspace"));
    await expect(page.getByTestId("workspace-name")).toHaveValue("Resume Co");
  });
});
