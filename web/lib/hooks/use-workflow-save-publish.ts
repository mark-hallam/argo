/**
 * React hook wrapping the workflow Save-as-Draft API call.
 *
 * Exposes a stable `saveDraft` callback plus in-flight (`isSaving`)
 * and last-error (`lastError`) state. Consumers — currently the
 * builder Save button — call `saveDraft({...})` and render toasts
 * from the result + any caught `ApiError`.
 *
 * The hook itself owns no retry policy; the calling component is
 * responsible for surfacing a retry CTA when `lastError` is set.
 */
import { useCallback, useState } from "react";

import {
  saveDraft as saveDraftRequest,
  type SaveDraftArgs,
  type SaveDraftResult,
} from "@/lib/api/workflows-save-publish";

export interface UseWorkflowSavePublishApi {
  saveDraft: (args: SaveDraftArgs) => Promise<SaveDraftResult>;
  isSaving: boolean;
  lastError: Error | null;
  reset: () => void;
}

export function useWorkflowSavePublish(): UseWorkflowSavePublishApi {
  const [isSaving, setIsSaving] = useState(false);
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

  const reset = useCallback(() => {
    setLastError(null);
    setIsSaving(false);
  }, []);

  return { saveDraft, isSaving, lastError, reset };
}
