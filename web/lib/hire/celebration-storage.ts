/**
 * localStorage-backed celebration store for the Step-6 hand-off.
 *
 * After a hire commit, `/hire/done/<workerId>` writes a CelebrationEntry
 * here; the approvals inbox reads the entries on mount and renders one
 * CelebrationBanner per recently-hired worker. v1 is per-device + per-
 * browser; the post-hire state machine lives behind
 * `mission-enterprise-hire-post-state`.
 *
 * Single storage key holds an array of entries (avoids per-key prefix
 * scanning when listing). All exports degrade silently when
 * `window.localStorage` is unavailable — banner just doesn't appear.
 */

export interface CelebrationEntry {
  workerId: string;
  workerName: string;
  /** ISO timestamp the worker was hired. Used to order banners. */
  hiredAt: string;
  /** True if the user skipped Step 5 (Connect) — banner shows the inert variant. */
  inert: boolean;
  /** ISO timestamp at which the entry should be dropped. */
  ttl: string;
}

export const CELEBRATIONS_STORAGE_KEY = "skycot.celebrations";

/** 24h — after that the moment has passed; surfacing later feels weird. */
export const CELEBRATION_TTL_MS = 24 * 60 * 60 * 1000;

function getStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function looksLikeEntry(value: unknown): value is CelebrationEntry {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.workerId === "string" &&
    typeof v.workerName === "string" &&
    typeof v.hiredAt === "string" &&
    typeof v.ttl === "string" &&
    typeof v.inert === "boolean"
  );
}

function readRaw(): CelebrationEntry[] {
  const storage = getStorage();
  if (!storage) return [];
  let raw: string | null;
  try {
    raw = storage.getItem(CELEBRATIONS_STORAGE_KEY);
  } catch {
    return [];
  }
  if (!raw) return [];
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) return [];
  return parsed.filter(looksLikeEntry);
}

function writeRaw(entries: CelebrationEntry[]): boolean {
  const storage = getStorage();
  if (!storage) return false;
  try {
    storage.setItem(CELEBRATIONS_STORAGE_KEY, JSON.stringify(entries));
    return true;
  } catch {
    return false;
  }
}

/**
 * Upsert a celebration. If an entry for `workerId` already exists it is
 * replaced (re-hiring the same worker shouldn't queue duplicates).
 * Auto-stamps a fresh `ttl` of 24h from now.
 */
export function setCelebration(
  workerId: string,
  data: { workerName: string; inert?: boolean; hiredAt?: string },
): boolean {
  const now = Date.now();
  const entry: CelebrationEntry = {
    workerId,
    workerName: data.workerName,
    hiredAt: data.hiredAt ?? new Date(now).toISOString(),
    inert: data.inert ?? false,
    ttl: new Date(now + CELEBRATION_TTL_MS).toISOString(),
  };
  const current = readRaw().filter((e) => e.workerId !== workerId);
  return writeRaw([...current, entry]);
}

/**
 * Read the unexpired celebration entries, oldest-first. Side-effect:
 * if any expired entries are removed during prune, the cleaned list
 * is written back so the next read is cheap.
 */
export function getCelebrations(): CelebrationEntry[] {
  const entries = readRaw();
  const fresh = filterUnexpired(entries);
  if (fresh.length !== entries.length) writeRaw(fresh);
  return fresh.sort((a, b) => a.hiredAt.localeCompare(b.hiredAt));
}

/** Remove the celebration for `workerId`. Idempotent. */
export function clearCelebration(workerId: string): void {
  const entries = readRaw();
  const next = entries.filter((e) => e.workerId !== workerId);
  if (next.length !== entries.length) writeRaw(next);
}

/**
 * Drop expired entries from storage. Exposed for the hook's mount-time
 * sweep; `getCelebrations()` calls it inline as a side-effect already.
 */
export function pruneExpired(): void {
  const entries = readRaw();
  const fresh = filterUnexpired(entries);
  if (fresh.length !== entries.length) writeRaw(fresh);
}

function filterUnexpired(entries: CelebrationEntry[]): CelebrationEntry[] {
  const now = Date.now();
  return entries.filter((e) => {
    const expiry = Date.parse(e.ttl);
    return Number.isFinite(expiry) && expiry > now;
  });
}
