"use client";

import { Check, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Cosmetic 5-stage progress checklist for Step 3 (Drafting).
 *
 * Pure presentational — receives a `currentStage` (0..5) from the
 * parent and renders the matching state per row:
 *  - index < currentStage  → completed (check icon)
 *  - index === currentStage → active (spinner)
 *  - index > currentStage  → pending (muted bullet)
 *
 * When `currentStage >= STAGES.length`, every row is checked. The
 * parent decides timing (`~700ms` intervals per stage); this list
 * does not own a timer.
 */
export const DRAFTING_STAGES = [
  "Building her persona",
  "Drafting her first-week routine",
  "Picking a few opening examples",
  "Tuning her voice to match yours",
  "Almost done…",
] as const;

export interface DraftingProgressListProps {
  /** 0-based stage index. `STAGES.length` ⇒ all done. */
  currentStage: number;
  /** Optional extra classes for the wrapper `<ol>`. */
  className?: string;
}

export function DraftingProgressList({
  currentStage,
  className,
}: DraftingProgressListProps) {
  return (
    <ol
      data-testid="hire-step3-progress-list"
      className={cn("space-y-3", className)}
      aria-label="Drafting progress"
    >
      {DRAFTING_STAGES.map((label, idx) => {
        const state: "done" | "active" | "pending" =
          idx < currentStage ? "done" : idx === currentStage ? "active" : "pending";
        return (
          <li
            key={label}
            data-testid={`hire-step3-stage-${idx}`}
            data-state={state}
            className={cn(
              "flex items-center gap-3 text-sm transition-opacity",
              state === "pending" && "opacity-40",
              state === "done" && "text-[var(--foreground)]",
              state === "active" && "text-[var(--foreground)]",
            )}
          >
            <span
              aria-hidden
              className={cn(
                "grid size-5 place-items-center rounded-full border",
                state === "done"
                  ? "border-[var(--primary)] bg-[var(--primary)] text-[var(--primary-foreground)]"
                  : state === "active"
                    ? "border-[var(--primary)] text-[var(--primary)]"
                    : "border-[var(--border)] text-[var(--muted-foreground)]",
              )}
            >
              {state === "done" ? (
                <Check className="size-3" />
              ) : state === "active" ? (
                <Loader2 className="size-3 animate-spin" />
              ) : (
                <span className="size-1.5 rounded-full bg-current" />
              )}
            </span>
            <span className={cn(state === "pending" && "text-[var(--muted-foreground)]")}>
              {label}
            </span>
          </li>
        );
      })}
    </ol>
  );
}
