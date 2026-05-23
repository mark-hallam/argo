import {
  DEFAULT_FILTER_STATE,
  PAGE_SIZES,
  type RunsFilterState,
  type RunsPageSize,
  type RunsRangePreset,
} from "./runs-filter-bar";
import type { WorkflowRunStatus } from "@/lib/api/workflow-runs";
import { WORKFLOW_RUN_STATUSES } from "@/lib/api/workflow-runs";

const SECTION_LABEL_BY_RANGE: Record<RunsRangePreset, string> = {
  "24h": "LAST 24 HOURS",
  "7d": "LAST 7 DAYS",
  "30d": "LAST 30 DAYS",
  custom: "CUSTOM RANGE",
};

const VALID_RANGES: ReadonlySet<RunsRangePreset> = new Set([
  "24h",
  "7d",
  "30d",
  "custom",
]);

const HOURS_24_MS = 24 * 60 * 60 * 1_000;
const DAYS_7_MS = 7 * HOURS_24_MS;
const DAYS_30_MS = 30 * HOURS_24_MS;

export function sectionLabelForRange(r: RunsRangePreset): string {
  return SECTION_LABEL_BY_RANGE[r] ?? SECTION_LABEL_BY_RANGE["7d"];
}

export function parseFilterStateFromUrl(
  searchParams: URLSearchParams | { get(name: string): string | null } | null,
): RunsFilterState {
  const rawRange = searchParams?.get("range") ?? null;
  const range =
    rawRange && VALID_RANGES.has(rawRange as RunsRangePreset)
      ? (rawRange as RunsRangePreset)
      : DEFAULT_FILTER_STATE.range;
  const rawStatuses = (searchParams?.get("status") ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter((s): s is WorkflowRunStatus =>
      WORKFLOW_RUN_STATUSES.includes(s as WorkflowRunStatus),
    );
  const rawPage = Number.parseInt(searchParams?.get("page") ?? "", 10);
  const rawPageSize = Number.parseInt(searchParams?.get("page_size") ?? "", 10);
  const pageSize = PAGE_SIZES.includes(rawPageSize as RunsPageSize)
    ? (rawPageSize as RunsPageSize)
    : DEFAULT_FILTER_STATE.pageSize;
  return {
    range,
    customSince: searchParams?.get("since") ?? null,
    customUntil: searchParams?.get("until") ?? null,
    statuses: rawStatuses,
    page: Number.isFinite(rawPage) && rawPage > 0 ? rawPage : 1,
    pageSize,
  };
}

export function writeFilterStateToParams(
  params: URLSearchParams,
  state: RunsFilterState,
): void {
  if (state.range === DEFAULT_FILTER_STATE.range) {
    params.delete("range");
  } else {
    params.set("range", state.range);
  }
  if (state.range === "custom") {
    if (state.customSince) params.set("since", state.customSince);
    else params.delete("since");
    if (state.customUntil) params.set("until", state.customUntil);
    else params.delete("until");
  } else {
    params.delete("since");
    params.delete("until");
  }
  if (state.statuses.length > 0) {
    params.set("status", state.statuses.join(","));
  } else {
    params.delete("status");
  }
  if (state.page === 1) params.delete("page");
  else params.set("page", String(state.page));
  if (state.pageSize === DEFAULT_FILTER_STATE.pageSize) {
    params.delete("page_size");
  } else {
    params.set("page_size", String(state.pageSize));
  }
}

export function resolveDateRange(state: RunsFilterState): {
  sinceTs: number | undefined;
  untilTs: number | undefined;
} {
  const now = Date.now();
  if (state.range === "custom") {
    return {
      sinceTs: state.customSince ? Date.parse(state.customSince) * 1_000 : undefined,
      untilTs: state.customUntil
        ? (Date.parse(state.customUntil) + HOURS_24_MS) * 1_000
        : undefined,
    };
  }
  const offsetMs =
    state.range === "24h"
      ? HOURS_24_MS
      : state.range === "30d"
        ? DAYS_30_MS
        : DAYS_7_MS;
  return { sinceTs: (now - offsetMs) * 1_000, untilTs: undefined };
}
