"use client";

import { useCallback, useEffect, useState } from "react";
import { clearDraft as clearDraftStore, loadDraft, saveDraft } from "./draft-store";
import type { HireDraftState } from "./draft-types";
import {
  clearIdempotencyKey,
  getOrCreateIdempotencyKey,
} from "./idempotency-key";

/**
 * Working state held by the hook before the user has touched the
 * flow — distinct from a persisted draft in that there's no `ts` /
 * `version` yet. The hook writes a `HireDraftState` to localStorage
 * the first time `update()` is called.
 */
export type HireDraftBody = Omit<HireDraftState, "version" | "ts">;

export interface UseHireDraftResult {
  /** Current in-memory draft, or `null` until hydration completes. */
  state: HireDraftState | null;
  /**
   * Patch the draft. Accepts a partial — merged shallowly into the
   * current state. Persists to localStorage and updates in-memory.
   */
  update: (patch: Partial<HireDraftBody>) => Promise<void>;
  /** Clear the draft from storage and reset in-memory state to `null`. */
  clear: () => Promise<void>;
  /** `true` until the initial `loadDraft()` resolves. */
  isLoading: boolean;
  /** Last error from a load/save/clear attempt. */
  error: Error | null;
  /**
   * UUID v7 idempotency key generated on Step 1 first-load and
   * persisted across Step 2–6. The gateway hire endpoint de-dupes on
   * `(tenant, key, created_at > now - 1h)` so a double-click or
   * reload cannot create a duplicate worker row. `null` until the
   * hook has mounted on the client (SSR).
   */
  idempotencyKey: string | null;
}

const EMPTY_BODY: HireDraftBody = { step: 1 };

/**
 * React hook wrapping the localStorage draft store. Mirrors the
 * TanStack Query result shape (`{ data, isLoading, error }`) so the
 * v1.5 swap to a gateway-backed store is mechanical.
 *
 * SSR-safe: localStorage access is gated by a `useEffect`, so the
 * first render returns `state=null, isLoading=true` and the second
 * render (post-hydrate) reflects the persisted draft.
 */
export function useHireDraft(): UseHireDraftResult {
  const [state, setState] = useState<HireDraftState | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    // Lazy-generate the hire-flow idempotency key on first mount.
    // Step 1 page is the typical first-load, but any step page mounts
    // the same hook and produces / reuses the same persisted key.
    setIdempotencyKey(getOrCreateIdempotencyKey());
    loadDraft()
      .then((draft) => {
        if (!cancelled) {
          setState(draft);
          setIsLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setIsLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const update = useCallback(async (patch: Partial<HireDraftBody>) => {
    setError(null);
    setState((prev) => {
      const base: HireDraftBody = prev
        ? { step: prev.step, problem: prev.problem, answers: prev.answers, workerDraft: prev.workerDraft }
        : EMPTY_BODY;
      const merged: HireDraftBody = { ...base, ...patch };
      const next: HireDraftState = { ...merged, version: 1, ts: Date.now() };
      void saveDraft(merged).catch((err: unknown) => {
        setError(err instanceof Error ? err : new Error(String(err)));
      });
      return next;
    });
  }, []);

  const clear = useCallback(async () => {
    setError(null);
    try {
      await clearDraftStore();
      clearIdempotencyKey();
      setState(null);
      setIdempotencyKey(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  return { state, update, clear, isLoading, error, idempotencyKey };
}
