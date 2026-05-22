import { describe, expect, it, vi } from "vitest";

/**
 * `/onboarding` (bare URL) must land users on the welcome step instead
 * of 404ing. Before this fix `web/app/(setup)/onboarding/` had no
 * `page.tsx`, so every link that dropped the `/welcome` suffix —
 * magic-link verify redirects, `safeReturnToOrHome` (which allowlists
 * the bare `/onboarding` path), shared invite URLs — silently 404'd.
 *
 * Companion to the click-through coverage in `./wizard.test.tsx`: that
 * file exercises the 5-step flow once a user is inside it; this file
 * pins the entry-point redirect.
 */

const redirectMock = vi.fn((url: string) => {
  throw new Error(`NEXT_REDIRECT:${url}`);
});

vi.mock("next/navigation", () => ({
  redirect: redirectMock,
}));

describe("/onboarding index redirect", () => {
  it("redirects bare `/onboarding` to `/onboarding/welcome`", async () => {
    const mod = await import("@/app/(setup)/onboarding/page");
    expect(() => mod.default()).toThrow(/NEXT_REDIRECT:\/onboarding\/welcome/);
    expect(redirectMock).toHaveBeenCalledWith("/onboarding/welcome");
  });
});
