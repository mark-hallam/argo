import { cn } from "@/lib/utils";

/**
 * Placeholder card used in the Team grid until the real
 * TeammateCard composite lands in brief E3-b. Shape-matches the
 * card the composite will render (avatar circle, name + tagline,
 * status pill, two short result lines) so the grid layout doesn't
 * shift when E3-b slots in.
 *
 * The skeleton is purely visual — no interactions, no real data.
 */
export interface TeammateCardSkeletonProps {
  className?: string;
  /** Forwarded to outer div for tests. */
  "data-testid"?: string;
}

export function TeammateCardSkeleton({
  className,
  "data-testid": testId,
}: TeammateCardSkeletonProps) {
  return (
    <div
      data-testid={testId ?? "teammate-card-skeleton"}
      aria-hidden
      className={cn(
        "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-4 animate-pulse",
        className,
      )}
    >
      <div className="flex items-start gap-3">
        <div className="size-10 rounded-full bg-[var(--muted)]" />
        <div className="flex-1 space-y-2">
          <div className="h-3 w-2/5 rounded bg-[var(--muted)]" />
          <div className="h-2.5 w-3/4 rounded bg-[var(--muted)]" />
        </div>
        <div className="h-5 w-14 rounded-[var(--radius-sm)] bg-[var(--muted)]" />
      </div>

      <div className="space-y-2">
        <div className="h-2.5 w-5/6 rounded bg-[var(--muted)]" />
        <div className="h-2.5 w-2/3 rounded bg-[var(--muted)]" />
      </div>
    </div>
  );
}
