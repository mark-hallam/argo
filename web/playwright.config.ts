import { defineConfig } from "@playwright/test";

/**
 * Playwright config for SKYCOT web.
 *
 * Spawns `pnpm dev` on demand (reused if already running locally), and
 * runs specs in `tests/e2e/`. Specs that exercise the mock-fallback
 * path point `NEXT_PUBLIC_GATEWAY_URL` at a non-existent port so the
 * gateway client surfaces a `NetworkError`/404 and the approvals API
 * falls back to the bundled sample data deterministically.
 */
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
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    env: {
      NEXT_PUBLIC_GATEWAY_AUTH_BYPASS: "1",
      NEXT_PUBLIC_GATEWAY_URL: "http://127.0.0.1:1",
    },
    timeout: 120_000,
  },
});
