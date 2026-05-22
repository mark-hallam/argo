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
 */
import { useCallback, useState } from "react";

import {
  publishHead as publishHeadRequest,
  saveDraft as saveDraftRequest,
  type PublishHeadArgs,
  type PublishHeadResult,
  type SaveDraftArgs,
  type SaveDraftResult,
} from "@/lib/api/workflows-save-publish";

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

  const saveDraft = useCallback(async (args: SaveDraftArgs) => {
    setIsSaving(true);
    setLastError(null);
    try {
      const result = await saveDraftRequest(args);
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
