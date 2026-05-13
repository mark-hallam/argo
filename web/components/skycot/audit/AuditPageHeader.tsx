"use client";

import type { ReactNode } from "react";
import { ChevronDown, Download, Filter, Search } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  AUDIT_RANGE_LABELS,
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  isCustomRange,
  type AuditRange,
  type AuditRangePreset,
} from "@/lib/types/audit";

/**
 * Sticky page header for the `/audit` route. Renders:
 *  - title "Audit" + subtitle "Forensic + compliance view"
 *  - action row: Filter trigger (icon-only — E5-b ships the popover
 *    via the optional `filterTrigger` slot; absent ⇒ legacy button
 *    wired to `onFilterClick`), time-range dropdown (8 options +
 *    Custom), Export trigger (E5-e implements the dialog)
 *  - full-width search input below
 *
 * URL state lives on the parent page; this component is presentational
 * and routes every change through the supplied callbacks.
 */

const CUSTOM_VALUE = "custom";

export interface AuditPageHeaderProps {
  range: AuditRange;
  query: string;
  onRangeChange: (range: AuditRangePreset | "custom") => void;
  onQueryChange: (value: string) => void;
  onExportClick: () => void;
  /** Legacy click handler — used only when `filterTrigger` is absent. */
  onFilterClick?: () => void;
  /** Slot for the E5-b filter-popover trigger; replaces the inline
   *  button when provided so the parent owns popover lifecycle. */
  filterTrigger?: ReactNode;
}

export function AuditPageHeader({
  range,
  query,
  onRangeChange,
  onQueryChange,
  onFilterClick,
  onExportClick,
  filterTrigger,
}: AuditPageHeaderProps) {
  const selectValue: AuditRangePreset | "custom" = isCustomRange(range)
    ? "custom"
    : range;

  return (
    <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Audit
            </h1>
            <p
              className="text-sm text-[var(--muted-foreground)]"
              data-testid="audit-subtitle"
            >
              Forensic + compliance view
            </p>
          </div>

          <div className="flex items-center gap-2">
            {filterTrigger ?? (
              <button
                type="button"
                aria-label="Open filter popover"
                data-testid="audit-filter-trigger"
                onClick={onFilterClick}
                className={cn(buttonVariants({ variant: "secondary", size: "sm" }), "px-2")}
              >
                <Filter aria-hidden className="size-4" />
                <span className="sr-only">Filter</span>
              </button>
            )}

            <RangeSelect
              value={selectValue}
              onChange={(v) => onRangeChange(v)}
            />

            <button
              type="button"
              data-testid="audit-export-trigger"
              onClick={onExportClick}
              className={buttonVariants({ variant: "secondary", size: "sm" })}
            >
              <Download aria-hidden className="size-4" />
              <span className="hidden sm:inline">Export</span>
            </button>
          </div>
        </div>

        <div className="mt-4 relative">
          <Search
            aria-hidden
            className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]"
          />
          <input
            type="search"
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            placeholder="Search action, worker, or action ID..."
            aria-label="Search audit timeline"
            data-testid="audit-search-input"
            className="w-full h-10 pl-9 pr-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
          />
        </div>
      </div>
    </div>
  );
}

function RangeSelect({
  value,
  onChange,
}: {
  value: AuditRangePreset | typeof CUSTOM_VALUE;
  onChange: (v: AuditRangePreset | typeof CUSTOM_VALUE) => void;
}) {
  return (
    <label className="relative inline-flex items-center text-sm">
      <span className="sr-only">Time range</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as AuditRangePreset | typeof CUSTOM_VALUE)}
        aria-label="Time range"
        data-testid="audit-range-select"
        className="appearance-none h-9 pl-3 pr-8 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
      >
        {AUDIT_RANGE_PRESETS.map((r) => (
          <option key={r} value={r}>
            {AUDIT_RANGE_LABELS[r]}
            {r === DEFAULT_AUDIT_RANGE ? " (default)" : ""}
          </option>
        ))}
        <option value={CUSTOM_VALUE}>Custom range…</option>
      </select>
      <ChevronDown
        aria-hidden
        className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]"
      />
    </label>
  );
}
