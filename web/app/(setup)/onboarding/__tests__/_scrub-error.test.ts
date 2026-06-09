import { describe, expect, it } from "vitest";
import { scrubOnboardingErrorMessage } from "@/app/(setup)/onboarding/_scrub-error";

/**
 * sprint-ws1-canonicalize-loopback-host-and-fix-demo-mode-error.
 *
 * The scrubber detects the gateway's auth-reject leak (`X-Argo-Principal`
 * / `principal`) and substitutes user-facing copy. The substitute used to
 * claim the user was "in demo mode" — which was FALSE: the real cause is a
 * dropped `__Host-`-pinned session (e.g. a non-canonical loopback host),
 * so the honest message is that the session expired and they should sign
 * in again. These pin the honest copy + the unchanged leak-detection.
 */
describe("scrubOnboardingErrorMessage — honest re-auth copy", () => {
  const REAUTH = /sign in again|session expired/i;

  it("scrubs an X-Argo-Principal leak to honest re-auth copy (not demo mode)", () => {
    const result = scrubOnboardingErrorMessage(
      new Error("X-Argo-Principal header must carry a verified human:<email> value"),
    );
    expect(result.wasProtocolLeak).toBe(true);
    expect(result.message.toLowerCase()).not.toContain("demo mode");
    expect(result.message).not.toContain("X-Argo-Principal");
    expect(result.message).not.toContain("principal");
    expect(result.message).toMatch(REAUTH);
  });

  it("scrubs a bare 'principal' leak to honest re-auth copy", () => {
    const result = scrubOnboardingErrorMessage(new Error("missing principal"));
    expect(result.wasProtocolLeak).toBe(true);
    expect(result.message.toLowerCase()).not.toContain("demo mode");
    expect(result.message).toMatch(REAUTH);
  });

  it("passes a benign message through unchanged", () => {
    const result = scrubOnboardingErrorMessage(new Error("Workspace name is required"));
    expect(result.wasProtocolLeak).toBe(false);
    expect(result.message).toBe("Workspace name is required");
  });

  it("returns generic-failure copy for empty / non-Error input", () => {
    const empty = scrubOnboardingErrorMessage("");
    expect(empty.wasProtocolLeak).toBe(false);
    expect(empty.message.length).toBeGreaterThan(0);
    expect(empty.message.toLowerCase()).not.toContain("demo mode");

    const nullish = scrubOnboardingErrorMessage(null);
    expect(nullish.wasProtocolLeak).toBe(false);
    expect(nullish.message.length).toBeGreaterThan(0);
  });
});
