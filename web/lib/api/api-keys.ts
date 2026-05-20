/**
 * API keys client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. Gateway
 * endpoints land later (planned brief
 * `mission-gateway-api-keys-endpoint`); until then every call here will
 * 404 and fall back to the bundled `SAMPLE_API_KEYS` dataset so the
 * `/settings/api-keys` demo keeps working in dev without a running
 * gateway.
 *
 * Acceptance contract (mirrors `lib/api/team.ts`):
 * - `fetchApiKeys` returns `ApiKeyRow[]`.
 * - `createApiKey` returns the new row PLUS the one-time-visible full
 *   token (the reveal panel reads this once then forgets it).
 * - `revokeApiKey` returns a `{ ok: true, id }` envelope.
 * - 404 from the gateway → mock fallback in dev (`source = "mock-offline"`).
 * - `ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately, no network call.
 * - Strict production (`NODE_ENV === "production"` AND
 *   `NEXT_PUBLIC_ENV === "production"`): no mock fallback. 404 and any
 *   other `ApiError` propagate.
 * - `NetworkError` (gateway unreachable) → mock fallback in dev,
 *   propagates in prod.
 */

import { apiDelete, apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_API_KEYS } from "@/lib/sample-api-keys";

export type ApiKeyScope =
  | "read:workers"
  | "write:workers"
  | "read:workflows"
  | "write:workflows"
  | "read:audit"
  | "write:audit"
  | "read:skills"
  | "write:skills"
  | "read:members"
  | "write:members"
  | "run:workflows";

export interface ApiKeyRow {
  id: string;
  name: string;
  /** 12-char preview with mask, e.g. `sk_live_a4b8...c2e1`. */
  prefix: string;
  scopes: ApiKeyScope[];
  cohort_ids: string[];
  all_cohorts: boolean;
  created_at: string;
  last_used_at: string | null;
  expires_at: string | null;
  revoked: boolean;
}

export type ApiKeyExpiry = "never" | "30d" | "90d" | "1y" | { custom: string };

export interface CreateApiKeyRequest {
  name: string;
  scopes: ApiKeyScope[];
  cohort_ids: string[];
  all_cohorts: boolean;
  expiry: ApiKeyExpiry;
}

export interface CreateApiKeyResponse {
  key: ApiKeyRow;
  /** Full bearer token. Reveal-once — never returned again. */
  token: string;
}

export type ApiKeysDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchApiKeysResult {
  keys: ApiKeyRow[];
  source: ApiKeysDataSource;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export async function fetchApiKeys(): Promise<FetchApiKeysResult> {
  if (isAuthBypassed()) {
    return { keys: SAMPLE_API_KEYS, source: "mock-bypass" };
  }
  try {
    const keys = await apiGet<ApiKeyRow[]>("/api/v1/api-keys");
    return { keys, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { keys: SAMPLE_API_KEYS, source: "mock-offline" };
    }
    throw err;
  }
}

/** Build a deterministic-looking fake token for the mock-offline reveal panel. */
function mockToken(): string {
  const alphabet = "abcdefghijklmnopqrstuvwxyz0123456789";
  let body = "";
  for (let i = 0; i < 32; i++) {
    body += alphabet[Math.floor(Math.random() * alphabet.length)];
  }
  return `sk_live_${body}`;
}

function mockKeyFrom(req: CreateApiKeyRequest, token: string): ApiKeyRow {
  return {
    id: `key_${Date.now().toString(36)}`,
    name: req.name.trim() || "Untitled key",
    prefix: `${token.slice(0, 12)}...${token.slice(-4)}`,
    scopes: req.scopes,
    cohort_ids: req.cohort_ids,
    all_cohorts: req.all_cohorts,
    created_at: new Date().toISOString(),
    last_used_at: null,
    expires_at: expiryToIso(req.expiry),
    revoked: false,
  };
}

function expiryToIso(expiry: ApiKeyExpiry): string | null {
  if (expiry === "never") return null;
  if (typeof expiry === "object" && "custom" in expiry) return expiry.custom;
  const d = new Date();
  if (expiry === "30d") d.setDate(d.getDate() + 30);
  if (expiry === "90d") d.setDate(d.getDate() + 90);
  if (expiry === "1y") d.setFullYear(d.getFullYear() + 1);
  return d.toISOString();
}

export async function createApiKey(
  req: CreateApiKeyRequest,
): Promise<CreateApiKeyResponse> {
  if (isAuthBypassed()) {
    const token = mockToken();
    return { key: mockKeyFrom(req, token), token };
  }
  try {
    return await apiPost<CreateApiKeyResponse, CreateApiKeyRequest>(
      "/api/v1/api-keys",
      req,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      const token = mockToken();
      return { key: mockKeyFrom(req, token), token };
    }
    throw err;
  }
}

export async function revokeApiKey(id: string): Promise<{ ok: true; id: string }> {
  if (isAuthBypassed()) {
    return { ok: true, id };
  }
  try {
    await apiDelete<unknown>(`/api/v1/api-keys/${encodeURIComponent(id)}`);
    return { ok: true, id };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { ok: true, id };
    }
    throw err;
  }
}
