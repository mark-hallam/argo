"use client";

import { cn } from "@/lib/utils";

/**
 * Progress indicator for multi-step flows (currently the hire flow).
 *
 * One dot per step. Step state:
 *  - `done` — already-passed step. Rendered as a small filled dot at
 *    40% opacity. Clickable when `onJumpBack` is provided (jumps back,
 *    preserving state per the brief).
 *  - `current` — active step. Rendered as a larger filled dot with a
 *    primary-colored ring halo. Not clickable.
 *  - `upcoming` — future step. Rendered as an empty muted dot. Not
 *    clickable.
 *
 * Hover tooltips come from the optional `stepLabels` array (one label
 * per step); they fall back to a positional label ("Step N").
 */
export interface ProgressDotsProps {
  /** Total step count. */
  total: number;
  /** 1-based current step. Values < 1 clamp to 1; > total clamp to total. */
  current: number;
  /** Per-step hover tooltip text; same length as `total` ideally. */
  stepLabels?: string[];
  /** When provided, clicking a `done` dot calls this with the 1-based step. */
  onJumpBack?: (step: number) => void;
  /** Accessible label for the whole control. */
  label?: string;
  className?: string;
}

export function ProgressDots({
  total,
  current,
  stepLabels,
  onJumpBack,
  label,
  className,
}: ProgressDotsProps) {
  const safeTotal = Math.max(1, Math.floor(total));
  const safeCurrent = Math.min(Math.max(1, Math.floor(current)), safeTotal);

  return (
    <div
      role="progressbar"
      aria-valuenow={safeCurrent}
      aria-valuemin={1}
      aria-valuemax={safeTotal}
      aria-label={label ?? "Progress"}
      className={cn("inline-flex items-center gap-1.5", className)}
    >
      {Array.from({ length: safeTotal }).map((_, i) => {
        const step = i + 1;
        const state: "done" | "current" | "upcoming" =
          step < safeCurrent ? "done" : step === safeCurrent ? "current" : "upcoming";
        const tooltip = stepLabels?.[i] ?? `Step ${step}`;
        const canJump = state === "done" && typeof onJumpBack === "function";
        const baseDot = cn(
          "block rounded-full transition-all duration-[var(--duration-fast)]",
          state === "done" && "size-2 bg-[var(--primary)] opacity-40",
          state === "current" && "size-2.5 bg-[var(--primary)] ring-2 ring-[var(--primary)]/30",
          state === "upcoming" && "size-2 bg-[var(--muted)]",
        );
        if (canJump) {
          return (
            <button
              key={step}
              type="button"
              title={tooltip}
              aria-label={`Go back to ${tooltip}`}
              data-state={state}
              data-testid={`progress-dot-${step}`}
              onClick={() => onJumpBack?.(step)}
              className={cn(
                "grid place-items-center size-5 rounded-full focus-visible:outline-none",
                "focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                "hover:opacity-100",
              )}
            >
              <span aria-hidden className={baseDot} />
            </button>
          );
        }
        return (
          <span
            key={step}
            title={tooltip}
            aria-label={tooltip}
            data-state={state}
            data-testid={`progress-dot-${step}`}
            className={baseDot}
          />
        );
      })}
    </div>
  );
}
