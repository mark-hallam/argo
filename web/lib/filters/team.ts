// Pure filter logic for the team page (E3-d). Two dimensions
// (status / search) combined as AND. Status is a single WorkerStatus
// (or null = "all"); search is a case-insensitive substring matched
// against `worker.name` + `worker.role.tagline`. State lives in URL
// search params (`?status=working&q=mariah`).

import {
  WORKER_STATUSES,
  type Worker,
  type WorkerStatus,
} from "@/lib/types/worker";

export interface TeamFilters {
  status: WorkerStatus | null;
  q: string;
}

export const EMPTY_TEAM_FILTERS: TeamFilters = { status: null, q: "" };

function isWorkerStatus(value: string): value is WorkerStatus {
  return (WORKER_STATUSES as readonly string[]).includes(value);
}

// Hand-typed URLs with unknown ?status= values are coerced to "all"
// (null) so the page renders the full grid instead of an empty state.
export function parseTeamFilters(
  params: URLSearchParams | { get(name: string): string | null },
): TeamFilters {
  const rawStatus = params.get("status");
  const rawQ = params.get("q");
  return {
    status: rawStatus && isWorkerStatus(rawStatus) ? rawStatus : null,
    q: (rawQ ?? "").trim(),
  };
}

export function serializeTeamFilters(filters: TeamFilters): string {
  const params = new URLSearchParams();
  if (filters.status) params.set("status", filters.status);
  if (filters.q.length > 0) params.set("q", filters.q);
  return params.toString();
}

export function isTeamFilterEmpty(filters: TeamFilters): boolean {
  return filters.status === null && filters.q.length === 0;
}

export function activeTeamFilterCount(filters: TeamFilters): number {
  return (filters.status ? 1 : 0) + (filters.q.length > 0 ? 1 : 0);
}

function workerMatchesQuery(worker: Worker, q: string): boolean {
  if (!q) return true;
  const needle = q.toLowerCase();
  return (
    worker.name.toLowerCase().includes(needle) ||
    worker.role.tagline.toLowerCase().includes(needle)
  );
}

export function applyTeamFilters(
  workers: Worker[],
  filters: TeamFilters,
): Worker[] {
  if (isTeamFilterEmpty(filters)) return workers;
  return workers.filter((w) => {
    if (filters.status && w.status !== filters.status) return false;
    if (!workerMatchesQuery(w, filters.q)) return false;
    return true;
  });
}

export function clearTeamDimension(
  filters: TeamFilters,
  dimension: keyof TeamFilters,
): TeamFilters {
  if (dimension === "status") return { ...filters, status: null };
  return { ...filters, q: "" };
}
