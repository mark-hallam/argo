"use client";

/**
 * Filter + pagination chrome for the workflow runs table.
 *
 * Sits above [`WorkflowRunsTable`] and emits the state it owns
 * (status chips, date-range preset, page, page_size) back to the
 * parent so the parent can serialise it into the URL.
 *
 * Pure presentational + state-callback — no router or fetch side
 * effects. The parent (`page.tsx`) owns URL state so the filtered
 * view is shareable / bookmarkable per the brief acceptance.
 */

import type { WorkflowRunStatus } from "@/lib/api/workflow-runs";

export type RunsRangePreset = "24h" | "7d" | "30d" | "custom";

export const RANGE_PRESET_LABELS: Record<RunsRangePreset, string> = {
  "24h": "Last 24h",
  "7d": "Last 7d",
  "30d": "Last 30d",
  custom: "Custom",
};

export const STATUS_CHIPS: { value: WorkflowRunStatus; label: string }[] = [
  { value: "success", label: "Success" },
  { value: "failed", label: "Failed" },
  { value: "running", label: "Running" },
  { value: "queued", label: "Queued" },
];

export const PAGE_SIZES = [25, 50, 100] as const;
export type RunsPageSize = (typeof PAGE_SIZES)[number];

export interface RunsFilterState {
  range: RunsRangePreset;
  /** ISO yyyy-mm-dd; used only when `range === 'custom'`. */
  customSince: string | null;
  customUntil: string | null;
  /** Empty = show all statuses. */
  statuses: WorkflowRunStatus[];
  page: number;
  pageSize: RunsPageSize;
}

export const DEFAULT_FILTER_STATE: RunsFilterState = {
  range: "7d",
  customSince: null,
  customUntil: null,
  statuses: [],
  page: 1,
  pageSize: 25,
};

interface Props {
  state: RunsFilterState;
  onChange: (next: RunsFilterState) => void;
  /** Total filtered rows reported by the gateway. */
  totalFiltered: number;
  /** Rows currently rendered (may be less than pageSize on last page). */
  visibleCount: number;
  /** When true, hide the bar but reserve layout space. */
  isLoading?: boolean;
}

export function WorkflowRunsFilterBar({
  state,
  onChange,
  totalFiltered,
  visibleCount,
  isLoading,
}: Props) {
  const setRange = (range: RunsRangePreset) => {
    onChange({ ...state, range, page: 1 });
  };
  const toggleStatus = (s: WorkflowRunStatus) => {
    const next = state.statuses.includes(s)
      ? state.statuses.filter((x) => x !== s)
      : [...state.statuses, s];
    onChange({ ...state, statuses: next, page: 1 });
  };
  const setPageSize = (pageSize: RunsPageSize) => {
    onChange({ ...state, pageSize, page: 1 });
  };
  const setCustom = (kind: "since" | "until", value: string) => {
    onChange({
      ...state,
      range: "custom",
      ...(kind === "since"
        ? { customSince: value || null }
        : { customUntil: value || null }),
      page: 1,
    });
  };
  const clearAll = () => onChange({ ...DEFAULT_FILTER_STATE });

  const pageStart =
    totalFiltered === 0 ? 0 : (state.page - 1) * state.pageSize + 1;
  const pageEnd =
    totalFiltered === 0 ? 0 : pageStart + Math.max(visibleCount, 0) - 1;
  const isLastPage = pageEnd >= totalFiltered;
  const hasFilters =
    state.statuses.length > 0 ||
    state.range !== DEFAULT_FILTER_STATE.range ||
    state.pageSize !== DEFAULT_FILTER_STATE.pageSize;

  return (
    <div
      data-testid="wfd-runs-filter-bar"
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: 12,
        alignItems: "center",
        padding: "12px 0",
        fontFamily: "Inter, sans-serif",
        fontSize: 12,
        color: "var(--muted-foreground, #52525b)",
      }}
    >
      {/* range presets */}
      <div role="radiogroup" aria-label="Date range" style={{ display: "flex", gap: 4 }}>
        {(Object.keys(RANGE_PRESET_LABELS) as RunsRangePreset[]).map((preset) => (
          <button
            key={preset}
            type="button"
            data-testid={`wfd-range-${preset}`}
            role="radio"
            aria-checked={state.range === preset}
            onClick={() => setRange(preset)}
            style={chipStyle(state.range === preset)}
          >
            {RANGE_PRESET_LABELS[preset]}
          </button>
        ))}
      </div>

      {state.range === "custom" ? (
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input
            type="date"
            aria-label="Custom range start"
            data-testid="wfd-custom-since"
            value={state.customSince ?? ""}
            onChange={(e) => setCustom("since", e.target.value)}
            style={dateInputStyle()}
          />
          <span>→</span>
          <input
            type="date"
            aria-label="Custom range end"
            data-testid="wfd-custom-until"
            value={state.customUntil ?? ""}
            onChange={(e) => setCustom("until", e.target.value)}
            style={dateInputStyle()}
          />
        </div>
      ) : null}

      {/* status chips */}
      <div
        role="group"
        aria-label="Filter by status"
        style={{ display: "flex", gap: 4, marginLeft: 8 }}
      >
        {STATUS_CHIPS.map((s) => {
          const active = state.statuses.includes(s.value);
          return (
            <button
              key={s.value}
              type="button"
              data-testid={`wfd-status-${s.value}`}
              aria-pressed={active}
              onClick={() => toggleStatus(s.value)}
              style={chipStyle(active)}
            >
              {s.label}
            </button>
          );
        })}
      </div>

      {/* page size + showing X of Y */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginLeft: "auto" }}>
        <label
          htmlFor="wfd-page-size"
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            letterSpacing: "0.14em",
            textTransform: "uppercase",
          }}
        >
          Rows
        </label>
        <select
          id="wfd-page-size"
          data-testid="wfd-page-size"
          value={state.pageSize}
          onChange={(e) =>
            setPageSize(Number(e.target.value) as RunsPageSize)
          }
          style={{
            border: "1px solid var(--border, #ece9f5)",
            borderRadius: 6,
            padding: "4px 8px",
            fontSize: 12,
            background: "white",
          }}
        >
          {PAGE_SIZES.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <span
          data-testid="wfd-pagination-indicator"
          style={{ fontVariantNumeric: "tabular-nums" }}
        >
          {isLoading
            ? "Loading…"
            : totalFiltered === 0
              ? "0 of 0"
              : `Showing ${pageStart}-${pageEnd} of ${totalFiltered.toLocaleString()}`}
        </span>
        <button
          type="button"
          data-testid="wfd-prev-page"
          disabled={state.page <= 1 || isLoading}
          onClick={() => onChange({ ...state, page: state.page - 1 })}
          style={pagerButtonStyle(state.page <= 1 || !!isLoading)}
        >
          ‹ Prev
        </button>
        <button
          type="button"
          data-testid="wfd-next-page"
          disabled={isLastPage || isLoading}
          onClick={() => onChange({ ...state, page: state.page + 1 })}
          style={pagerButtonStyle(isLastPage || !!isLoading)}
        >
          Next ›
        </button>
        {hasFilters ? (
          <button
            type="button"
            data-testid="wfd-clear-filters"
            onClick={clearAll}
            style={{
              ...chipStyle(false),
              borderColor: "transparent",
              textDecoration: "underline",
            }}
          >
            Clear
          </button>
        ) : null}
      </div>
    </div>
  );
}

function chipStyle(active: boolean): React.CSSProperties {
  return {
    border: `1px solid ${active ? "#8b5cf6" : "var(--border, #ece9f5)"}`,
    background: active ? "rgba(139, 92, 246, 0.08)" : "white",
    color: active ? "#5b21b6" : "var(--muted-foreground, #52525b)",
    borderRadius: 999,
    padding: "4px 12px",
    fontSize: 12,
    cursor: "pointer",
    fontFamily: "Inter, sans-serif",
    fontWeight: active ? 600 : 500,
  };
}

function pagerButtonStyle(disabled: boolean): React.CSSProperties {
  return {
    border: "1px solid var(--border, #ece9f5)",
    background: "white",
    borderRadius: 6,
    padding: "4px 10px",
    fontSize: 12,
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
    fontFamily: "Inter, sans-serif",
  };
}

function dateInputStyle(): React.CSSProperties {
  return {
    border: "1px solid var(--border, #ece9f5)",
    borderRadius: 6,
    padding: "4px 8px",
    fontSize: 12,
    background: "white",
    fontFamily: "Inter, sans-serif",
  };
}
