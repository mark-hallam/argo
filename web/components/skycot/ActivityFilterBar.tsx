"use client";

import { useId } from "react";
import { cn } from "@/lib/utils";
import {
  WORKER_ACTIVITY_ACTION_LABELS,
  WORKER_ACTIVITY_ACTION_TYPES,
  WORKER_ACTIVITY_RANGES,
  WORKER_ACTIVITY_RANGE_LABELS,
  WORKER_ACTIVITY_STATUSES,
  WORKER_ACTIVITY_STATUS_LABELS,
  type WorkerActivityActionType,
  type WorkerActivityFilters,
  type WorkerActivityRange,
  type WorkerActivityStatus,
} from "@/lib/api/worker-activity";

/**
 * Light filter bar above the per-worker Activity feed. Three
 * dropdowns: time range / action type / status. Native `<select>`
 * elements — the SKYCOT design system has no Dropdown primitive yet,
 * and native selects round-trip to the URL/keyboard-accessible
 * without extra work. When a dropdown primitive lands later,
 * swap-in is one component.
 *
 * The bar is dumb — it owns no state. Parent owns the
 * `WorkerActivityFilters` object and reacts to `onChange` by
 * pushing the new URL (`?range=…&type=…&status=…`) so the filter
 * state survives reload.
 */

export interface ActivityFilterBarProps {
  filters: WorkerActivityFilters;
  onChange: (next: WorkerActivityFilters) => void;
  className?: string;
  /** Disables interaction during a refetch. */
  disabled?: boolean;
}

export function ActivityFilterBar({
  filters,
  onChange,
  className,
  disabled,
}: ActivityFilterBarProps) {
  const rangeId = useId();
  const typeId = useId();
  const statusId = useId();

  return (
    <div
      data-testid="activity-filter-bar"
      className={cn(
        "flex flex-wrap items-end gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/60 p-3",
        className,
      )}
    >
      <FilterField label="Time range" htmlFor={rangeId}>
        <select
          id={rangeId}
          data-testid="activity-filter-range"
          aria-label="Filter by time range"
          disabled={disabled}
          value={filters.range}
          onChange={(e) =>
            onChange({ ...filters, range: e.target.value as WorkerActivityRange })
          }
          className={selectClass}
        >
          {WORKER_ACTIVITY_RANGES.map((r) => (
            <option key={r} value={r}>
              {WORKER_ACTIVITY_RANGE_LABELS[r]}
            </option>
          ))}
        </select>
      </FilterField>

      <FilterField label="Action type" htmlFor={typeId}>
        <select
          id={typeId}
          data-testid="activity-filter-type"
          aria-label="Filter by action type"
          disabled={disabled}
          value={filters.actionType ?? "all"}
          onChange={(e) => {
            const v = e.target.value;
            onChange({
              ...filters,
              actionType: v === "all" ? undefined : (v as WorkerActivityActionType),
            });
          }}
          className={selectClass}
        >
          <option value="all">All</option>
          {WORKER_ACTIVITY_ACTION_TYPES.map((t) => (
            <option key={t} value={t}>
              {WORKER_ACTIVITY_ACTION_LABELS[t]}
            </option>
          ))}
        </select>
      </FilterField>

      <FilterField label="Status" htmlFor={statusId}>
        <select
          id={statusId}
          data-testid="activity-filter-status"
          aria-label="Filter by status"
          disabled={disabled}
          value={filters.status ?? "all"}
          onChange={(e) => {
            const v = e.target.value;
            onChange({
              ...filters,
              status: v === "all" ? undefined : (v as WorkerActivityStatus),
            });
          }}
          className={selectClass}
        >
          <option value="all">All</option>
          {WORKER_ACTIVITY_STATUSES.map((s) => (
            <option key={s} value={s}>
              {WORKER_ACTIVITY_STATUS_LABELS[s]}
            </option>
          ))}
        </select>
      </FilterField>
    </div>
  );
}

const selectClass = cn(
  "block rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)]",
  "px-2.5 py-1.5 text-sm text-[var(--foreground)]",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--background)]",
  "disabled:cursor-not-allowed disabled:opacity-60",
);

function FilterField({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label
        htmlFor={htmlFor}
        className="text-[11px] font-medium uppercase tracking-wide text-[var(--muted-foreground)]"
      >
        {label}
      </label>
      {children}
    </div>
  );
}
