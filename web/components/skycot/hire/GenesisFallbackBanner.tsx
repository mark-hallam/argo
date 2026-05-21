"use client";

import { AlertTriangle, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/Button";

/**
 * Step 3 banner surfaced when the worker-draft mutation falls back to
 * the stub (gateway 404 / malformed / 5xx / network error). Copy is
 * plain-language per the operator preference — names the consequence
 * ("Generic questions only") rather than the technical failure
 * ("Genesis service unavailable").
 *
 * The Retry button re-invokes the gateway call via the caller's
 * `onRetry` (the hook's `start()`). The banner uses a static visual
 * emphasis only — no shake / pulse animation, so reduced-motion
 * preferences are satisfied implicitly.
 */
export interface GenesisFallbackBannerProps {
  onRetry: () => void;
  /**
   * `true` while the underlying mutation is in flight. The Retry
   * button is disabled and labeled "Retrying…" so users don't pile
   * up identical mutations.
   */
  isRetrying?: boolean;
}

export function GenesisFallbackBanner({
  onRetry,
  isRetrying = false,
}: GenesisFallbackBannerProps) {
  return (
    <div
      role="alert"
      aria-live="polite"
      data-testid="hire-step3-fallback-banner"
      className="flex flex-col gap-3 rounded-[var(--radius-md)] border border-[var(--warning-border,var(--border))] bg-[var(--warning-surface,var(--accent))] px-4 py-3 text-sm text-[var(--warning-foreground,var(--accent-foreground))] sm:flex-row sm:items-center sm:justify-between"
    >
      <div className="flex items-start gap-2">
        <AlertTriangle
          className="mt-0.5 size-4 shrink-0 text-[var(--warning-icon,currentColor)]"
          aria-hidden
        />
        <p className="leading-snug">
          <strong className="font-semibold">Generic questions only</strong>
          {" — we couldn’t reach Genesis. The worker we draft from your answers may be generic too."}
        </p>
      </div>
      <Button
        type="button"
        variant="secondary"
        size="sm"
        data-testid="hire-step3-fallback-retry"
        onClick={onRetry}
        disabled={isRetrying}
        className="shrink-0"
      >
        <RefreshCw
          className={isRetrying ? "animate-spin" : undefined}
          aria-hidden
        />
        {isRetrying ? "Retrying…" : "Retry"}
      </Button>
    </div>
  );
}
