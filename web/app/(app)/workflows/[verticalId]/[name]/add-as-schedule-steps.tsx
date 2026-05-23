"use client";

import type { Worker } from "@/lib/types/worker";
import { cn } from "@/lib/utils";
import {
  CADENCE_PRESETS,
  CUSTOM_CADENCE_ID,
  type DrawerStep,
} from "./add-as-schedule-helpers";

export function StepIndicator({ step }: { step: DrawerStep }) {
  return (
    <ol
      aria-label="Steps"
      data-testid="add-as-schedule-stepper"
      className="flex items-center gap-2 border-b border-[var(--border)] px-6 py-3 text-xs uppercase tracking-wider text-[var(--muted-foreground)]"
    >
      {[1, 2, 3].map((n) => (
        <li key={n} className="flex items-center gap-2">
          <span
            data-active={n === step ? "true" : "false"}
            className={cn(
              "grid size-5 place-items-center rounded-full border text-[10px]",
              n === step
                ? "border-[var(--foreground)] bg-[var(--foreground)] text-[var(--background)]"
                : n < step
                  ? "border-[var(--foreground)] text-[var(--foreground)]"
                  : "border-[var(--border)]",
            )}
          >
            {n}
          </span>
          <span>{n === 1 ? "Worker" : n === 2 ? "Cadence" : "Confirm"}</span>
          {n < 3 ? <span aria-hidden>·</span> : null}
        </li>
      ))}
    </ol>
  );
}

export function WorkerStep({
  workers,
  workerId,
  onSelect,
  isLoading,
  verticalId,
}: {
  workers: Worker[];
  workerId: string;
  onSelect: (id: string) => void;
  isLoading: boolean;
  verticalId: string;
}) {
  if (isLoading) {
    return (
      <p className="text-sm text-[var(--muted-foreground)]">Loading workers…</p>
    );
  }
  if (workers.length === 0) {
    return (
      <p
        data-testid="add-as-schedule-no-workers"
        className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] px-4 py-6 text-center text-sm text-[var(--muted-foreground)]"
      >
        No workers in the <span className="font-medium">{verticalId}</span>{" "}
        vertical yet. Hire one first.
      </p>
    );
  }
  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="sr-only">Choose a worker</legend>
      {workers.map((w) => {
        const checked = w.id === workerId;
        return (
          <label
            key={w.id}
            data-testid={`add-as-schedule-worker-${w.id}`}
            data-checked={checked ? "true" : "false"}
            className={cn(
              "flex items-center justify-between gap-3 rounded-[var(--radius-md)] border px-4 py-3 text-sm cursor-pointer",
              checked
                ? "border-[var(--foreground)] bg-[var(--muted)]/50"
                : "border-[var(--border)] hover:bg-[var(--muted)]/30",
            )}
          >
            <span className="flex items-center gap-3 min-w-0">
              <input
                type="radio"
                name="add-as-schedule-worker"
                value={w.id}
                checked={checked}
                onChange={() => onSelect(w.id)}
                className="size-4"
              />
              <span className="flex flex-col min-w-0">
                <span className="font-medium text-[var(--foreground)] truncate">
                  {w.name}
                </span>
                <span className="text-xs text-[var(--muted-foreground)] truncate">
                  {w.role.tagline}
                </span>
              </span>
            </span>
            <span className="flex items-center gap-2 text-xs text-[var(--muted-foreground)] shrink-0">
              {w.team?.name ? <span>{w.team.name}</span> : null}
              <span
                className={cn(
                  "rounded-full px-2 py-0.5 text-[10px] uppercase tracking-wide",
                  w.status === "working"
                    ? "bg-emerald-100 text-emerald-700"
                    : w.status === "waiting"
                      ? "bg-amber-100 text-amber-700"
                      : "bg-zinc-100 text-zinc-600",
                )}
              >
                {w.status}
              </span>
            </span>
          </label>
        );
      })}
    </fieldset>
  );
}

export function CadenceStep({
  cadenceId,
  onSelect,
  customCron,
  onCustomCronChange,
  showCustom,
  onToggleCustom,
  cadenceValid,
}: {
  cadenceId: string;
  onSelect: (id: string) => void;
  customCron: string;
  onCustomCronChange: (s: string) => void;
  showCustom: boolean;
  onToggleCustom: () => void;
  cadenceValid: boolean;
}) {
  return (
    <div className="flex flex-col gap-3">
      <fieldset className="flex flex-col gap-2">
        <legend className="sr-only">Choose a cadence</legend>
        {CADENCE_PRESETS.map((c) => {
          const checked = cadenceId === c.id;
          return (
            <label
              key={c.id}
              data-testid={`add-as-schedule-cadence-${c.id}`}
              data-checked={checked ? "true" : "false"}
              className={cn(
                "flex items-start justify-between gap-3 rounded-[var(--radius-md)] border px-4 py-3 text-sm cursor-pointer",
                checked
                  ? "border-[var(--foreground)] bg-[var(--muted)]/50"
                  : "border-[var(--border)] hover:bg-[var(--muted)]/30",
              )}
            >
              <span className="flex items-start gap-3 min-w-0">
                <input
                  type="radio"
                  name="add-as-schedule-cadence"
                  value={c.id}
                  checked={checked}
                  onChange={() => onSelect(c.id)}
                  className="mt-0.5 size-4"
                />
                <span className="flex flex-col">
                  <span className="font-medium text-[var(--foreground)]">
                    {c.label}
                  </span>
                  <span className="text-xs text-[var(--muted-foreground)]">
                    {c.detail}
                  </span>
                </span>
              </span>
              <code className="text-[10px] text-[var(--muted-foreground)] shrink-0">
                {c.cron}
              </code>
            </label>
          );
        })}
      </fieldset>
      <div className="pt-1">
        <button
          type="button"
          onClick={onToggleCustom}
          data-testid="add-as-schedule-toggle-custom"
          className="text-xs underline-offset-2 hover:underline text-[var(--muted-foreground)]"
        >
          {showCustom ? "Hide advanced (custom cron)" : "Advanced: custom cron"}
        </button>
        {showCustom ? (
          <label className="mt-2 flex flex-col gap-1 text-sm">
            <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
              Custom 5-field cron (minute hour dom month dow)
            </span>
            <input
              type="text"
              data-testid="add-as-schedule-custom-cron"
              value={customCron}
              onChange={(e) => {
                onCustomCronChange(e.target.value);
                onSelect(CUSTOM_CADENCE_ID);
              }}
              placeholder="*/30 9-17 * * 1-5"
              className={cn(
                "rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]",
                "px-3 py-2 font-mono text-xs",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              )}
            />
            {cadenceId === CUSTOM_CADENCE_ID && !cadenceValid ? (
              <span
                data-testid="add-as-schedule-cron-error"
                className="text-xs text-red-600"
              >
                Cron must be 5 space-separated fields.
              </span>
            ) : null}
          </label>
        ) : null}
      </div>
    </div>
  );
}

export function ConfirmStep({
  workflowName,
  workerName,
  cadenceLabel,
  cadenceSpec,
}: {
  workflowName: string;
  workerName: string;
  cadenceLabel: string;
  cadenceSpec: string;
}) {
  return (
    <dl
      data-testid="add-as-schedule-confirm"
      className="grid grid-cols-[120px_1fr] gap-x-4 gap-y-3 text-sm"
    >
      <dt className="text-xs uppercase tracking-wide text-[var(--muted-foreground)]">
        Workflow
      </dt>
      <dd className="font-medium text-[var(--foreground)]">{workflowName}</dd>
      <dt className="text-xs uppercase tracking-wide text-[var(--muted-foreground)]">
        Worker
      </dt>
      <dd className="font-medium text-[var(--foreground)]">{workerName}</dd>
      <dt className="text-xs uppercase tracking-wide text-[var(--muted-foreground)]">
        Cadence
      </dt>
      <dd>
        <div className="font-medium text-[var(--foreground)]">{cadenceLabel}</div>
        <code className="text-[10px] text-[var(--muted-foreground)]">
          {cadenceSpec}
        </code>
      </dd>
    </dl>
  );
}
