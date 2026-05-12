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

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

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
