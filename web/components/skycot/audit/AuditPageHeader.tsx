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
 * Sticky B3-enterprise header for `/audit`. URL state lives on the
 * parent page; this component only renders controls and callbacks.
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
    <div
      data-testid="audit-page-header"
      className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/90 backdrop-blur-md"
    >
      <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-5">
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div className="min-w-0 space-y-1">
            <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
              Audit · Chain explorer
            </p>
            <h1
              className="text-2xl leading-tight text-[var(--foreground)] sm:text-3xl"
              style={{ fontFamily: "var(--font-audiowide)" }}
            >
              Audit{" "}
              <span
                style={{
                  background: "var(--brand-gradient-3)",
                  WebkitBackgroundClip: "text",
                  backgroundClip: "text",
                  color: "transparent",
                }}
              >
                chain
              </span>
            </h1>
            <p
              className="text-sm text-[var(--muted-foreground)]"
              data-testid="audit-subtitle"
            >
              Every live action, hash-linked and replayable.
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
            onKeyDown={(e) => {
              // Defend against global ⌘K listener seeing our keystrokes if a
              // mid-render focus blip flips event ordering. The chord itself
              // (Cmd/Ctrl+K) still bubbles so the palette can open from here.
              if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
            }}
            onKeyUp={(e) => {
              if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
            }}
            placeholder="Search action, worker, hash, or action ID..."
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
