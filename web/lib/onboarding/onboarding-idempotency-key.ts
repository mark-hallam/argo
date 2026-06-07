/**
 * Idempotency key for the onboarding first-hire POST.
 *
 * Kept SEPARATE from the multi-step `/hire` flow's
 * `HIRE_IDEMPOTENCY_STORAGE_KEY` (`lib/hire/idempotency-key.ts`) so the two
 * flows never share a token: the onboarding first hire and a later full-wizard
 * hire are different operations and must de-dupe independently.
 *
 * The SAME key is returned on the initial `firstHire` submit AND on the
 * post-review `confirmFirstHire` call (both read `getOrCreateOnboardingIdempotencyKey`),
 * so a needs_review → confirm round-trip — and a native Enter-key double-fire
 * on the form — de-dupe against the same `(tenant_id, idempotency_key)` the
 * gateway keys on. `clearOnboardingIdempotencyKey()` runs on the success path
 * (alongside the draft reset) so the next hire mints a fresh token.
 *
 * SSR-safe: every accessor checks `typeof window` before touching
 * `localStorage` and returns `null` when storage is unavailable; the caller
 * then omits `idempotency_key` and lets the gateway accept the request
 * without client-side de-dupe.
 */

import { uuidV7 } from "@/lib/hire/idempotency-key";

/** localStorage key — distinct from `HIRE_IDEMPOTENCY_STORAGE_KEY`. */
export const ONBOARDING_IDEMPOTENCY_STORAGE_KEY =
  "skycot.onboarding.firstHire.idempotencyKey";

function onboardingKeyStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    // Sandbox / privacy mode — treat as absent and fall through to omitting
    // the key (the gateway accepts the request without de-dupe).
    return null;
  }
}

/**
 * Return the persisted onboarding idempotency key, generating + persisting
 * one on first call. Returns `null` only when storage is unavailable (SSR /
 * sandbox).
 */
export function getOrCreateOnboardingIdempotencyKey(): string | null {
  const ls = onboardingKeyStorage();
  if (!ls) return null;
  try {
    const existing = ls.getItem(ONBOARDING_IDEMPOTENCY_STORAGE_KEY);
    if (existing && existing.length > 0) return existing;
  } catch {
    return null;
  }
  const fresh = uuidV7();
  try {
    ls.setItem(ONBOARDING_IDEMPOTENCY_STORAGE_KEY, fresh);
  } catch {
    // Quota / privacy write failure — still return the fresh key for THIS
    // submit (degraded: a later confirm in the same tab may mint its own;
    // acceptable when persistence is blocked).
  }
  return fresh;
}

/** Drop the persisted onboarding key — paired with the draft `reset()`. */
export function clearOnboardingIdempotencyKey(): void {
  const ls = onboardingKeyStorage();
  if (!ls) return;
  try {
    ls.removeItem(ONBOARDING_IDEMPOTENCY_STORAGE_KEY);
  } catch {
    /* best-effort */
  }
}
