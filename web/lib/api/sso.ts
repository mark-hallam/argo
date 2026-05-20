/**
 * Client helpers for enterprise SSO — OAuth (Google / GitHub / Microsoft)
 * + SAML 2.0 (Okta / Azure AD / custom).
 *
 * Contract (gateway side, separate brief):
 *  - GET  /api/v1/auth/sso/discover?email=<email>     → SsoDiscoveryResult
 *  - GET  /api/v1/auth/oauth/<provider>/start         → 302 to IdP (no JSON)
 *  - POST /api/v1/auth/oauth/<provider>/callback      → { needs_onboarding, tenant_id }
 *                                                       + Set-Cookie session
 *  - GET  /api/v1/auth/saml/config                    → SamlConfig | null
 *  - POST /api/v1/auth/saml/config                    → SamlConfig (upsert)
 *  - POST /api/v1/auth/saml/config/test               → { ok, message }
 *  - GET  /api/v1/auth/saml/metadata                  → XML
 *
 * For OAuth start we navigate via `window.location` because the gateway
 * must set the OAuth state cookie on its own origin before redirecting
 * to the IdP. The callback exchange is proxied through Next API
 * (`/api/auth/oauth/<provider>/callback`) so the resulting session
 * cookie lands on the SPA origin — same pattern as `/api/auth/verify`.
 */

import { apiGet, apiPost } from "./client";

export type OAuthProvider = "google" | "github" | "microsoft";

export const OAUTH_PROVIDERS: OAuthProvider[] = ["google", "github", "microsoft"];

export type SsoDiscoveryMode = "magic_link" | "oauth" | "saml";

export interface SsoDiscoveryResult {
  /** What sign-in flow this email's workspace requires. */
  mode: SsoDiscoveryMode;
  /** When `mode === "oauth"`, which provider the workspace prefers. */
  provider?: OAuthProvider;
  /** When `mode === "saml"`, the IdP-initiated SP URL to navigate to. */
  ssoStartUrl?: string;
  /** Workspace id if matched by domain — useful for telemetry. */
  workspaceId?: string;
}

export interface SamlConfig {
  /** IdP entity ID — e.g. `http://www.okta.com/abc123`. */
  idpEntityId: string;
  /** SSO endpoint URL on the IdP — where SAML AuthnRequests go. */
  ssoUrl: string;
  /** PEM-encoded X.509 signing certificate (multi-line). */
  certificate: string;
  /** Friendly label: `okta`, `azure`, or `custom`. */
  flavor: "okta" | "azure" | "custom";
  /** Workspace toggle: allow owners to bypass SSO with magic-link. */
  allowOwnerBypass: boolean;
  /** Optional JSON map of IdP attribute → SKYCOT field (role, cohort). */
  attributeMappingJson?: string;
}

export interface SamlConfigStatus {
  configured: boolean;
  config: SamlConfig | null;
}

export interface SamlTestResult {
  ok: boolean;
  message: string;
}

export interface OAuthCallbackResult {
  /** True if the gateway created a brand-new user during this callback. */
  needs_onboarding: boolean;
  /** Tenant id of the signed-in workspace. */
  tenant_id: string | null;
}

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  if (typeof process !== "undefined" && process.env?.NEXT_PUBLIC_GATEWAY_URL) {
    return process.env.NEXT_PUBLIC_GATEWAY_URL;
  }
  return DEFAULT_GATEWAY;
}

/**
 * Compute the OAuth start URL for a given provider.
 *
 * The gateway is the OAuth client; it must own the state cookie before
 * redirecting to the IdP, so we navigate by full origin (not via the
 * Next API proxy). `redirect_to` is forwarded so the callback page can
 * route the user back to their original destination after sign-in.
 */
export function oauthStartUrl(provider: OAuthProvider, redirectTo?: string): string {
  const url = new URL(`/api/v1/auth/oauth/${provider}/start`, gatewayBase());
  if (redirectTo) url.searchParams.set("redirect_to", redirectTo);
  return url.toString();
}

/** Navigate to the OAuth start URL — typed wrapper around `window.location`. */
export function startOAuth(provider: OAuthProvider, redirectTo?: string): void {
  if (typeof window === "undefined") return;
  window.location.href = oauthStartUrl(provider, redirectTo);
}

/**
 * Probe the gateway for the SSO setting for an email's workspace.
 *
 * Returns `mode: "magic_link"` for unknown / unmatched emails — the
 * caller should then fall through to the standard signup flow. Gateway
 * 404s are treated as "no SSO configured" rather than errors.
 */
export async function discoverSso(email: string): Promise<SsoDiscoveryResult> {
  const params = new URLSearchParams({ email });
  try {
    return await apiGet<SsoDiscoveryResult>(
      `/api/v1/auth/sso/discover?${params.toString()}`,
    );
  } catch (err) {
    if (isNotFound(err)) {
      return { mode: "magic_link" };
    }
    throw err;
  }
}

/**
 * Exchange the OAuth `code` for a session cookie via the Next API
 * proxy. Mirrors the magic-link verify flow: the proxy forwards
 * upstream Set-Cookie so the session cookie lands on the SPA origin.
 */
export async function completeOAuth(
  provider: OAuthProvider,
  code: string,
  state: string,
): Promise<OAuthCallbackResult> {
  const res = await fetch(`/api/auth/oauth/${provider}/callback`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ code, state }),
    cache: "no-store",
    credentials: "same-origin",
  });
  if (!res.ok) {
    const body = (await res.json().catch(() => ({}))) as {
      error?: { message?: string };
    };
    throw new Error(body.error?.message ?? `OAuth callback failed (${res.status})`);
  }
  return (await res.json()) as OAuthCallbackResult;
}

export async function getSamlConfig(): Promise<SamlConfigStatus> {
  try {
    const config = await apiGet<SamlConfig>("/api/v1/auth/saml/config");
    return { configured: true, config };
  } catch (err) {
    if (isNotFound(err)) {
      return { configured: false, config: null };
    }
    throw err;
  }
}

export function saveSamlConfig(input: SamlConfig): Promise<SamlConfig> {
  return apiPost<SamlConfig, SamlConfig>("/api/v1/auth/saml/config", input);
}

export function testSamlConnection(input: SamlConfig): Promise<SamlTestResult> {
  return apiPost<SamlTestResult, SamlConfig>(
    "/api/v1/auth/saml/config/test",
    input,
  );
}

/**
 * URL to the SP (service-provider) metadata XML. The settings UI
 * exposes this as a download link the operator hands to their IdP
 * administrator.
 */
export function samlMetadataUrl(): string {
  return new URL("/api/v1/auth/saml/metadata", gatewayBase()).toString();
}

function isNotFound(err: unknown): boolean {
  return (
    typeof err === "object" &&
    err !== null &&
    "status" in err &&
    (err as { status?: number }).status === 404
  );
}
