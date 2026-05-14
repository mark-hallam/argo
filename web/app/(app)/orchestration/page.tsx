"use client";

import { useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { Loader2 } from "lucide-react";
import { QueueTable } from "@/components/skycot/orch/QueueTable";
import { SlaCard } from "@/components/skycot/orch/SlaCard";
import {
  EMPTY_ORCHESTRATION_FILTERS,
  applyOrchestrationFilters,
  cancelRun,
  fetchOrchestrationRuns,
  forceEscalate,
  resetRetry,
  summarizeRuns,
  type FetchOrchestrationResult,
  type OrchestrationFilters,
  type OrchestrationStatus,
  type SlaState,
} from "@/lib/api/orchestration";

export default function OrchestrationPage() {
  const [data, setData] = useState<FetchOrchestrationResult | null>(null);
  const [filters, setFilters] = useState<OrchestrationFilters>(EMPTY_ORCHESTRATION_FILTERS);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    fetchOrchestrationRuns()
      .then((next) => {
        if (alive) setData(next);
      })
      .catch((err) => {
        if (alive) setError(err instanceof Error ? err.message : "Failed to load queue");
      });
    return () => {
      alive = false;
    };
  }, []);

  const runs = data?.runs ?? [];
  const filtered = useMemo(() => applyOrchestrationFilters(runs, filters), [runs, filters]);
  const summary = useMemo(() => summarizeRuns(filtered), [filtered]);
  const workers = useMemo(() => [...new Set(runs.map((run) => run.worker))].sort(), [runs]);
  const skills = useMemo(() => [...new Set(runs.map((run) => run.skill))].sort(), [runs]);

  async function runAction(label: string, op: () => Promise<void>) {
    await op();
    setToast(label);
    window.setTimeout(() => setToast(null), 1800);
  }

  return (
    <>
      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-6xl px-4 py-5 sm:px-6 lg:px-8">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="space-y-1">
              <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
                Orchestration
              </h1>
              <p className="text-sm text-[var(--muted-foreground)]" data-testid="orchestration-subtitle">
                {data ? `${filtered.length} runs shown from ${data.source}` : "Loading queue"}
              </p>
            </div>
            <FilterBar filters={filters} setFilters={setFilters} workers={workers} skills={skills} />
          </div>
        </div>
      </div>

      <main className="mx-auto max-w-6xl space-y-5 px-4 py-6 sm:px-6 lg:px-8" data-testid="orchestration-page">
        {toast ? (
          <div className="rounded-[var(--radius-md)] border border-emerald-300 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
            {toast}
          </div>
        ) : null}
        {error ? <ErrorState message={error} /> : null}
        {!data && !error ? <LoadingState /> : null}
        {data ? (
          <>
            <SlaCard summary={summary} />
            <div className="overflow-x-auto">
              <QueueTable
                runs={filtered}
                onCancel={(id) => void runAction("Run cancelled", () => cancelRun(id))}
                onResetRetry={(id) => void runAction("Retry reset", () => resetRetry(id))}
                onForceEscalate={(id) => void runAction("Escalation forced", () => forceEscalate(id))}
              />
            </div>
          </>
        ) : null}
      </main>
    </>
  );
}

function FilterBar({
  filters,
  setFilters,
  workers,
  skills,
}: {
  filters: OrchestrationFilters;
  setFilters: Dispatch<SetStateAction<OrchestrationFilters>>;
  workers: string[];
  skills: string[];
}) {
  return (
    <div className="grid grid-cols-2 gap-2 lg:grid-cols-4" data-testid="orchestration-filters">
      <Select label="Worker" value={filters.worker} onChange={(worker) => setFilters((f) => ({ ...f, worker }))} options={workers} />
      <Select label="Skill" value={filters.skill} onChange={(skill) => setFilters((f) => ({ ...f, skill }))} options={skills} />
      <Select
        label="Status"
        value={filters.status}
        onChange={(status) => setFilters((f) => ({ ...f, status: status as OrchestrationStatus | "all" }))}
        options={["all", "queued", "running", "waiting", "blocked", "done"]}
      />
      <Select
        label="SLA"
        value={filters.slaState}
        onChange={(slaState) => setFilters((f) => ({ ...f, slaState: slaState as SlaState | "all" }))}
        options={["all", "healthy", "warning", "breached"]}
      />
    </div>
  );
}

function Select({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: string[];
}) {
  return (
    <label className="space-y-1 text-xs font-medium text-[var(--muted-foreground)]">
      <span>{label}</span>
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-9 w-full rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-2 text-sm text-[var(--foreground)]"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {option || "all"}
          </option>
        ))}
      </select>
    </label>
  );
}

function LoadingState() {
  return <Loader2 className="mx-auto mt-20 size-5 animate-spin text-[var(--muted-foreground)]" />;
}

function ErrorState({ message }: { message: string }) {
  return (
    <div className="rounded-[var(--radius-md)] border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700">
      {message}
    </div>
  );
}
