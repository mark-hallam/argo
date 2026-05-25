import { defineConfig } from "@playwright/test";

/**
 * Playwright config for SKYCOT web.
 *
 * Two lanes share this config:
 *
 * 1. **Local lane** (default, `ARGO_E2E_TARGET` unset or `local`).
 *    Spawns the SPA on port 3000 + (unless `SKIP_MARKETING_SERVER=1`)
 *    the marketing site on port 3001 on demand. Specs in
 *    `tests/e2e/*.spec.ts` exercise the mock-fallback path by pointing
 *    `NEXT_PUBLIC_GATEWAY_URL` at a non-existent port so the gateway
 *    client surfaces a `NetworkError`/404 and the approvals API falls
 *    back to bundled sample data deterministically; the onboarding
 *    spec instead intercepts gateway calls via `page.route()`.
 *
 * 2. **Deployed-real lane** (`ARGO_E2E_TARGET=deploy`). Specs in
 *    `tests/e2e/deployed/*.spec.ts` run against a real gateway / SPA /
 *    marketing trio. No Next.js servers are spun up — `baseURL`
 *    becomes the deployed SPA URL, retries are bumped (network flakes
 *    are real, not mocks), and the `webServer` block is dropped.
 *    See `docs/skycot/operator/deployed-e2e.md` for the runbook.
 */
const deployMode = process.env.ARGO_E2E_TARGET === "deploy";
const skipMarketing = process.env.SKIP_MARKETING_SERVER === "1";
const spaUrl = process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL;

export default defineConfig({
  testDir: "./tests/e2e",
  // Deployed lane only runs specs explicitly opted in under `deployed/`;
  // local lane runs the rest. Each lane ignores the other's files so
  // `pnpm test:e2e` and `pnpm test:e2e:deploy` are clean disjoint sets.
  testIgnore: deployMode ? ["**/tests/e2e/*.spec.ts"] : ["**/tests/e2e/deployed/**"],
  fullyParallel: !deployMode,
  forbidOnly: !!process.env.CI,
  retries: deployMode ? 2 : process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "list" : "line",
  use: {
    baseURL: deployMode ? (spaUrl ?? "https://app.skycot.com") : "http://localhost:3000",
    trace: "on-first-retry",
  },
  webServer: deployMode
    ? undefined
    : [
        {
          command: "pnpm dev",
          url: "http://localhost:3000",
          reuseExistingServer: !process.env.CI,
          env: {
            NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS: "1",
            NEXT_PUBLIC_GATEWAY_URL: "http://127.0.0.1:1",
          },
          timeout: 120_000,
        },
        ...(skipMarketing
          ? []
          : [
              {
                command: "pnpm --dir ../marketing dev",
                url: "http://localhost:3001",
                reuseExistingServer: !process.env.CI,
                env: { NEXT_PUBLIC_APP_URL: "http://localhost:3000" },
                timeout: 120_000,
              },
            ]),
      ],
});
