"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createApiKey,
  fetchApiKeys,
  revokeApiKey,
  type ApiKeyRow,
  type CreateApiKeyRequest,
  type CreateApiKeyResponse,
  type FetchApiKeysResult,
} from "@/lib/api/api-keys";

const API_KEYS_KEY = ["api-keys"] as const;

/** Workspace-scoped list of long-lived API keys. */
export function useApiKeys() {
  return useQuery<FetchApiKeysResult>({
    queryKey: API_KEYS_KEY,
    queryFn: fetchApiKeys,
  });
}

/**
 * Mint a new API key. Returns the full token ONCE in `data.token` — the
 * caller must show the reveal panel immediately; the cache only stores
 * the row (without the token) for the table view.
 */
export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation<CreateApiKeyResponse, Error, CreateApiKeyRequest>({
    mutationFn: createApiKey,
    onSuccess: (resp) => {
      qc.setQueryData<FetchApiKeysResult>(API_KEYS_KEY, (prev) => {
        if (!prev) return { keys: [resp.key], source: "mock-offline" };
        return { ...prev, keys: [resp.key, ...prev.keys] };
      });
    },
  });
}

interface RevokeContext {
  prev: FetchApiKeysResult | undefined;
}

/** Revoke an API key. Optimistic — flips `revoked: true` then rolls back on error. */
export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation<{ ok: true; id: string }, Error, string, RevokeContext>({
    mutationFn: revokeApiKey,
    onMutate: async (id): Promise<RevokeContext> => {
      await qc.cancelQueries({ queryKey: API_KEYS_KEY });
      const prev = qc.getQueryData<FetchApiKeysResult>(API_KEYS_KEY);
      qc.setQueryData<FetchApiKeysResult>(API_KEYS_KEY, (curr) => {
        if (!curr) return curr;
        return {
          ...curr,
          keys: curr.keys.map((k: ApiKeyRow) =>
            k.id === id ? { ...k, revoked: true } : k,
          ),
        };
      });
      return { prev };
    },
    onError: (_err, _id, ctx) => {
      if (ctx?.prev) qc.setQueryData(API_KEYS_KEY, ctx.prev);
    },
  });
}
