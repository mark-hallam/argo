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

import { ApiError, GatewayTimeoutError, type ApiErrorEnvelope } from "./types";
import { resolveTenantId } from "./tenant";
import { resolveCsrfToken, fetchCsrfTokenViaXhr } from "@/lib/auth/csrf-token";
import {
  buildSigninUrlWithReturnTo,
  captureCurrentLocationAsReturnTo,
} from "@/lib/auth/return-to";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

const MUTATING_METHODS = new Set(["POST", "PATCH", "DELETE", "PUT"]);

/**
 * Default timeout budgets (milliseconds). These are the universal
 * backstop that makes a perpetual loading skeleton structurally
 * impossible — every read is guaranteed to settle within the budget,
 * even against a gateway that accepts the socket but never replies.
 *
 * - READ (GET/HEAD): 15s. Comfortably above a healthy gateway's
 *   sub-second answer, so it only fires on a genuine stall; tight enough
 *   that a wedged socket can't strand a page forever. Per-call override
 *   via `ApiRequestInit.timeoutMs` (e.g. `/team` uses a snappier 5s).
 * - MUTATION (POST/PATCH/PUT/DELETE): 0 = UNBOUNDED by default.
 *   Mutations can legitimately run long (Genesis generation, a workflow
 *   execution), and they surface a button spinner, not a page skeleton —
 *   so they are not part of the perpetual-skeleton class. Callers that
 *   want a bound pass `timeoutMs` explicitly.
 * - SESSION PEEK: 10s. The internal `/api/auth/session` read that every
 *   request awaits first; if it hung, it would strand every page. On
 *   timeout it resolves to "no session" (the existing catch), so the
 *   request proceeds unauthenticated rather than hanging.
 */
const READ_TIMEOUT_MS = 15_000;
const SESSION_PEEK_TIMEOUT_MS = 10_000;

/**
 * `fetch` bounded by a timeout, via a `Promise.race` (NOT solely an
 * `AbortController`): the race guarantees the rejection fires even if the
 * underlying fetch implementation — or a test mock — never honours the
 * abort signal. The controller is still aborted on timeout to free the
 * real socket. `timeoutMs <= 0` means unbounded (plain `fetch`).
 *
 * On timeout this rejects with {@link GatewayTimeoutError}; a
 * caller-supplied `signal` is chained so an external abort still works.
 */
async function fetchWithTimeout(
  url: string,
  init: RequestInit,
  timeoutMs: number,
  path: string,
): Promise<Response> {
  if (!timeoutMs || timeoutMs <= 0) return fetch(url, init);

  const controller = new AbortController();
  const callerSignal = init.signal ?? undefined;
  if (callerSignal) {
    if (callerSignal.aborted) controller.abort();
    else
      callerSignal.addEventListener("abort", () => controller.abort(), {
        once: true,
      });
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new GatewayTimeoutError(timeoutMs, path));
    }, timeoutMs);
  });

  const fetchPromise = fetch(url, { ...init, signal: controller.signal });
  // Swallow the post-timeout AbortError on the losing race branch so a
  // wedged socket's eventual rejection never surfaces as an unhandled
  // rejection after the timeout has already won the race.
  fetchPromise.catch(() => {});

  try {
    return await Promise.race([fetchPromise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

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
 * True when a 403 body is the gateway's CSRF double-submit rejection
 * (`{"error":"csrf_missing",...}` or `{"error":"csrf_mismatch",...}`
 * from `middleware_csrf`). These are the only 403s worth a token
 * re-fetch + single retry; an RBAC `{"error":{"code":"forbidden"}}`
 * envelope is NOT (the operator genuinely lacks the capability, so a
 * retry would just 403 again).
 */
function isCsrfRejection(rawBody: string): boolean {
  if (!rawBody) return false;
  try {
    const parsed = JSON.parse(rawBody) as { error?: unknown };
    return parsed.error === "csrf_missing" || parsed.error === "csrf_mismatch";
  } catch {
    return false;
  }
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

// Browser-side fetches go through Next.js's same-origin rewrite at
// `/api/v1/*` (see `web/next.config.ts`) so the browser never crosses
// origins and CORS never enters the picture. SSR and route-handler
// contexts run inside the Next.js process and must use the absolute
// gateway URL because the Next process is the requester, not a browser.
function gatewayBase(): string {
  if (typeof window !== "undefined") return "";
  return resolveGatewayUrl();
}

export interface ApiRequestInit extends Omit<RequestInit, "body"> {
  /** Bearer token override. Defaults to the active session's token. */
  token?: string;
  /** JSON body; serialised automatically. */
  json?: unknown;
  /**
   * Per-call timeout budget in milliseconds. Overrides the method
   * default (15s for reads, unbounded for mutations). `0` (or negative)
   * disables the timeout for this call. On expiry the request rejects
   * with `GatewayTimeoutError`.
   */
  timeoutMs?: number;
}

interface SessionPeek {
  gatewayToken?: string;
  principal?: string;
  /**
   * Tenant id surfaced by the session JSON, when present. Today's
   * magic-link JWT doesn't carry one, so this is undefined and the
   * gateway falls back to the principal-derived tenant. The
   * workspace-switcher track populates this so the SPA can stamp
   * `X-Argo-Tenant-Id` on every request.
   */
  tenantId?: string;
}

async function readSessionPeek(): Promise<SessionPeek> {
  if (typeof window === "undefined") {
    const { auth } = await import("@/auth");
    const session = await auth();
    return {
      gatewayToken: session?.gatewayToken,
      principal: session?.principal,
      tenantId: (session as { tenantId?: string } | null)?.tenantId,
    };
  }
  try {
    const res = await fetchWithTimeout(
      "/api/auth/session",
      { credentials: "same-origin" },
      SESSION_PEEK_TIMEOUT_MS,
      "/api/auth/session",
    );
    if (!res.ok) return {};
    const session = (await res.json()) as SessionPeek;
    return {
      gatewayToken: session?.gatewayToken,
      principal: session?.principal,
      tenantId: session?.tenantId,
    };
  } catch {
    return {};
  }
}

async function readSessionToken(): Promise<string | undefined> {
  return (await readSessionPeek()).gatewayToken;
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
  const { token: tokenOverride, json, headers, timeoutMs, ...rest } = init;
  const peek = tokenOverride ? { gatewayToken: tokenOverride, principal: undefined } : await readSessionPeek();
  const token = peek.gatewayToken;

  const finalHeaders = new Headers(headers);
  if (json !== undefined) finalHeaders.set("Content-Type", "application/json");
  if (token) finalHeaders.set("Authorization", `Bearer ${token}`);
  // Stamp X-Argo-Principal from the session's decoded JWT so the
  // gateway's dev-bypass auth path sees a real `human:<email>`
  // principal rather than falling back to `system:dev-bypass`.
  // Routes like /api/v1/onboarding/* require the human:<email>
  // shape; without this header they 401 with the X-Argo-Principal
  // header is required message. Caller-supplied header wins.
  if (peek.principal && !finalHeaders.has("X-Argo-Principal")) {
    finalHeaders.set("X-Argo-Principal", peek.principal);
  }
  // Stamp X-Argo-Tenant-Id from the workspace-switcher override (if
  // pinned) or the session payload. Today the resolver returns
  // undefined in single-workspace mode, so the gateway falls back to
  // the principal-derived tenant — no observable change. Once the
  // workspace switcher lands, the resolver returns the active
  // workspace's tenant id and the gateway scopes reads/writes to it.
  // Caller-supplied header wins. See `web/lib/api/tenant.ts`.
  if (!finalHeaders.has("X-Argo-Tenant-Id")) {
    const tenantId = resolveTenantId(peek);
    if (tenantId) finalHeaders.set("X-Argo-Tenant-Id", tenantId);
  }
  finalHeaders.set("Accept", "application/json");
  // CSRF double-submit (Stream claude-rebase-pr-1835-cookies). The
  // gateway's `middleware_csrf` enforces the header on every
  // cookie-authenticated mutating request; attaching it
  // unconditionally on mutating verbs is a no-op for bearer-auth
  // requests (no `__Host-session` cookie → gate skips) but covers
  // the cookie-auth surface without further client-side changes.
  if (MUTATING_METHODS.has(method.toUpperCase()) && !finalHeaders.has("X-CSRF-Token")) {
    const csrf = await resolveCsrfToken();
    if (csrf) {
      finalHeaders.set("X-CSRF-Token", csrf);
    } else if (typeof window !== "undefined") {
      // Browser-side mutating request with no resolvable CSRF token
      // is either pre-login (no `__Host-csrf` minted yet, in which
      // case the request is about to 401 anyway) or a regression
      // (cookie missing mid-session, or both cookie read AND XHR
      // fallback failed). Either way the gateway will return
      // 403 csrf_missing on a cookie-authenticated route. Surface
      // it once at the call site so the demo MP4 doesn't silently
      // 403 on Run-now — see brief
      // `fix-workflow-execute-csrf-token-wiring-verify-before-demo`.
      console.warn(
        `[api/client] ${method} ${path}: no CSRF token resolved — cookie-auth mutating requests will be rejected with 403 csrf_missing`,
      );
    }
  }

  const url = path.startsWith("http") ? path : `${gatewayBase()}${path}`;
  // Reads carry the universal 15s backstop (the perpetual-skeleton fix);
  // mutations stay unbounded unless the caller opts in. A per-call
  // `timeoutMs` overrides either default.
  const isRead = method === "GET" || method === "HEAD";
  const effectiveTimeoutMs = timeoutMs ?? (isRead ? READ_TIMEOUT_MS : 0);
  const body = json !== undefined ? JSON.stringify(json) : undefined;
  const send = () =>
    fetchWithTimeout(
      url,
      { ...rest, method, headers: finalHeaders, body },
      effectiveTimeoutMs,
      path,
    );

  let res = await send();

  // CSRF self-heal (brief fix-browser-hire-csrf-after-gateway-restart).
  // A cookie-auth mutating request can come back 403 csrf_missing /
  // csrf_mismatch when the SPA's token has desynced from the
  // `__Host-csrf` cookie — most often after a mid-session gateway
  // restart re-minted it. Re-fetch a fresh token from
  // `GET /api/v1/auth/csrf`, stamp it, and retry the request EXACTLY
  // once. Browser-only (the XHR fetch needs same-origin cookies) and
  // only for mutating verbs (reads never carry the CSRF header). If the
  // retry still fails the honest error surfaces below — nothing is
  // swallowed, and an RBAC `forbidden` 403 is never retried.
  if (
    res.status === 403 &&
    typeof window !== "undefined" &&
    MUTATING_METHODS.has(method.toUpperCase())
  ) {
    const rawBody = await res
      .clone()
      .text()
      .catch(() => "");
    if (isCsrfRejection(rawBody)) {
      const fresh = await fetchCsrfTokenViaXhr();
      if (fresh) {
        finalHeaders.set("X-CSRF-Token", fresh);
        res = await send();
      }
    }
  }

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

export function apiPut<T, B = unknown>(path: string, body?: B, init?: ApiRequestInit): Promise<T> {
  return request<T>("PUT", path, { ...init, json: body });
}

export function apiDelete<T>(path: string, init?: ApiRequestInit): Promise<T> {
  return request<T>("DELETE", path, init);
}

export { ApiError } from "./types";
