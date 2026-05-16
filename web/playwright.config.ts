import { defineConfig } from "@playwright/test";

/**
 * Playwright config for SKYCOT web.
 *
 * Spawns the SPA on port 3000 + (unless `SKIP_MARKETING_SERVER=1`) the
 * marketing site on port 3001 on demand. Most specs in `tests/e2e/`
 * exercise the mock-fallback path by pointing `NEXT_PUBLIC_GATEWAY_URL`
 * at a non-existent port so the gateway client surfaces a
 * `NetworkError`/404 and the approvals API falls back to the bundled
 * sample data deterministically; the onboarding spec instead intercepts
 * gateway calls via `page.route()`.
 */
const skipMarketing = process.env.SKIP_MARKETING_SERVER === "1";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "list" : "line",
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },
  webServer: [
    {
      command: "pnpm dev",
      url: "http://localhost:3000",
      reuseExistingServer: !process.env.CI,
      env: {
        NEXT_PUBLIC_GATEWAY_AUTH_BYPASS: "1",
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
