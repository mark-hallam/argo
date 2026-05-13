"use client";

import { Loader2 } from "lucide-react";

/**
 * AddResponsibility Step 2 — Classify (spinner / error).
 *
 * Renders while the classifier mutation is in-flight. The container
 * dialog manages the in-flight timing; the mock fallback in
 * `classifyDuty` simulates 200-500ms latency client-side so the
 * spinner is visible even with the gateway offline.
 */

export interface Step2ClassifyProps {
  workerName: string;
  error?: string | null;
  onRetry: () => void;
  onCancel: () => void;
}

export function Step2Classify({
  workerName,
  error,
  onRetry,
  onCancel,
}: Step2ClassifyProps) {
  if (error) {
    return (
      <div data-testid="duty-step-classify-error" className="space-y-3 text-sm">
        <p className="text-[var(--foreground)]">
          Couldn&apos;t figure out the shape of this: {error}
        </p>
        <div className="flex justify-end gap-2 text-xs">
          <button
            type="button"
            onClick={onCancel}
            data-testid="duty-classify-cancel"
            className="text-[var(--muted-foreground)] underline-offset-4 hover:underline"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onRetry}
            data-testid="duty-classify-retry"
            className="text-[var(--primary)] underline-offset-4 hover:underline"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="duty-step-classify"
      className="flex flex-col items-center justify-center gap-3 py-12 text-center"
    >
      <Loader2
        aria-hidden
        className="size-6 animate-spin text-[var(--muted-foreground)]"
      />
      <p className="text-sm text-[var(--muted-foreground)]">
        Figuring out the shape of this responsibility&hellip;
      </p>
      <p className="text-xs text-[var(--muted-foreground)]/80">
        {workerName} is sorting it into a recurring duty.
      </p>
    </div>
  );
}
