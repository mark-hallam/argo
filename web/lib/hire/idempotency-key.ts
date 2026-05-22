/**
 * Idempotency key for the hire-commit POST.
 *
 * The key is a UUID v7 (time-sortable) generated lazily on Step 1
 * first-load and persisted across Steps 2–6 in localStorage. The
 * gateway de-dupes against `(tenant_id, idempotency_key, created_at >
 * now - 1h)` so a double-click or page reload during Step 4 cannot
 * create a second worker row.
 *
 * The key lives in its own localStorage entry (not on
 * [[HireDraftState]]) because Phase 1 of the hire-flow schema split
 * locked `draft-types.ts` against changes. `clearIdempotencyKey()` is
 * called alongside the draft clear so a fresh hire flow gets a fresh
 * key.
 *
 * SSR-safe: every accessor checks for `typeof window` before touching
 * `localStorage`. The server build returns `null` and the client
 * generates on first mount.
 */

/** localStorage key — separate from [[HIRE_DRAFT_STORAGE_KEY]]. */
export const HIRE_IDEMPOTENCY_STORAGE_KEY = "skycot.hireDraft.idempotencyKey";

/**
 * Generate a UUID v7 — 48-bit big-endian Unix-ms timestamp prefix,
 * version 7 nibble, variant 0b10, and 74 random bits. Time-sortable,
 * suitable for use as an idempotency key the gateway can range-scan.
 *
 * `crypto.getRandomValues` is required (Node 19+, all modern browsers).
 */
export function uuidV7(): string {
  const bytes = new Uint8Array(16);
  // 48-bit timestamp, big-endian. Divisions handle the 32-bit-int
  // limit of JS bitwise ops; `Date.now()` is well under 2^48.
  const ts = Date.now();
  bytes[0] = Math.floor(ts / 2 ** 40) & 0xff;
  bytes[1] = Math.floor(ts / 2 ** 32) & 0xff;
  bytes[2] = Math.floor(ts / 2 ** 24) & 0xff;
  bytes[3] = Math.floor(ts / 2 ** 16) & 0xff;
  bytes[4] = Math.floor(ts / 2 ** 8) & 0xff;
  bytes[5] = ts & 0xff;
  const rand = new Uint8Array(10);
  crypto.getRandomValues(rand);
  bytes.set(rand, 6);
  bytes[6] = (bytes[6] & 0x0f) | 0x70; // version 7
  bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant (10xx)
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
}

function safeLocalStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    // localStorage can throw in sandbox / privacy modes; treat as
    // absent rather than crashing the hire flow.
    return null;
  }
}

/**
 * Read the persisted idempotency key, or `null` on SSR / before
 * first-load. Does NOT generate one — use [[getOrCreateIdempotencyKey]]
 * for the lazy-generate-on-Step-1 path.
 */
export function readIdempotencyKey(): string | null {
  const ls = safeLocalStorage();
  if (!ls) return null;
  try {
    const v = ls.getItem(HIRE_IDEMPOTENCY_STORAGE_KEY);
    return v && v.length > 0 ? v : null;
  } catch {
    return null;
  }
}

/**
 * Return the persisted idempotency key, generating + persisting one
 * on first call. Returns `null` only when `localStorage` is
 * unavailable (SSR / sandboxed iframe) — callers in that case should
 * fall through to omitting `idempotency_key` from the POST body and
 * let the gateway accept the request without de-dupe.
 */
export function getOrCreateIdempotencyKey(): string | null {
  const ls = safeLocalStorage();
  if (!ls) return null;
  const existing = readIdempotencyKey();
  if (existing) return existing;
  const fresh = uuidV7();
  try {
    ls.setItem(HIRE_IDEMPOTENCY_STORAGE_KEY, fresh);
  } catch {
    // Quota / privacy errors fall through; the caller still gets a
    // fresh key for this submit even if persistence failed.
  }
  return fresh;
}

/** Drop the persisted key — paired with the [[useHireDraft]] clear path. */
export function clearIdempotencyKey(): void {
  const ls = safeLocalStorage();
  if (!ls) return;
  try {
    ls.removeItem(HIRE_IDEMPOTENCY_STORAGE_KEY);
  } catch {
    /* best-effort */
  }
}
