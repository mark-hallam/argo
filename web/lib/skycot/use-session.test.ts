import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEMO_SESSION_FALLBACK,
  deriveSession,
  getDemoSession,
} from "./use-session";

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

  it("returns the neutral demo fallback when demo mode is on and draft is empty", () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    const result = deriveSession(
      { gatewayToken: mintToken({ email: "real@buyer.com", tnt: "tenant_other" }) },
      "authenticated",
    );
    expect(result).toEqual(DEMO_SESSION_FALLBACK);
    // The pre-step-2 fallback must never carry the old hardcoded
    // "Acmeco · Owner" / "Sam Harlow" identity.
    expect(result.workspaceLabel).not.toMatch(/Acmeco/i);
    expect(result.displayName).not.toMatch(/Sam Harlow/i);
  });

  it("derives demo session from onboarding draft when workspace name is typed", () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    const result = deriveSession(
      null,
      "authenticated",
      { workspaceName: "Acme Growth", tenantId: null },
    );
    expect(result.workspaceLabel).toBe("Acme Growth · Owner");
    expect(result.email).toBe("acme-growth@demo.skycot.local");
    expect(result.tenantId).toBe("tenant_acme-growth");
    expect(result.isDemoMode).toBe(true);
    expect(result.role).toBe("owner");
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

describe("getDemoSession", () => {
  it("returns the neutral fallback for a null draft", () => {
    expect(getDemoSession(null)).toEqual(DEMO_SESSION_FALLBACK);
  });

  it("returns the neutral fallback for a whitespace-only workspace name", () => {
    expect(getDemoSession({ workspaceName: "   ", tenantId: null })).toEqual(
      DEMO_SESSION_FALLBACK,
    );
  });

  it("prefers the draft-provisioned tenant id over the derived one", () => {
    const result = getDemoSession({
      workspaceName: "Acme Growth",
      tenantId: "demo-tenant-acme-growth",
    });
    expect(result.tenantId).toBe("demo-tenant-acme-growth");
    expect(result.workspaceLabel).toBe("Acme Growth · Owner");
  });

  it("slugifies the workspace name into a valid email local-part", () => {
    const result = getDemoSession({
      workspaceName: "Acme Co · Sales!",
      tenantId: null,
    });
    expect(result.email).toBe("acme-co-sales@demo.skycot.local");
    expect(result.tenantId).toBe("tenant_acme-co-sales");
    expect(result.workspaceLabel).toBe("Acme Co · Sales! · Owner");
  });

  it("never re-introduces the old 'Acmeco · Owner' hardcoded label", () => {
    const result = getDemoSession({
      workspaceName: "Acmeco Growth",
      tenantId: null,
    });
    expect(result.workspaceLabel).toBe("Acmeco Growth · Owner");
    expect(result.workspaceLabel).not.toBe("Acmeco · Owner");
  });
});
