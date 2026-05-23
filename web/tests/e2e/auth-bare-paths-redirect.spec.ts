import { expect, test } from "@playwright/test";

/**
 * Bare auth paths (`/signup`, `/signin`, `/verify`) must 308-redirect
 * to the canonical `/auth/*` routes. Configured in `web/next.config.ts`
 * `redirects()`. Spec source: stream `web-redirect-bare-auth-paths`.
 *
 * Uses `request.get(..., { maxRedirects: 0 })` to inspect the raw
 * redirect response — `page.goto()` would follow it transparently.
 */

const CASES = [
  { source: "/signup", target: "/auth/signup" },
  { source: "/signin", target: "/auth/signin" },
  { source: "/verify", target: "/auth/verify" },
] as const;

for (const { source, target } of CASES) {
  test(`bare path ${source} 308-redirects to ${target}`, async ({ request }) => {
    const res = await request.get(source, { maxRedirects: 0 });
    expect(res.status()).toBe(308);
    const location = res.headers()["location"];
    expect(location).toBeDefined();
    expect(location).toBe(target);
  });
}
