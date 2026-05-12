"use client";

import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { WORKER_STATUS_LABELS } from "@/lib/types/worker";
import { isTeamFilterEmpty, type TeamFilters } from "@/lib/filters/team";

export interface TeamFilterChipRailProps {
  filters: TeamFilters;
  onClearDimension: (dimension: keyof TeamFilters) => void;
  onClearAll: () => void;
}

export function TeamFilterChipRail({
  filters,
  onClearDimension,
  onClearAll,
}: TeamFilterChipRailProps) {
  if (isTeamFilterEmpty(filters)) return null;

  return (
    <div
      className="mt-3 flex flex-wrap items-center gap-2"
      data-testid="team-filter-chip-rail"
    >
      <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
        Filters
      </span>
      {filters.status ? (
        <Chip
          label={`Status: ${WORKER_STATUS_LABELS[filters.status]}`}
          onRemove={() => onClearDimension("status")}
          testId="team-filter-chip-status"
        />
      ) : null}
      {filters.q.length > 0 ? (
        <Chip
          label={`Search: "${filters.q}"`}
          onRemove={() => onClearDimension("q")}
          testId="team-filter-chip-search"
        />
      ) : null}
      <button
        type="button"
        onClick={onClearAll}
        className="ml-auto text-xs text-[var(--primary)] underline-offset-4 hover:underline"
        data-testid="team-filter-chip-rail-clear-all"
      >
        Clear all
      </button>
    </div>
  );
}

function Chip({
  label,
  onRemove,
  testId,
}: {
  label: string;
  onRemove: () => void;
  testId: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full",
        "border border-[var(--border)] bg-[var(--card)]/60 px-2.5 py-1 text-xs text-[var(--foreground)]",
      )}
      data-testid={testId}
    >
      <span>{label}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${label} filter`}
        className="text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        data-testid={`${testId}-remove`}
      >
        <X className="size-3" />
      </button>
    </span>
  );
}
