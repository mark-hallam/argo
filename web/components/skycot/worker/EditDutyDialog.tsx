"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import { isValidCron } from "@/components/skycot/worker/duty/Step1Capture";
import { useUpdateDuty } from "@/lib/hooks/use-duties";
import type { WorkerDuty } from "@/lib/api/worker-overview";

/**
 * Modal editor for an existing duty (label / 5-field cron / enabled).
 *
 * Wraps `useUpdateDuty` so the gateway-returned (or mock-fallback)
 * `WorkerDuty` lands back in the overview cache without a refetch.
 * Client-side validation matches the gateway's `MIN_LABEL_LEN=4` /
 * `MAX_LABEL_LEN=100` / 5-field cron contract so a Save click that
 * gets a 400 means we missed a bound — not a routine failure mode.
 */

export interface EditDutyDialogProps {
  open: boolean;
  workerId: string;
  duty: WorkerDuty | null;
  onOpenChange: (open: boolean) => void;
  onSaved?: (duty: WorkerDuty) => void;
  onError?: (err: Error) => void;
}

const LABEL_MIN = 4;
const LABEL_MAX = 100;
const DEFAULT_CRON = "0 9 * * *";

/**
 * Pull a 5-field cron expression out of the WorkerDuty.schedule
 * string. The SPA-side shape uses `"cron: <spec>"`; the gateway PATCH
 * response normalises into the same shape. If we can't extract a
 * cron, fall back to the daily-9am default so the field is always
 * editable rather than blank.
 */
function extractCron(schedule: string): string {
  const m = schedule.match(/^cron:\s*(.+)$/i);
  if (m && isValidCron(m[1].trim())) return m[1].trim();
  if (isValidCron(schedule.trim())) return schedule.trim();
  return DEFAULT_CRON;
}

interface FormState {
  label: string;
  cron: string;
  enabled: boolean;
}

function initialForm(duty: WorkerDuty | null): FormState {
  return {
    label: duty?.label ?? "",
    cron: duty ? extractCron(duty.schedule) : DEFAULT_CRON,
    enabled: duty?.enabled ?? true,
  };
}

export function EditDutyDialog({
  open,
  workerId,
  duty,
  onOpenChange,
  onSaved,
  onError,
}: EditDutyDialogProps) {
  const updateMutation = useUpdateDuty(workerId);
  const [form, setForm] = useState<FormState>(() => initialForm(duty));
  const [submitError, setSubmitError] = useState<string | null>(null);

  // Reset state when the dialog opens for a new duty.
  useEffect(() => {
    if (open) {
      setForm(initialForm(duty));
      setSubmitError(null);
    }
  }, [open, duty]);

  const trimmedLabel = form.label.trim();
  const labelValid =
    trimmedLabel.length >= LABEL_MIN && trimmedLabel.length <= LABEL_MAX;
  const cronValid = isValidCron(form.cron);
  const formValid = labelValid && cronValid;

  const dirty = useMemo(() => {
    if (!duty) return false;
    const baseline = initialForm(duty);
    return (
      form.label !== baseline.label ||
      form.cron !== baseline.cron ||
      form.enabled !== baseline.enabled
    );
  }, [duty, form]);

  const saveDisabled = !duty || !formValid || !dirty || updateMutation.isPending;

  const close = () => onOpenChange(false);

  const handleSave = async () => {
    if (!duty || saveDisabled) return;
    setSubmitError(null);
    const baseline = initialForm(duty);
    const patch: { label?: string; cron?: string; enabled?: boolean } = {};
    if (form.label !== baseline.label) patch.label = trimmedLabel;
    if (form.cron !== baseline.cron) patch.cron = form.cron.trim();
    if (form.enabled !== baseline.enabled) patch.enabled = form.enabled;

    try {
      const result = await updateMutation.mutateAsync({
        dutyId: duty.id,
        ...patch,
      });
      onSaved?.(result.duty);
      onOpenChange(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setSubmitError(message);
      if (err instanceof Error) onError?.(err);
    }
  };

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
          data-testid="edit-duty-dialog"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none max-h-[85vh] overflow-y-auto",
          )}
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            Edit schedule
          </DialogPrimitive.Title>
          <DialogPrimitive.Description
            data-testid="edit-duty-description"
            className="mt-1 text-sm text-[var(--muted-foreground)]"
          >
            Adjust the label, cadence, or whether this schedule is active.
            Changes save to this worker straight away.
          </DialogPrimitive.Description>

          <div className="mt-5 space-y-4">
            <label className="block text-sm">
              <span className="font-medium text-[var(--foreground)]">Label</span>
              <input
                type="text"
                value={form.label}
                onChange={(e) =>
                  setForm((s) => ({ ...s, label: e.target.value }))
                }
                data-testid="edit-duty-label"
                aria-label="Schedule label"
                maxLength={LABEL_MAX + 10}
                className={cn(
                  "mt-1 block w-full rounded-[var(--radius-md)] border bg-[var(--background)] px-3 py-2 text-sm",
                  "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  labelValid || trimmedLabel.length === 0
                    ? "border-[var(--border)]"
                    : "border-[var(--destructive)]/70",
                )}
              />
              {!labelValid && trimmedLabel.length > 0 ? (
                <p
                  data-testid="edit-duty-label-error"
                  className="mt-1 text-xs text-[var(--destructive)]"
                >
                  Label must be {LABEL_MIN}–{LABEL_MAX} characters.
                </p>
              ) : null}
            </label>

            <label className="block text-sm">
              <span className="font-medium text-[var(--foreground)]">
                Schedule (5-field cron)
              </span>
              <input
                type="text"
                value={form.cron}
                onChange={(e) =>
                  setForm((s) => ({ ...s, cron: e.target.value }))
                }
                data-testid="edit-duty-cron"
                aria-label="Cron expression"
                spellCheck={false}
                className={cn(
                  "mt-1 block w-full rounded-[var(--radius-md)] border bg-[var(--background)] px-3 py-2 font-mono text-sm",
                  "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  cronValid
                    ? "border-[var(--border)]"
                    : "border-[var(--destructive)]/70",
                )}
              />
              {!cronValid ? (
                <p
                  data-testid="edit-duty-cron-error"
                  className="mt-1 text-xs text-[var(--destructive)]"
                >
                  Use 5 space-separated cron fields, e.g. <code>0 9 * * 1-5</code>.
                </p>
              ) : (
                <p className="mt-1 text-xs text-[var(--muted-foreground)]">
                  Format: minute hour day-of-month month day-of-week.
                </p>
              )}
            </label>

            <label className="flex items-center justify-between gap-3 rounded-[var(--radius-md)] border border-[var(--border)] px-3 py-2 text-sm">
              <span className="font-medium text-[var(--foreground)]">
                Active
                <span className="block text-xs font-normal text-[var(--muted-foreground)]">
                  Disabled schedules stay on the worker but stop firing.
                </span>
              </span>
              <button
                type="button"
                role="switch"
                aria-checked={form.enabled}
                onClick={() =>
                  setForm((s) => ({ ...s, enabled: !s.enabled }))
                }
                data-testid="edit-duty-enabled"
                className={cn(
                  "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors",
                  "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  form.enabled ? "bg-[var(--primary)]" : "bg-[var(--muted)]",
                )}
              >
                <span
                  aria-hidden
                  className={cn(
                    "inline-block h-4 w-4 transform rounded-full bg-[var(--background)] shadow transition-transform",
                    "mt-0.5 ml-0.5",
                    form.enabled && "translate-x-4",
                  )}
                />
              </button>
            </label>

            {submitError ? (
              <p
                data-testid="edit-duty-error"
                className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-3 py-2 text-sm text-[var(--destructive-foreground)]"
              >
                Couldn&apos;t save: {submitError}
              </p>
            ) : null}
          </div>

          <div className="mt-6 flex justify-end gap-2">
            <Button
              variant="ghost"
              data-testid="edit-duty-cancel"
              onClick={close}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              data-testid="edit-duty-save"
              onClick={handleSave}
              loading={updateMutation.isPending}
              disabled={saveDisabled}
            >
              Save changes
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
