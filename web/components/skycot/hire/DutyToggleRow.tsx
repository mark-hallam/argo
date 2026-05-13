"use client";

import { useId } from "react";
import { Clock, Zap } from "lucide-react";
import type { WorkerDuty } from "@/lib/hire/draft-types";
import { cn } from "@/lib/utils";

/**
 * One duty row inside the Step 4 worker card.
 *
 * Layout (left-to-right):
 *  - Checkbox toggling `defaultEnabled` in the parent draft.
 *  - Label (clicking the label also toggles the checkbox via `htmlFor`).
 *  - Schedule chip — clock icon + cron expression for `cron`, lightning
 *    + event name for `event`.
 *
 * v1 keeps the row read-only besides the toggle. Editing the duty
 * label / schedule is deferred to v2 (`mission-enterprise-hire-duty-
 * editor`); the user already has the Adjust drawer for free-text
 * redirection.
 */
export interface DutyToggleRowProps {
  duty: WorkerDuty;
  /** 0-based index inside the worker draft's `duties` array. */
  index: number;
  onToggle: (index: number, nextEnabled: boolean) => void;
}

function describeSchedule(schedule: WorkerDuty["schedule"]): string {
  if (schedule.kind === "cron") return schedule.spec;
  return schedule.spec;
}

export function DutyToggleRow({ duty, index, onToggle }: DutyToggleRowProps) {
  const id = useId();
  const Icon = duty.schedule.kind === "cron" ? Clock : Zap;
  return (
    <div
      data-testid={`hire-step4-duty-${index}`}
      className={cn(
        "flex items-start gap-3 rounded-[var(--radius-md)] border border-[var(--border)] px-3 py-2",
        duty.defaultEnabled ? "bg-[var(--card)]" : "bg-[var(--muted)]/30",
      )}
    >
      <input
        id={id}
        type="checkbox"
        data-testid={`hire-step4-duty-${index}-toggle`}
        checked={duty.defaultEnabled}
        onChange={(e) => onToggle(index, e.target.checked)}
        className="mt-1 size-4 accent-[var(--primary)]"
        aria-label={`Enable duty: ${duty.label}`}
      />
      <label
        htmlFor={id}
        className="flex-1 cursor-pointer select-none text-sm text-[var(--foreground)]"
      >
        {duty.label}
      </label>
      <span
        className="inline-flex items-center gap-1 rounded-[var(--radius-sm)] bg-[var(--accent)] px-2 py-0.5 text-xs font-mono text-[var(--accent-foreground)]"
        title={duty.schedule.kind === "cron" ? "Cron schedule" : "Event trigger"}
      >
        <Icon className="size-3" aria-hidden />
        {describeSchedule(duty.schedule)}
      </span>
    </div>
  );
}
