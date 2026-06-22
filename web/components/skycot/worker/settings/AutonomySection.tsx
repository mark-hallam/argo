"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import {
  AUTONOMY_LEVELS,
  AUTONOMY_LABELS,
  AUTONOMY_DESCRIPTIONS,
  AUTONOMY_FLOOR_ITEMS,
  type AutonomyConfig,
  type AutonomyLevel,
} from "@/lib/api/worker-settings";
import { autonomyFormSchema } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Autonomy section — three-option radio. Switching from Balanced or
 * Aggressive down to Conservative triggers a confirm dialog that
 * surfaces the trust-ratchet reset (the worker re-asks for any
 * action she had been doing autonomously).
 */

const RATCHET_RESET_LEVEL: AutonomyLevel = "conservative";

export interface AutonomySectionProps {
  value: AutonomyConfig;
  workerName: string;
  onSave: (next: AutonomyConfig) => Promise<void>;
  isSaving?: boolean;
}

export function AutonomySection({
  value,
  workerName,
  onSave,
  isSaving = false,
}: AutonomySectionProps) {
  const toast = useToast();
  const form = useZodForm(autonomyFormSchema, {
    defaultValues: { level: value.level },
  });
  const { register, handleSubmit, reset, formState, watch } = form;
  const [pendingConfirm, setPendingConfirm] = useState<AutonomyLevel | null>(null);

  useEffect(() => {
    reset({ level: value.level });
  }, [value.level, reset]);

  const level = watch("level");

  const commit = async (next: AutonomyLevel) => {
    try {
      await onSave({ level: next });
      toast.success("Autonomy level saved");
      reset({ level: next });
    } catch (err) {
      toast.error("Couldn't save autonomy level", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  };

  const submit = handleSubmit(async (values) => {
    if (
      values.level === RATCHET_RESET_LEVEL &&
      value.level !== RATCHET_RESET_LEVEL
    ) {
      setPendingConfirm(values.level);
      return;
    }
    await commit(values.level);
  });

  const submitDisabled = !formState.isDirty || formState.isSubmitting || isSaving;

  return (
    <Card data-testid="card-settings-autonomy">
      <CardHeader>
        <CardTitle>Autonomy level</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          How often {workerName} pauses to ask for approval before acting.
        </p>
      </CardHeader>
      <CardBody className="pt-1">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
          noValidate
        >
          <ul role="radiogroup" aria-label="Autonomy level" className="space-y-2">
            {AUTONOMY_LEVELS.map((opt) => (
              <li key={opt}>
                <label
                  data-testid={`autonomy-option-${opt}`}
                  title={
                    opt === RATCHET_RESET_LEVEL
                      ? `Accept Safe resets the trust ratchet — ${workerName} asks again for any action she's been doing autonomously.`
                      : undefined
                  }
                  className={cn(
                    "flex flex-col gap-1 rounded-[var(--radius-md)] border px-3 py-2.5 cursor-pointer",
                    level === opt
                      ? "border-[var(--primary)]/60 bg-[var(--accent)]/20"
                      : "border-[var(--border)]",
                  )}
                >
                  <span className="flex items-center gap-3">
                    <input
                      type="radio"
                      value={opt}
                      data-testid={`autonomy-radio-${opt}`}
                      className="size-3.5 accent-[var(--primary)]"
                      {...register("level")}
                    />
                    <span className="text-sm font-medium text-[var(--foreground)]">
                      {AUTONOMY_LABELS[opt]}
                    </span>
                  </span>
                  <span className="pl-[1.625rem] text-xs leading-snug text-[var(--muted-foreground)]">
                    {AUTONOMY_DESCRIPTIONS[opt]}
                  </span>
                </label>
              </li>
            ))}
          </ul>
          <div
            data-testid="autonomy-floor"
            className="mt-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/40 px-3 py-2.5"
          >
            <p className="text-xs font-semibold text-[var(--foreground)]">
              These always wait for you — even on Bypass
            </p>
            <ul className="mt-1.5 flex flex-wrap gap-1.5">
              {AUTONOMY_FLOOR_ITEMS.map((item) => (
                <li
                  key={item}
                  className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-2 py-0.5 text-[11px] text-[var(--muted-foreground)]"
                >
                  {item}
                </li>
              ))}
            </ul>
          </div>
          {formState.errors.level ? (
            <p
              role="alert"
              data-testid="autonomy-error-level"
              className="mt-2 text-xs text-[var(--destructive)]"
            >
              {formState.errors.level.message}
            </p>
          ) : null}
          <div className="mt-4 flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={!formState.isDirty || formState.isSubmitting || isSaving}
              onClick={() => reset({ level: value.level })}
              data-testid="autonomy-cancel"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={submitDisabled}
              loading={formState.isSubmitting || isSaving}
              data-testid="autonomy-save"
            >
              Save
            </Button>
          </div>
        </form>
      </CardBody>
      <AlertDialog
        open={pendingConfirm !== null}
        onOpenChange={(open) => {
          if (!open) setPendingConfirm(null);
        }}
        title="Reset trust ratchet?"
        description={`Switching to Accept Safe resets the trust ratchet — ${workerName} asks again for any action she's been doing autonomously.`}
        confirmLabel="Reset & save"
        cancelLabel="Cancel"
        confirmVariant="destructive"
        onConfirm={() => {
          if (pendingConfirm) {
            void commit(pendingConfirm);
          }
          setPendingConfirm(null);
        }}
        onCancel={() => setPendingConfirm(null)}
      />
    </Card>
  );
}
