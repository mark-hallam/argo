import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * Responsive wrapper for the Team page grid.
 *
 * Breakpoints (per the wireframe + brief E3-a):
 *   < 640px → 1 column
 *   640-1024px → 2 columns
 *   > 1024px → 3 columns
 *
 * Children render in source order. The grid itself does no sorting
 * or filtering — callers pre-sort with `compareWorkersForGrid`.
 */
export interface TeammateGridProps {
  children: ReactNode;
  className?: string;
}

export function TeammateGrid({ children, className }: TeammateGridProps) {
  return (
    <div
      data-testid="teammate-grid"
      className={cn(
        "grid gap-4 sm:gap-5",
        "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
        className,
      )}
    >
      {children}
    </div>
  );
}
