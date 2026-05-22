/**
 * Typed `fetch` wrapper for the SKYCOT gateway.
 *
 * Conventions:
 * - URLs are paths (`/api/v1/...`); the base is `NEXT_PUBLIC_GATEWAY_URL`.
 * - Bearer token comes from the NextAuth session on the server; on the
 *   client we read the cached session via `getBearerToken()` below.
 * - Non-2xx → `ApiError`. 4xx bodies are parsed as the Rust gateway's
 *   `{ error: { code, message } }` envelope; 5xx / parse failures keep
 *   the raw body.
 * - 204 No Content returns `undefined as T`.
 */

import { ApiError, type ApiErrorEnvelope } from "./types";
import { resolveCsrfToken } from "@/lib/auth/csrf-token";
import {
  buildSigninUrlWithReturnTo,
  captureCurrentLocationAsReturnTo,
} from "@/lib/auth/return-to";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";
const MUTATING_METHODS = new Set(["POST", "PATCH", "DELETE", "PUT"]);

/**
 * Session-expired 401 detection. The gateway's session-cookie verify
 * middleware returns 401 with body `{"error":"session expired"}`,
 * `{"error":"session invalid"}`, `{"error":"session revoked"}`, or
 * the SSO-session shape `{"error":{"code":"session_expired", ...}}`
 * when the user's JWT cookie is no longer valid. RBAC denials
 * (operator lacks a capability) come back as 403 with
 * `{"error":{"code":"forbidden", ...}}` and never hit this path.
 *
 * Bearer-auth failures (`{"error":"unauthorized","reason":"..."}`)
 * are the dev-bearer-token path and intentionally bypass the
 * redirect — those users should NOT be bounced to the magic-link
 * signin surface.
 */
const SESSION_END_RE = /session\s+(expired|invalid|revoked)/i;
const SESSION_END_CODES = new Set([
  "session_expired",
  "session_invalid",
  "session_revoked",
]);

function isSessionEndedBody(rawBody: string): boolean {
  if (!rawBody) return false;
  let parsed: unknown;
  try {
    parsed = JSON.parse(rawBody);
  } catch {
    return false;
  }
  if (typeof parsed !== "object" || parsed === null) return false;
  const errVal = (parsed as { error?: unknown }).error;
  if (typeof errVal === "string") {
    return SESSION_END_RE.test(errVal);
  }
  if (typeof errVal === "object" && errVal !== null) {
    const code = (errVal as { code?: unknown }).code;
    if (typeof code === "string" && SESSION_END_CODES.has(code)) return true;
  }
  return false;
}

/**
 * Browser-only redirect to `/auth/signin?returnTo=<current path>`.
 * No-ops under SSR/RSC and when already on an `/auth/*` surface
 * (the user is mid-signin; another bounce would be useless and
 * could loop). Uses `location.assign` rather than the Next.js
 * router because the API client module has no router context.
 */
function redirectToSignin(): void {
  if (typeof window === "undefined") return;
  if (window.location.pathname.startsWith("/auth/")) return;
  const current = captureCurrentLocationAsReturnTo();
  const target = buildSigninUrlWithReturnTo(current);
  window.location.assign(target);
}

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

export interface ApiRequestInit extends Omit<RequestInit, "body"> {
  /** Bearer token override. Defaults to the active session's token. */
  token?: string;
  /** JSON body; serialised automatically. */
  json?: unknown;
}

async function readSessionToken(): Promise<string | undefined> {
  if (typeof window === "undefined") {
    const { auth } = await import("@/auth");
    const session = await auth();
    return session?.gatewayToken;
  }
  try {
    const res = await fetch("/api/auth/session", { credentials: "same-origin" });
    if (!res.ok) return undefined;
    const session = (await res.json()) as { gatewayToken?: string };
    return session?.gatewayToken;
  } catch {
    return undefined;
  }
}

async function parseError(res: Response): Promise<ApiError> {
  const text = await res.text().catch(() => "");
  // Session-ended on a cookie-authenticated request → fire the
  // returnTo redirect before surfacing the ApiError to the call
  // site. The throw still happens so in-flight UI can clean up; the
  // redirect navigates the document away in parallel.
  if (res.status === 401 && isSessionEndedBody(text)) {
    redirectToSignin();
    return new ApiError(res.status, "Session expired", "session_expired", text || undefined);
  }
  if (res.status >= 400 && res.status < 500 && text) {
    try {
      const parsed = JSON.parse(text) as Partial<ApiErrorEnvelope>;
      if (parsed?.error?.code || parsed?.error?.message) {
        return new ApiError(
          res.status,
          parsed.error.message ?? res.statusText,
          parsed.error.code,
          parsed,
        );
      }
    } catch {
      /* fall through */
    }
  }
  return new ApiError(res.status, res.statusText || `HTTP ${res.status}`, undefined, text || undefined);
}

async function request<T>(method: string, path: string, init: ApiRequestInit = {}): Promise<T> {
  const { token: tokenOverride, json, headers, ...rest } = init;
  const token = tokenOverride ?? (await readSessionToken());

  const finalHeaders = new Headers(headers);
  if (json !== undefined) finalHeaders.set("Content-Type", "application/json");
  if (token) finalHeaders.set("Authorization", `Bearer ${token}`);
  finalHeaders.set("Accept", "application/json");
  // CSRF double-submit (Stream claude-rebase-pr-1835-cookies). The
  // gateway's `middleware_csrf` enforces the header on every
  // cookie-authenticated mutating request; attaching it
  // unconditionally on mutating verbs is a no-op for bearer-auth
  // requests (no `__Host-session` cookie → gate skips) but covers
  // the cookie-auth surface without further client-side changes.
  if (MUTATING_METHODS.has(method.toUpperCase()) && !finalHeaders.has("X-CSRF-Token")) {
    const csrf = await resolveCsrfToken();
    if (csrf) finalHeaders.set("X-CSRF-Token", csrf);
  }

  const url = path.startsWith("http") ? path : `${gatewayBase()}${path}`;
  const res = await fetch(url, {
    ...rest,
    method,
    headers: finalHeaders,
    body: json !== undefined ? JSON.stringify(json) : undefined,
  });

  if (!res.ok) throw await parseError(res);
  if (res.status === 204) return undefined as T;

  const ct = res.headers.get("content-type") ?? "";
  if (ct.includes("application/json")) return (await res.json()) as T;
  return (await res.text()) as unknown as T;
}

export function apiGet<T>(path: string, init?: ApiRequestInit): Promise<T> {
  return request<T>("GET", path, init);
}

export function apiPost<T, B = unknown>(path: string, body?: B, init?: ApiRequestInit): Promise<T> {
  return request<T>("POST", path, { ...init, json: body });
}

export function apiPatch<T, B = unknown>(path: string, body?: B, init?: ApiRequestInit): Promise<T> {
  return request<T>("PATCH", path, { ...init, json: body });
}

export function apiDelete<T>(path: string, init?: ApiRequestInit): Promise<T> {
  return request<T>("DELETE", path, init);
}

export { ApiError } from "./types";
