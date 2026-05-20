import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  completeOAuth,
  discoverSso,
  oauthStartUrl,
  samlMetadataUrl,
  saveSamlConfig,
  testSamlConnection,
  getSamlConfig,
  OAUTH_PROVIDERS,
  type SamlConfig,
} from "./sso";

const ORIGINAL_FETCH = globalThis.fetch;

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function installMock(responder: (req: Request) => Response | Promise<Response>) {
  const fn = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(jsonResponse(200, {}));
    }
    return Promise.resolve(responder(new Request(url, init)));
  });
  globalThis.fetch = fn as unknown as typeof fetch;
  return fn;
}

const SAMPLE_CONFIG: SamlConfig = {
  idpEntityId: "http://www.okta.com/abc",
  ssoUrl: "https://example.okta.com/sso/saml",
  certificate: "-----BEGIN CERTIFICATE-----\nMIID\n-----END CERTIFICATE-----",
  flavor: "okta",
  allowOwnerBypass: true,
};

describe("lib/api/sso", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.restoreAllMocks();
  });

  it("constructs the OAuth start URL with redirect_to", () => {
    expect(oauthStartUrl("google", "/dashboard")).toBe(
      "http://gw.test/api/v1/auth/oauth/google/start?redirect_to=%2Fdashboard",
    );
    expect(oauthStartUrl("github")).toBe(
      "http://gw.test/api/v1/auth/oauth/github/start",
    );
  });

  it("exposes a stable provider list", () => {
    expect(OAUTH_PROVIDERS).toEqual(["google", "github", "microsoft"]);
  });

  it("returns the SP metadata URL on the gateway origin", () => {
    expect(samlMetadataUrl()).toBe("http://gw.test/api/v1/auth/saml/metadata");
  });

  it("discoverSso returns the gateway payload on 2xx", async () => {
    installMock(() =>
      jsonResponse(200, { mode: "oauth", provider: "google", workspaceId: "ws-1" }),
    );
    const result = await discoverSso("a@b.com");
    expect(result).toEqual({ mode: "oauth", provider: "google", workspaceId: "ws-1" });
  });

  it("discoverSso falls back to magic_link on 404", async () => {
    installMock(() =>
      jsonResponse(404, { error: { code: "not_found", message: "no sso" } }),
    );
    const result = await discoverSso("a@b.com");
    expect(result).toEqual({ mode: "magic_link" });
  });

  it("completeOAuth POSTs to the Next API proxy and unwraps the body", async () => {
    const fn = installMock((req) => {
      expect(req.url).toContain("/api/auth/oauth/google/callback");
      return jsonResponse(200, { ok: true, needs_onboarding: true, tenant_id: "t-9" });
    });
    const result = await completeOAuth("google", "code-1", "state-1");
    expect(result.needs_onboarding).toBe(true);
    expect(result.tenant_id).toBe("t-9");
    expect(fn).toHaveBeenCalled();
  });

  it("completeOAuth throws on non-2xx", async () => {
    installMock(() =>
      jsonResponse(400, { error: { code: "bad_state", message: "state mismatch" } }),
    );
    await expect(completeOAuth("google", "c", "s")).rejects.toThrow(/state mismatch/);
  });

  it("getSamlConfig returns not-configured on 404", async () => {
    installMock(() =>
      jsonResponse(404, { error: { code: "not_found", message: "no saml" } }),
    );
    const result = await getSamlConfig();
    expect(result.configured).toBe(false);
    expect(result.config).toBeNull();
  });

  it("getSamlConfig returns configured on 2xx", async () => {
    installMock(() => jsonResponse(200, SAMPLE_CONFIG));
    const result = await getSamlConfig();
    expect(result.configured).toBe(true);
    expect(result.config?.flavor).toBe("okta");
  });

  it("saveSamlConfig POSTs the draft and returns the saved record", async () => {
    installMock(() => jsonResponse(200, SAMPLE_CONFIG));
    const saved = await saveSamlConfig(SAMPLE_CONFIG);
    expect(saved.idpEntityId).toBe(SAMPLE_CONFIG.idpEntityId);
  });

  it("testSamlConnection returns the gateway verdict", async () => {
    installMock(() =>
      jsonResponse(200, { ok: true, message: "Reached IdP, certificate valid." }),
    );
    const result = await testSamlConnection(SAMPLE_CONFIG);
    expect(result.ok).toBe(true);
    expect(result.message).toMatch(/IdP/);
  });
});
