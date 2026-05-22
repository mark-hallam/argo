/**
 * React hook wrapping the workflow Save-as-Draft + Publish-to-Head API calls.
 *
 * Exposes stable `saveDraft` and `publishHead` callbacks plus in-flight
 * (`isSaving`, `isPublishing`) and last-error (`lastError`) state.
 * Consumers — currently the builder Save + Publish buttons — call the
 * callbacks and render toasts from the result + any caught `ApiError`.
 *
 * The hook itself owns no retry policy; the calling component is
 * responsible for surfacing a retry CTA when `lastError` is set.
 * `lastError` is shared between the two flows — the most recent failure
 * wins. Callers that need to differentiate (e.g. distinct retry CTAs)
 * inspect the rejection's `ApiError` directly.
 *
 * Concurrent-edit conflicts (409) are routed two ways: the underlying
 * `saveDraft` API call writes the conflict into a module-level store
 * (subscribable via [`useSaveDraftConflict`] below — the sibling
 * `ConcurrentEditBanner` reads from there) AND the hook throws a
 * `ConflictError` so the existing builder error-toast path still
 * fires. The 500-LOC ceiling on the builder page (see HARD RULE 6)
 * forbids threading conflict state as a prop, hence the module-store
 * detour.
 */
import { useCallback, useRef, useState, useSyncExternalStore } from "react";

import {
  clearSaveDraftConflict,
  getSaveDraftConflict,
  publishHead as publishHeadRequest,
  saveDraft as saveDraftRequest,
  subscribeSaveDraftConflict,
  type PublishHeadArgs,
  type PublishHeadResult,
  type SaveDraftArgs,
  type SaveDraftConflict,
  type SaveDraftResult,
} from "@/lib/api/workflows-save-publish";

export class ConcurrentEditError extends Error {
  readonly conflict: SaveDraftConflict;
  constructor(conflict: SaveDraftConflict) {
    super(
      `Workflow was updated by someone else (now v${conflict.current_version}) — refresh to see the latest.`,
    );
    this.name = "ConcurrentEditError";
    this.conflict = conflict;
  }
}

export interface UseWorkflowSavePublishApi {
  saveDraft: (args: SaveDraftArgs) => Promise<SaveDraftResult>;
  publishHead: (args: PublishHeadArgs) => Promise<PublishHeadResult>;
  isSaving: boolean;
  isPublishing: boolean;
  lastError: Error | null;
  reset: () => void;
}

export function useWorkflowSavePublish(): UseWorkflowSavePublishApi {
  const [isSaving, setIsSaving] = useState(false);
  const [isPublishing, setIsPublishing] = useState(false);
  const [lastError, setLastError] = useState<Error | null>(null);
  // Per-workflow loaded version cache, populated from each successful
  // save's `version` field. We auto-include this as `if_version` on
  // the NEXT save so the gateway's optimistic-lock contract fires
  // without callers having to thread version state through props.
  // Callers can override by passing `args.if_version` explicitly.
  const loadedVersionRef = useRef<Map<string, number>>(new Map());

  const saveDraft = useCallback(async (args: SaveDraftArgs) => {
    setIsSaving(true);
    setLastError(null);
    const cacheKey = `${args.vertical_id}/${args.name}`;
    const effectiveIfVersion =
      typeof args.if_version === "number"
        ? args.if_version
        : loadedVersionRef.current.get(cacheKey);
    try {
      const result = await saveDraftRequest({
        ...args,
        if_version: effectiveIfVersion,
      });
      if (result.kind === "conflict") {
        // Don't update the cache on conflict — the user hasn't
        // accepted the live head yet; their dirty draft is still on
        // top of the version they originally loaded.
        throw new ConcurrentEditError(result);
      }
      loadedVersionRef.current.set(cacheKey, result.version);
      return result;
    } catch (err) {
      const wrapped = err instanceof Error ? err : new Error(String(err));
      setLastError(wrapped);
      throw wrapped;
    } finally {
      setIsSaving(false);
    }
  }, []);

  const publishHead = useCallback(async (args: PublishHeadArgs) => {
    setIsPublishing(true);
    setLastError(null);
    try {
      const result = await publishHeadRequest(args);
      return result;
    } catch (err) {
      const wrapped = err instanceof Error ? err : new Error(String(err));
      setLastError(wrapped);
      throw wrapped;
    } finally {
      setIsPublishing(false);
    }
  }, []);

  const reset = useCallback(() => {
    setLastError(null);
    setIsSaving(false);
    setIsPublishing(false);
  }, []);

  return { saveDraft, publishHead, isSaving, isPublishing, lastError, reset };
}

/**
 * Subscribe to the most-recent save-draft conflict (409 from the
 * gateway). Returns `null` when there is no live conflict. Consumers
 * call [`clearSaveDraftConflict`] (re-exported) to dismiss the banner.
 */
export function useSaveDraftConflict(): SaveDraftConflict | null {
  return useSyncExternalStore(
    subscribeSaveDraftConflict,
    getSaveDraftConflict,
    () => null,
  );
}

export { clearSaveDraftConflict } from "@/lib/api/workflows-save-publish";
