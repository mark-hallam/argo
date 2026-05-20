import { cn } from "@/lib/utils";

/**
 * Loading placeholder for the C2 holographic-showcase TeammateCard.
 * Shape-matches the live card (gradient banner block, portrait notch,
 * name + role lines, trait chip row, two body lines) so the grid
 * doesn't visibly shift between loading and loaded.
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
        "rounded-[24px] overflow-hidden bg-[var(--card)] animate-pulse",
        "shadow-[0_1px_2px_rgba(0,0,0,0.06),0_32px_60px_-16px_rgba(139,92,246,0.18)]",
        "ring-2 ring-[rgba(139,92,246,0.10)] ring-inset",
        "flex flex-col",
        className,
      )}
    >
      <div className="h-[120px] bg-gradient-to-br from-[#8b5cf6]/40 to-[#ec4899]/40" />
      <div className="-mt-14 ml-5 size-[92px] rounded-[26px] bg-[var(--card)] p-[5px]">
        <div className="size-full rounded-[21px] bg-[var(--muted)]" />
      </div>
      <div className="px-6 pt-3 space-y-2">
        <div className="h-5 w-2/5 rounded bg-[var(--muted)]" />
        <div className="h-3 w-3/5 rounded bg-[var(--muted)]" />
        <div className="h-5 w-20 rounded-full bg-[var(--muted)]" />
      </div>
      <div className="px-6 pt-4 flex gap-2 flex-wrap">
        <div className="h-5 w-16 rounded-full bg-[var(--muted)]" />
        <div className="h-5 w-20 rounded-full bg-[var(--muted)]" />
        <div className="h-5 w-12 rounded-full bg-[var(--muted)]" />
      </div>
      <div className="px-6 pt-4 pb-6 space-y-2">
        <div className="h-2.5 w-4/5 rounded bg-[var(--muted)]" />
        <div className="h-2.5 w-2/3 rounded bg-[var(--muted)]" />
      </div>
    </div>
  );
}
