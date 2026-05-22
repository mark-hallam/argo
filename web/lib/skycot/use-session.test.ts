import { afterEach, describe, expect, it, vi } from "vitest";
import { DEMO_SESSION, deriveSession } from "./use-session";

/**
 * Helper to mint a JWT body. Signature segment is irrelevant — the
 * SPA never verifies the signature, the gateway does. Header and
 * signature get junk values so the three-segment split keeps working.
 */
function mintToken(body: Record<string, unknown>): string {
  const b64 = (s: string) =>
    Buffer.from(s, "utf8").toString("base64").replace(/=+$/, "").replace(/\+/g, "-").replace(/\//g, "_");
  return ["x", b64(JSON.stringify(body)), "y"].join(".");
}

describe("deriveSession", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("returns the seeded demo identity when demo mode is on", () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    const result = deriveSession(
      { gatewayToken: mintToken({ email: "real@buyer.com", tnt: "tenant_other" }) },
      "authenticated",
    );
    expect(result).toEqual(DEMO_SESSION);
  });

  it("decodes the JWT body for email + tenant + role on the live path", () => {
    const token = mintToken({
      sub: "user_42",
      email: "ada@example.com",
      tnt: "tenant_atlas",
      role: "admin",
    });
    const result = deriveSession({ gatewayToken: token }, "authenticated");
    expect(result.email).toBe("ada@example.com");
    expect(result.tenantId).toBe("tenant_atlas");
    expect(result.role).toBe("admin");
    expect(result.workspaceLabel).toBe("Atlas · Admin");
    expect(result.isDemoMode).toBe(false);
  });

  it("falls back to the email local-part when no name claim is present", () => {
    const token = mintToken({ email: "jane.doe@example.com" });
    const result = deriveSession({ gatewayToken: token }, "authenticated");
    expect(result.displayName).toBe("jane.doe");
  });

  it("prefers the explicit name claim over the email local-part", () => {
    const token = mintToken({ email: "j@x.com", name: "Jane Doe" });
    const result = deriveSession({ gatewayToken: token }, "authenticated");
    expect(result.displayName).toBe("Jane Doe");
  });

  it("uses 'Guest' + 'Personal workspace' for an unauthenticated session", () => {
    const result = deriveSession(null, "unauthenticated");
    expect(result.displayName).toBe("Guest");
    expect(result.workspaceLabel).toBe("Personal workspace");
    expect(result.email).toBeNull();
    expect(result.tenantId).toBeNull();
    expect(result.role).toBeNull();
  });

  it("survives a malformed JWT without throwing", () => {
    const result = deriveSession({ gatewayToken: "not.a.jwt" }, "authenticated");
    expect(result.displayName).toBe("Guest");
    expect(result.email).toBeNull();
  });
});
