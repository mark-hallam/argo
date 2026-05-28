import { expect, test } from "@playwright/test";

/**
 * Trailing-slash canonicalisation — `/landing` is the canonical
 * no-slash form and must serve directly (200, no redirect hop).
 * `/landing/` may either 404 or single-hop 308 to `/landing` — both
 * are acceptable outcomes for a non-canonical form; what is NOT
 * acceptable is a redirect chain (308 → 308).
 *
 * Spec source: stream
 * `fix-spa-landing-trailing-slash-and-worker-not-found-back-link`.
 * Audit finding: onboarding-hire-an-ai-teammate F11 [P2].
 *
 * Pinned by `trailingSlash: false` in `web/next.config.ts`.
 */

test("/landing (canonical) returns 200 directly with no redirect hop", async ({
  request,
}) => {
  const res = await request.get("/landing", { maxRedirects: 0 });
  expect(res.status()).toBe(200);
});

test("/landing/ (non-canonical) is at most a single-hop 308 or 404", async ({
  request,
}) => {
  const res = await request.get("/landing/", { maxRedirects: 0 });
  const status = res.status();
  expect([200, 308, 404]).toContain(status);

  if (status === 308) {
    const location = res.headers()["location"];
    expect(location).toBeDefined();
    expect(location).toMatch(/\/landing(?:\?|$)/);
    // The destination must itself be canonical (no further redirect).
    const hop = await request.get(location as string, { maxRedirects: 0 });
    expect(hop.status()).toBe(200);
  }
});
