import { cn } from "@/lib/utils";

/**
 * Loading placeholder for the `/retro` tenant-wide index. Shape-matches
 * the live retro card list (worker line, window, metrics, preview body)
 * so the layout doesn't shift between pending → resolved. Used in place
 * of a literal "Loading…" string to remove the half-finished feel on
 * cold SSR / first hydration.
 */
export interface RetroListSkeletonProps {
  className?: string;
  /** How many placeholder rows to render. Defaults to 3. */
  rows?: number;
}

export function RetroListSkeleton({
  className,
  rows = 3,
}: RetroListSkeletonProps) {
  return (
    <ul
      data-testid="retro-index-skeleton"
      role="status"
      aria-live="polite"
      aria-label="Loading retros"
      className={cn("space-y-3", className)}
    >
      {Array.from({ length: rows }).map((_, i) => (
        <li
          key={i}
          data-testid="retro-index-skeleton-row"
          aria-hidden
          className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4 animate-pulse"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="space-y-2">
              <div className="h-4 w-40 rounded bg-[var(--muted)]" />
              <div className="h-3 w-28 rounded bg-[var(--muted)]" />
            </div>
            <div className="h-5 w-16 rounded-full bg-[var(--muted)]" />
          </div>
          <div className="mt-3 h-3 w-3/5 rounded bg-[var(--muted)]" />
          <div className="mt-3 space-y-2">
            <div className="h-3 w-1/4 rounded bg-[var(--muted)]" />
            <div className="h-3 w-11/12 rounded bg-[var(--muted)]" />
            <div className="h-3 w-9/12 rounded bg-[var(--muted)]" />
          </div>
        </li>
      ))}
    </ul>
  );
}
