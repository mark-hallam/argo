"use client";

import { AlertTriangle, CircleStop, RotateCcw, Send } from "lucide-react";
import type { ComponentType } from "react";
import { RetryBadge } from "@/components/skycot/orch/RetryBadge";
import type { OrchestrationRun } from "@/lib/api/orchestration";
import { cn } from "@/lib/utils";

export function QueueTable({
  runs,
  onCancel,
  onResetRetry,
  onForceEscalate,
}: {
  runs: OrchestrationRun[];
  onCancel: (id: string) => void;
  onResetRetry: (id: string) => void;
  onForceEscalate: (id: string) => void;
}) {
  return (
    <div className="overflow-hidden rounded-[var(--radius-md)] border border-[var(--border)]">
      <table className="w-full min-w-[760px] text-left text-sm" data-testid="orchestration-table">
        <thead className="bg-[var(--muted)] text-xs uppercase text-[var(--muted-foreground)]">
          <tr>
            <th className="px-4 py-3 font-semibold">Run</th>
            <th className="px-4 py-3 font-semibold">Worker</th>
            <th className="px-4 py-3 font-semibold">Skill</th>
            <th className="px-4 py-3 font-semibold">SLA</th>
            <th className="px-4 py-3 font-semibold">Retry</th>
            <th className="px-4 py-3 text-right font-semibold">Actions</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-[var(--border)] bg-[var(--background)]">
          {runs.map((run) => (
            <tr key={run.id} data-testid="orchestration-row">
              <td className="px-4 py-3">
                <div className="font-medium text-[var(--foreground)]">{run.id}</div>
                <div className="text-xs text-[var(--muted-foreground)]">{run.status}</div>
              </td>
              <td className="px-4 py-3 text-[var(--foreground)]">{run.worker}</td>
              <td className="px-4 py-3 text-[var(--foreground)]">{run.skill}</td>
              <td className="px-4 py-3">
                <SlaPill run={run} />
              </td>
              <td className="px-4 py-3">
                <RetryBadge
                  attempt={run.retryAttempt}
                  max={run.retryMax}
                  backoff={run.retryBackoff}
                />
              </td>
              <td className="px-4 py-3">
                <div className="flex justify-end gap-1">
                  <IconButton label="Cancel run" onClick={() => onCancel(run.id)} icon={CircleStop} />
                  <IconButton label="Reset retry" onClick={() => onResetRetry(run.id)} icon={RotateCcw} />
                  <IconButton label="Force escalate" onClick={() => onForceEscalate(run.id)} icon={Send} />
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SlaPill({ run }: { run: OrchestrationRun }) {
  const pct = Math.round((run.latencyMs / run.targetLatencyMs) * 100);
  return (
    <span
      className={cn(
        "inline-flex h-7 items-center gap-1 rounded-[var(--radius-sm)] border px-2 text-xs",
        run.slaState === "breached"
          ? "border-red-300 bg-red-50 text-red-700"
          : run.slaState === "warning"
            ? "border-amber-300 bg-amber-50 text-amber-800"
            : "border-emerald-300 bg-emerald-50 text-emerald-700",
      )}
      title={`${run.latencyMs} ms of ${run.targetLatencyMs} ms`}
    >
      {run.slaState === "breached" ? <AlertTriangle className="size-3.5" aria-hidden /> : null}
      <span>{pct}%</span>
    </span>
  );
}

function IconButton({
  label,
  onClick,
  icon: Icon,
}: {
  label: string;
  onClick: () => void;
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="inline-flex size-8 items-center justify-center rounded-[var(--radius-sm)] border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)]"
    >
      <Icon className="size-4" aria-hidden />
    </button>
  );
}
