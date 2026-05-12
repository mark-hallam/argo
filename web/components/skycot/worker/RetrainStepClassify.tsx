"use client";

import { Loader2 } from "lucide-react";

/**
 * Retrain Step 2 — Classify (spinner-only).
 *
 * Renders while the classifier mutation is in-flight. The container
 * dialog manages timing (200-500ms mock latency simulated in
 * `classifyRetrain`'s mock fallback path).
 */

export interface RetrainStepClassifyProps {
  workerName: string;
  error?: string | null;
  onRetry: () => void;
  onCancel: () => void;
}

export function RetrainStepClassify({
  workerName,
  error,
  onRetry,
  onCancel,
}: RetrainStepClassifyProps) {
  if (error) {
    return (
      <div data-testid="retrain-step-classify-error" className="space-y-3 text-sm">
        <p className="text-[var(--foreground)]">
          Couldn&apos;t classify that change: {error}
        </p>
        <div className="flex justify-end gap-2 text-xs">
          <button
            type="button"
            onClick={onCancel}
            className="text-[var(--muted-foreground)] underline-offset-4 hover:underline"
            data-testid="retrain-classify-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onRetry}
            className="text-[var(--primary)] underline-offset-4 hover:underline"
            data-testid="retrain-classify-retry"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="retrain-step-classify"
      className="flex flex-col items-center justify-center gap-3 py-12 text-center"
    >
      <Loader2
        aria-hidden
        className="size-6 animate-spin text-[var(--muted-foreground)]"
      />
      <p className="text-sm text-[var(--muted-foreground)]">
        Figuring out the right place for this&hellip;
      </p>
      <p className="text-xs text-[var(--muted-foreground)]/80">
        {workerName} is sorting your change into the right bucket.
      </p>
    </div>
  );
}
