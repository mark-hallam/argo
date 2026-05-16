"use client";

import { useMutation } from "@tanstack/react-query";
import { apiPost, ApiError } from "@/lib/api/client";

export interface ProviderKeyRequest {
  apiKey: string;
}

export interface ProviderKeyResult {
  validated: boolean;
  testCost: number;
  testTokens?: number;
}

interface ValidatePayload {
  api_key: string;
}

interface ValidateResponse {
  validated: boolean;
  test_cost?: number;
  test_tokens?: number;
}

function isValidateResponse(value: unknown): value is ValidateResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.validated === "boolean";
}

export async function validateProviderKey(
  req: ProviderKeyRequest,
): Promise<ProviderKeyResult> {
  const raw = await apiPost<unknown, ValidatePayload>(
    "/api/v1/onboarding/provider-key",
    { api_key: req.apiKey },
  );
  if (!isValidateResponse(raw)) {
    throw new ApiError(502, "provider-key endpoint returned an unexpected shape");
  }
  if (!raw.validated) {
    throw new ApiError(400, "Anthropic rejected the key");
  }
  return {
    validated: true,
    testCost: raw.test_cost ?? 0.001,
    testTokens: raw.test_tokens,
  };
}

export function useProviderKey() {
  return useMutation<ProviderKeyResult, Error, ProviderKeyRequest>({
    mutationFn: validateProviderKey,
  });
}
