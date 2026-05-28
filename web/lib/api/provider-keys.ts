/**
 * Provider API-key client.
 *
 * Backs the inline "Add key / Rotate" form on `/settings/providers`.
 * Distinct from `lib/api/api-keys.ts`, which handles SKYCOT bearer
 * tokens for service accounts / CI integrations. These rows describe
 * the operator's *upstream* provider credentials (Anthropic, OpenAI,
 * Google, Azure, Bedrock) — the keys SKYCOT uses to call the models.
 *
 * Gateway endpoints land later (planned brief
 * `mission-gateway-provider-keys-endpoint`); until then every call
 * 404s and falls back to a per-tab in-memory store so the
 * `/settings/providers` form keeps working in dev. Auth-bypass +
 * 404-fallback contract mirrors `lib/api/settings.ts`.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { PROVIDER_OPTIONS, type ProviderName } from "@/lib/api/settings";

export interface ProviderKeyStatus {
  provider: ProviderName;
  configured: boolean;
  /** Masked preview, e.g. `sk-ant-...4xZ2`. `null` when no key set. */
  masked_preview: string | null;
  /** ISO timestamp of last add/rotate. `null` when no key set. */
  updated_at: string | null;
}

export interface SaveProviderKeyRequest {
  provider: ProviderName;
  /** Raw key. Server stores hashed + last-4; never returned in full. */
  key: string;
}

export type ProviderKeysSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchProviderKeysResult {
  keys: ProviderKeyStatus[];
  source: ProviderKeysSource;
}

export interface SaveProviderKeyResult {
  status: ProviderKeyStatus;
  source: ProviderKeysSource;
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

const mockStore = new Map<ProviderName, ProviderKeyStatus>();

function defaultUnconfigured(): ProviderKeyStatus[] {
  return PROVIDER_OPTIONS.map((opt) => {
    const cached = mockStore.get(opt.value);
    if (cached) return cached;
    return {
      provider: opt.value,
      configured: false,
      masked_preview: null,
      updated_at: null,
    };
  });
}

function mask(provider: ProviderName, key: string): string {
  const trimmed = key.trim();
  const tail = trimmed.length >= 4 ? trimmed.slice(-4) : trimmed;
  const prefix = provider === "anthropic" ? "sk-ant-" : "sk-";
  return `${prefix}...${tail}`;
}

export async function fetchProviderKeys(): Promise<FetchProviderKeysResult> {
  if (isAuthBypassed()) {
    return { keys: defaultUnconfigured(), source: "mock-bypass" };
  }
  try {
    const keys = await apiGet<ProviderKeyStatus[]>("/api/v1/providers/keys");
    return { keys, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { keys: defaultUnconfigured(), source: "mock-offline" };
    }
    throw err;
  }
}

export async function saveProviderKey(
  req: SaveProviderKeyRequest,
): Promise<SaveProviderKeyResult> {
  const next: ProviderKeyStatus = {
    provider: req.provider,
    configured: true,
    masked_preview: mask(req.provider, req.key),
    updated_at: new Date().toISOString(),
  };
  if (isAuthBypassed()) {
    mockStore.set(req.provider, next);
    return { status: next, source: "mock-bypass" };
  }
  try {
    const status = await apiPost<ProviderKeyStatus, SaveProviderKeyRequest>(
      "/api/v1/providers/keys",
      req,
    );
    return { status, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      mockStore.set(req.provider, next);
      return { status: next, source: "mock-offline" };
    }
    throw err;
  }
}

/** Test-only: reset the in-memory mock store. */
export function __resetProviderKeysMockStore(): void {
  mockStore.clear();
}
