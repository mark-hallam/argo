import {
  HIRE_DRAFT_STORAGE_KEY,
  HIRE_DRAFT_TTL_MS,
  HIRE_DRAFT_VERSION,
  type HireDraftState,
} from "./draft-types";

/**
 * localStorage-backed draft store for the hire flow.
 *
 * v1 is client-only: drafts live on the device that authored them.
 * v1.5 swaps this implementation for a gateway-backed store with
 * tenant + user scope (see `mission-enterprise-hire-draft-server-store`).
 * The function signatures are async so the swap is mechanical.
 *
 * **Failure-mode contract.** All exports must degrade gracefully when
 * `window.localStorage` is unavailable (server render, private-mode
 * browsers, storage quota exceeded). Errors are swallowed; readers
 * observe an empty store and writers no-op. The hire flow then runs in
 * volatile mode — losing the draft on reload is acceptable; crashing
 * mid-hire is not.
 */

function getStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function isExpired(ts: number): boolean {
  return Number.isFinite(ts) && Date.now() - ts > HIRE_DRAFT_TTL_MS;
}

function looksLikeDraft(value: unknown): value is HireDraftState {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (v.version !== HIRE_DRAFT_VERSION) return false;
  if (typeof v.ts !== "number") return false;
  const step = v.step;
  return step === 1 || step === 2 || step === 3 || step === 4 || step === 5 || step === 6;
}

/** Return the persisted draft, or `null` if none / expired / malformed. */
export async function loadDraft(): Promise<HireDraftState | null> {
  const storage = getStorage();
  if (!storage) return null;
  let raw: string | null;
  try {
    raw = storage.getItem(HIRE_DRAFT_STORAGE_KEY);
  } catch {
    return null;
  }
  if (!raw) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!looksLikeDraft(parsed)) return null;
  if (isExpired(parsed.ts)) {
    await clearDraft();
    return null;
  }
  return parsed;
}

/**
 * Persist `state` atomically with a fresh `ts`. Returns `true` on success,
 * `false` if storage was unavailable or the write threw (quota, etc.).
 */
export async function saveDraft(state: Omit<HireDraftState, "version" | "ts">): Promise<boolean> {
  const storage = getStorage();
  if (!storage) return false;
  const record: HireDraftState = {
    ...state,
    version: HIRE_DRAFT_VERSION,
    ts: Date.now(),
  };
  try {
    storage.setItem(HIRE_DRAFT_STORAGE_KEY, JSON.stringify(record));
    return true;
  } catch {
    return false;
  }
}

/** Remove the draft entry. Idempotent — calling on empty storage is fine. */
export async function clearDraft(): Promise<void> {
  const storage = getStorage();
  if (!storage) return;
  try {
    storage.removeItem(HIRE_DRAFT_STORAGE_KEY);
  } catch {
    // best-effort: nothing to fall back to.
  }
}

/** Cheap presence check that doesn't validate the payload. */
export async function hasActiveDraft(): Promise<boolean> {
  const draft = await loadDraft();
  return draft !== null;
}
