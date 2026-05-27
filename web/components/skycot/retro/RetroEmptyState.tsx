import Link from "next/link";

/**
 * Empty-state for `/retro` tenant-wide index. Two modes:
 * - `q` set: search came back empty — prompt a broader query.
 * - `q` blank: no retros at all — explain the per-worker entry point
 *   and offer a CTA to the Workers grid where retros are kicked off
 *   from a worker profile.
 */
export interface RetroEmptyStateProps {
  q: string;
}

export function RetroEmptyState({ q }: RetroEmptyStateProps) {
  const searching = q.length > 0;
  return (
    <div
      data-testid="retro-index-empty"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
    >
      <h2 className="text-base font-semibold text-[var(--foreground)]">
        {searching ? "No retros match your search" : "No retros yet"}
      </h2>
      <p className="text-sm text-[var(--muted-foreground)]">
        {searching
          ? "Try a broader theme — sections are searched case-insensitively."
          : "Capture what worked + what didn't last sprint. Retros are kicked off from a worker profile after a long-running schedule wraps."}
      </p>
      {searching ? null : (
        <p className="pt-2">
          <Link
            href="/workers"
            data-testid="retro-index-empty-cta"
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-4 py-2 text-sm font-medium text-[var(--primary-foreground)] transition-opacity hover:opacity-90"
          >
            Browse workers →
          </Link>
        </p>
      )}
    </div>
  );
}
