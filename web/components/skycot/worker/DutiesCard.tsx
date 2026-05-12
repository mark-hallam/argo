"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { Card, CardBody, CardFooter, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { WorkerDuty } from "@/lib/api/worker-overview";

/**
 * Duty list card. Each row has a toggle (enable/disable), a label,
 * a schedule chip, and a click target that opens the inline editor.
 *
 * v1 scope:
 *   - Toggle calls `onToggle(dutyId, enabled)` — parent wires the
 *     existing duty-toggle mutation when available; today's `useState`
 *     fallback keeps the toggle visually responsive without a backend.
 *   - Click-into-edit and "+ Add responsibility" stub to toasts
 *     pointing at E4-h. The dialogs themselves ship there.
 */

export interface DutiesCardProps {
  duties: WorkerDuty[];
  canEdit: boolean;
  /** Called when a duty is toggled. */
  onToggle?: (dutyId: string, enabled: boolean) => void;
  /** Click into a duty body — v1 stubs to a toast via this callback. */
  onEditDuty?: (dutyId: string) => void;
  /** "+ Add responsibility" button — v1 stubs to a toast. */
  onAddDuty?: () => void;
  /** Per-card error state. */
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

export function DutiesCard({
  duties,
  canEdit,
  onToggle,
  onEditDuty,
  onAddDuty,
  error,
  onRetry,
  className,
}: DutiesCardProps) {
  return (
    <Card data-testid="card-duties" className={className}>
      <CardHeader>
        <CardTitle>Responsibilities</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <DutiesErrorState message={error} onRetry={onRetry} />
        ) : duties.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            No responsibilities yet.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {duties.map((d) => (
              <DutyRow
                key={d.id}
                duty={d}
                canEdit={canEdit}
                onToggle={onToggle}
                onEditDuty={onEditDuty}
              />
            ))}
          </ul>
        )}
      </CardBody>
      <CardFooter>
        <Button
          variant="ghost"
          size="sm"
          onClick={onAddDuty}
          disabled={!canEdit}
          data-testid="duties-add"
        >
          <Plus className="size-3.5" />
          Add responsibility
        </Button>
      </CardFooter>
    </Card>
  );
}

function DutyRow({
  duty,
  canEdit,
  onToggle,
  onEditDuty,
}: {
  duty: WorkerDuty;
  canEdit: boolean;
  onToggle?: (dutyId: string, enabled: boolean) => void;
  onEditDuty?: (dutyId: string) => void;
}) {
  const [enabled, setEnabled] = useState(duty.enabled);

  const handleToggle = () => {
    if (!canEdit) return;
    const next = !enabled;
    setEnabled(next);
    onToggle?.(duty.id, next);
  };

  return (
    <li
      data-testid={`duty-row-${duty.id}`}
      data-enabled={enabled || undefined}
      className="flex items-center gap-3 py-3"
    >
      <DutyToggle
        dutyId={duty.id}
        enabled={enabled}
        canEdit={canEdit}
        onChange={handleToggle}
      />
      <button
        type="button"
        onClick={() => onEditDuty?.(duty.id)}
        data-testid={`duty-edit-${duty.id}`}
        className={cn(
          "flex-1 min-w-0 text-left rounded-[var(--radius-sm)] px-1 -mx-1",
          "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
          "hover:bg-[var(--accent)]/40",
        )}
      >
        <p
          className={cn(
            "text-sm font-medium",
            enabled ? "text-[var(--foreground)]" : "text-[var(--muted-foreground)] line-through",
          )}
        >
          {duty.label}
        </p>
      </button>
      <span
        data-testid={`duty-schedule-${duty.id}`}
        className="shrink-0 rounded-full bg-[var(--muted)] px-2 py-0.5 text-xs text-[var(--muted-foreground)]"
      >
        {duty.schedule}
      </span>
    </li>
  );
}

function DutyToggle({
  dutyId,
  enabled,
  canEdit,
  onChange,
}: {
  dutyId: string;
  enabled: boolean;
  canEdit: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      aria-label={enabled ? "Disable duty" : "Enable duty"}
      disabled={!canEdit}
      onClick={onChange}
      data-testid={`duty-toggle-${dutyId}`}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
        enabled ? "bg-[var(--primary)]" : "bg-[var(--muted)]",
        !canEdit && "cursor-not-allowed opacity-60",
      )}
    >
      <span
        aria-hidden
        className={cn(
          "inline-block h-4 w-4 transform rounded-full bg-[var(--background)] shadow transition-transform",
          "mt-0.5 ml-0.5",
          enabled && "translate-x-4",
        )}
      />
    </button>
  );
}

function DutiesErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}

export function DutiesCardSkeleton() {
  return (
    <Card data-testid="card-duties-skeleton">
      <CardHeader>
        <div className="h-4 w-32 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="h-5 w-9 rounded-full bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-4 w-20 rounded-full bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
