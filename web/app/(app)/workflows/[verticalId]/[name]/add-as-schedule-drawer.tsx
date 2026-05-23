"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { ArrowLeft, CalendarClock, Check, Loader2, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";

import { commitDuty } from "@/lib/api/duties";
import { listWorkersInVertical } from "@/lib/api/workers";
import type { Worker } from "@/lib/types/worker";
import { cn } from "@/lib/utils";

import {
  DEFAULT_CADENCE_ID,
  describeCadence,
  formatNextFire,
  resolveCadence,
  type DrawerStep,
} from "./add-as-schedule-helpers";
import {
  CadenceStep,
  ConfirmStep,
  StepIndicator,
  WorkerStep,
} from "./add-as-schedule-steps";

/**
 * 3-step drawer that turns a workflow into a recurring schedule
 * (a "duty" in storage) for a specific worker in the same vertical.
 * Step 1 picks a worker · Step 2 picks a cadence · Step 3 confirms.
 *
 * Posts via [[commitDuty]] which calls
 * `POST /api/v1/workers/<id>/duties/commit` and falls back to a mock
 * envelope in dev when the gateway 404s — matches the AddResponsibility
 * dialog's contract so this drawer ships against today's runtime
 * without a new gateway endpoint.
 */

export interface AddAsScheduleDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Workflow this schedule will run. */
  workflowName: string;
  /** Vertical to filter the worker list by. */
  verticalId: string;
  /** Test seam — overrides the worker fetch with a fixed list. */
  workersOverride?: Worker[];
}

export function AddAsScheduleDrawer({
  open,
  onOpenChange,
  workflowName,
  verticalId,
  workersOverride,
}: AddAsScheduleDrawerProps) {
  const [step, setStep] = useState<DrawerStep>(1);
  const [workerId, setWorkerId] = useState<string>("");
  const [cadenceId, setCadenceId] = useState<string>(DEFAULT_CADENCE_ID);
  const [customCron, setCustomCron] = useState<string>("");
  const [showCustom, setShowCustom] = useState<boolean>(false);
  const [submitting, setSubmitting] = useState<boolean>(false);

  const workersQuery = useQuery<Worker[]>({
    queryKey: ["workers-in-vertical", verticalId],
    queryFn: () => listWorkersInVertical(verticalId),
    enabled: open && !workersOverride && verticalId.length > 0,
    staleTime: 30_000,
  });
  const workers = workersOverride ?? workersQuery.data ?? [];

  useEffect(() => {
    if (!open) {
      setStep(1);
      setWorkerId("");
      setCadenceId(DEFAULT_CADENCE_ID);
      setCustomCron("");
      setShowCustom(false);
      setSubmitting(false);
    }
  }, [open]);

  const selectedWorker = useMemo(
    () => workers.find((w) => w.id === workerId) ?? null,
    [workers, workerId],
  );

  const cadenceSpec = useMemo(
    () => resolveCadence(cadenceId, customCron),
    [cadenceId, customCron],
  );
  const cadenceLabel = useMemo(
    () => describeCadence(cadenceId, customCron),
    [cadenceId, customCron],
  );

  const canAdvanceFromStep1 = Boolean(selectedWorker);
  const canAdvanceFromStep2 = cadenceSpec !== null;

  const handleNext = useCallback(() => {
    if (step === 1 && !canAdvanceFromStep1) return;
    if (step === 2 && !canAdvanceFromStep2) return;
    setStep((s) => (s === 3 ? s : ((s + 1) as DrawerStep)));
  }, [step, canAdvanceFromStep1, canAdvanceFromStep2]);

  const handleBack = useCallback(() => {
    setStep((s) => (s === 1 ? s : ((s - 1) as DrawerStep)));
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!selectedWorker || cadenceSpec === null) return;
    setSubmitting(true);
    try {
      const result = await commitDuty({
        workerId: selectedWorker.id,
        proposal: {
          kind: "recurring",
          label: workflowName,
          description: `Recurring schedule for the ${workflowName} workflow (${cadenceLabel}).`,
          schedule: { kind: "cron", spec: cadenceSpec },
        },
      });
      const fireText = result.nextFireAt
        ? formatNextFire(result.nextFireAt)
        : "the next matching cadence";
      toast.success(`Schedule created — first fire at ${fireText}`, {
        description: `${workflowName} → ${selectedWorker.name}`,
      });
      onOpenChange(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : "unknown error";
      toast.error(`Could not create schedule: ${message}`);
    } finally {
      setSubmitting(false);
    }
  }, [selectedWorker, cadenceSpec, cadenceLabel, workflowName, onOpenChange]);

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="add-as-schedule-drawer"
          className={cn(
            "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
            "sm:w-[92vw] sm:max-w-xl sm:-translate-x-1/2 sm:-translate-y-1/2",
            "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
            "flex flex-col sm:max-h-[85vh] focus:outline-none",
          )}
        >
          <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-6 py-5">
            <div className="flex-1 min-w-0">
              <DialogPrimitive.Title
                className="flex items-center gap-2 text-base font-semibold text-[var(--foreground)]"
                data-testid="add-as-schedule-title"
              >
                <CalendarClock aria-hidden size={18} />
                Add as Schedule
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
                Schedule <span className="font-medium">{workflowName}</span> to
                run on a recurring cadence — step {step} of 3.
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              data-testid="add-as-schedule-close"
              className={cn(
                "grid size-8 place-items-center rounded-[var(--radius-md)]",
                "text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              )}
            >
              <X aria-hidden size={18} />
            </DialogPrimitive.Close>
          </header>

          <StepIndicator step={step} />

          <div className="flex-1 overflow-y-auto px-6 py-5">
            {step === 1 ? (
              <WorkerStep
                workers={workers}
                workerId={workerId}
                onSelect={setWorkerId}
                isLoading={workersQuery.isPending && !workersOverride}
                verticalId={verticalId}
              />
            ) : null}
            {step === 2 ? (
              <CadenceStep
                cadenceId={cadenceId}
                onSelect={setCadenceId}
                customCron={customCron}
                onCustomCronChange={setCustomCron}
                showCustom={showCustom}
                onToggleCustom={() => setShowCustom((v) => !v)}
                cadenceValid={canAdvanceFromStep2}
              />
            ) : null}
            {step === 3 ? (
              <ConfirmStep
                workflowName={workflowName}
                workerName={selectedWorker?.name ?? ""}
                cadenceLabel={cadenceLabel}
                cadenceSpec={cadenceSpec ?? ""}
              />
            ) : null}
          </div>

          <footer className="flex items-center justify-between gap-3 border-t border-[var(--border)] px-6 py-4">
            <button
              type="button"
              onClick={step === 1 ? () => onOpenChange(false) : handleBack}
              disabled={submitting}
              data-testid="add-as-schedule-back"
              className={cn(
                "inline-flex items-center gap-1.5 rounded-[var(--radius-md)] border border-[var(--border)] px-3 py-2 text-sm",
                "text-[var(--foreground)] hover:bg-[var(--muted)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                "disabled:opacity-60",
              )}
            >
              <ArrowLeft aria-hidden size={14} />
              {step === 1 ? "Cancel" : "Back"}
            </button>
            {step < 3 ? (
              <button
                type="button"
                onClick={handleNext}
                disabled={
                  (step === 1 && !canAdvanceFromStep1) ||
                  (step === 2 && !canAdvanceFromStep2)
                }
                data-testid="add-as-schedule-next"
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-[var(--radius-md)] px-3 py-2 text-sm font-medium",
                  "bg-[var(--foreground)] text-[var(--background)]",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  "disabled:opacity-50",
                )}
              >
                Next
              </button>
            ) : (
              <button
                type="button"
                onClick={handleSubmit}
                disabled={submitting}
                data-testid="add-as-schedule-submit"
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-[var(--radius-md)] px-3 py-2 text-sm font-medium",
                  "bg-[var(--foreground)] text-[var(--background)]",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  "disabled:opacity-50",
                )}
              >
                {submitting ? (
                  <Loader2 aria-hidden size={14} className="animate-spin" />
                ) : (
                  <Check aria-hidden size={14} />
                )}
                {submitting ? "Creating…" : "Create schedule"}
              </button>
            )}
          </footer>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
