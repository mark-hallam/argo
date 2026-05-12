"use client";

import { cn } from "@/lib/utils";

/**
 * Progress indicator for multi-step flows (currently the hire
 * onboarding). One dot per step; states are `done`, `current`,
 * `upcoming`. The label is read by screen readers.
 */
export interface ProgressDotsProps {
  /** Total step count. Must be >= 1. */
  total: number;
  /** Zero-indexed current step. */
  current: number;
  /** Accessible label, e.g. "Hire flow progress". */
  label?: string;
  className?: string;
}

export function ProgressDots({ total, current, label, className }: ProgressDotsProps) {
  const safeTotal = Math.max(1, Math.floor(total));
  const safeCurrent = Math.min(Math.max(0, Math.floor(current)), safeTotal - 1);

  return (
    <div
      role="progressbar"
      aria-valuenow={safeCurrent + 1}
      aria-valuemin={1}
      aria-valuemax={safeTotal}
      aria-label={label ?? "Progress"}
      className={cn("inline-flex items-center gap-1.5", className)}
    >
      {Array.from({ length: safeTotal }).map((_, i) => {
        const state = i < safeCurrent ? "done" : i === safeCurrent ? "current" : "upcoming";
        return (
          <span
            key={i}
            aria-hidden
            data-state={state}
            className={cn(
              "block rounded-full transition-all duration-[var(--duration-fast)]",
              state === "done" && "size-2 bg-[var(--primary)]",
              state === "current" && "size-2.5 bg-[var(--primary)] ring-2 ring-[var(--primary)]/30",
              state === "upcoming" && "size-2 bg-[var(--muted)]",
            )}
          />
        );
      })}
    </div>
  );
}
