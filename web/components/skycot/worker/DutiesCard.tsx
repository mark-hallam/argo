"use client";

import { useEffect, useState } from "react";
import { Plus } from "lucide-react";
import { Card, CardBody, CardFooter, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { WorkerDuty } from "@/lib/api/worker-overview";
import { AddResponsibilityDialog } from "@/components/skycot/worker/AddResponsibilityDialog";
import type { CommitDutyResult } from "@/lib/api/duties";

/**
 * Duty list card. Each row has a toggle (enable/disable), a label,
 * a schedule chip, and a click target that opens the inline editor.
 *
 * "+ Add responsibility" mounts the AddResponsibilityDialog (E4-h)
 * when `workerId` + `workerName` are supplied; otherwise it still
 * fires the `onAddDuty` analytics seam so existing consumers
 * (WorkerOverviewContent E4-b) continue to receive the click signal.
 * On a successful commit the card surfaces a Step-6 confirmation
 * toast inline.
 */

export interface DutiesCardProps {
  duties: WorkerDuty[];
  canEdit: boolean;
  /** Worker id — required to mount the AddResponsibilityDialog. */
  workerId?: string;
  /** Worker first name — surfaces in dialog copy. */
  workerName?: string;
  /** Called when a duty is toggled. */
  onToggle?: (dutyId: string, enabled: boolean) => void;
  /** Click into a duty body — v1 stubs to a toast via this callback. */
  onEditDuty?: (dutyId: string) => void;
  /** Analytics seam — fires alongside the dialog open. */
  onAddDuty?: () => void;
  /** Per-card error state. */
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

export function DutiesCard({
  duties,
  canEdit,
  workerId,
  workerName,
  onToggle,
  onEditDuty,
  onAddDuty,
  error,
  onRetry,
  className,
}: DutiesCardProps) {
  const dialogAvailable = canEdit && Boolean(workerId && workerName);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<CommitDutyResult | null>(null);

  useEffect(() => {
    if (!confirmation) return;
    const t = setTimeout(() => setConfirmation(null), 6000);
    return () => clearTimeout(t);
  }, [confirmation]);

  const handleAddClick = () => {
    onAddDuty?.();
    if (dialogAvailable) setDialogOpen(true);
  };

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
          onClick={handleAddClick}
          disabled={!canEdit}
          data-testid="duties-add"
        >
          <Plus className="size-3.5" />
          Add responsibility
        </Button>
      </CardFooter>

      {dialogAvailable ? (
        <AddResponsibilityDialog
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          workerId={workerId!}
          workerName={workerName!}
          onCommitted={(result) => {
            setConfirmation(result);
            setDialogOpen(false);
          }}
        />
      ) : null}

      {confirmation ? (
        <DutyAddedToast
          workerName={workerName ?? "She"}
          result={confirmation}
          onClose={() => setConfirmation(null)}
        />
      ) : null}
    </Card>
  );
}

function DutyAddedToast({
  workerName,
  result,
  onClose,
}: {
  workerName: string;
  result: CommitDutyResult;
  onClose: () => void;
}) {
  const nextFire = result.nextFireAt
    ? new Date(result.nextFireAt).toLocaleString(undefined, {
        weekday: "short",
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      })
    : null;
  return (
    <div
      role="status"
      aria-live="polite"
      data-testid="duty-added-toast"
      className="fixed bottom-6 right-6 z-40 max-w-sm rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-3 text-sm text-[var(--card-foreground)] shadow-lg"
    >
      <p className="font-medium text-[var(--foreground)]">
        Added the new responsibility.
      </p>
      <p className="mt-1 text-xs text-[var(--muted-foreground)]">
        {result.duty.label}.
        {nextFire ? ` ${workerName}'s next run kicks off ${nextFire}.` : ""}
      </p>
      <button
        type="button"
        onClick={onClose}
        data-testid="duty-added-toast-dismiss"
        className="mt-2 text-xs text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Dismiss
      </button>
    </div>
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
