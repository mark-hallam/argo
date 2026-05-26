"use client";

import { useMutation } from "@tanstack/react-query";
import { apiPost, ApiError } from "@/lib/api/client";
import { isDemoMode } from "@/lib/skycot/demo-mode";

export interface ProviderKeyRequest {
  apiKey: string;
}

export interface ProviderKeyResult {
  validated: boolean;
  testCost: number;
  testTokens?: number;
  /**
   * `true` when the SPA short-circuited validation (demo mode) and no
   * real Anthropic round-trip happened. The connect-provider page swaps
   * the "$0.001 test call" banner for honest "Mock provider connected"
   * copy when this flag is set — see `feedback_no_theater_in_demo`.
   */
  mockProvider: boolean;
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

// Match the ~400ms feel of a real Anthropic round-trip so the demo
// camera sees a believable latency tick instead of an instant flip.
const DEMO_VALIDATE_DELAY_MS = 400;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function validateProviderKey(
  req: ProviderKeyRequest,
): Promise<ProviderKeyResult> {
  // Demo-mode short-circuit: the provider-key validation route requires
  // a verified human principal which the investor demo doesn't have.
  // We return `mockProvider: true` so the page renders honest "Mock
  // provider connected" copy — never a "$0.001 test call" lie. The
  // 400ms wait keeps the success beat from snapping instantly.
  if (isDemoMode()) {
    await delay(DEMO_VALIDATE_DELAY_MS);
    return { validated: true, testCost: 0, mockProvider: true };
  }
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
    mockProvider: false,
  };
}

export function useProviderKey() {
  return useMutation<ProviderKeyResult, Error, ProviderKeyRequest>({
    mutationFn: validateProviderKey,
  });
}
