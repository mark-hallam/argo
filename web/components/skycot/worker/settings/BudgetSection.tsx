"use client";

import { useEffect } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import {
  budgetPercent,
  budgetTone,
  type BudgetConfig,
} from "@/lib/api/worker-settings";
import { budgetFormSchema } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Budget section — daily + monthly cap editors and live usage
 * bars. v1 caps are per-worker only; the Track H brief
 * `mission-enterprise-budget-hierarchical-caps` promotes this to
 * tenant → department → team → worker, with the tighter ceiling
 * winning.
 */

const TONE_BG: Record<ReturnType<typeof budgetTone>, string> = {
  muted: "bg-[var(--muted)]",
  default: "bg-[var(--primary)]",
  warning: "bg-amber-500",
  destructive: "bg-[var(--destructive)]",
};

export interface BudgetSectionProps {
  value: BudgetConfig;
  onSave: (next: Pick<BudgetConfig, "dailyCapUsd" | "monthlyCapUsd">) => Promise<void>;
  isSaving?: boolean;
}

export function BudgetSection({ value, onSave, isSaving = false }: BudgetSectionProps) {
  const toast = useToast();
  const form = useZodForm(budgetFormSchema, {
    defaultValues: {
      dailyCapUsd: String(value.dailyCapUsd),
      monthlyCapUsd: String(value.monthlyCapUsd),
    },
  });
  const { register, handleSubmit, reset, formState } = form;

  useEffect(() => {
    reset({
      dailyCapUsd: String(value.dailyCapUsd),
      monthlyCapUsd: String(value.monthlyCapUsd),
    });
  }, [value.dailyCapUsd, value.monthlyCapUsd, reset]);

  const dailyPct = budgetPercent(value.usedTodayUsd, value.dailyCapUsd);
  const monthlyPct = budgetPercent(value.usedMonthUsd, value.monthlyCapUsd);
  const dailyTone = budgetTone(dailyPct);
  const monthlyTone = budgetTone(monthlyPct);
  const dailyPaused = dailyPct >= 100;

  const submit = handleSubmit(async (values) => {
    try {
      await onSave({
        dailyCapUsd: values.dailyCapUsd,
        monthlyCapUsd: values.monthlyCapUsd,
      });
      toast.success("Budget saved");
      reset({
        dailyCapUsd: String(values.dailyCapUsd),
        monthlyCapUsd: String(values.monthlyCapUsd),
      });
    } catch (err) {
      toast.error("Couldn't save budget", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

  const submitDisabled = !formState.isDirty || formState.isSubmitting || isSaving;

  return (
    <Card data-testid="card-settings-budget">
      <CardHeader>
        <CardTitle>Budget</CardTitle>
        <p
          data-testid="budget-summary"
          className="text-sm text-[var(--muted-foreground)]"
        >
          Daily cap: ${value.dailyCapUsd.toFixed(2)}. ${value.usedTodayUsd.toFixed(2)} used
          today ({Math.round(dailyPct)}%). Monthly cap: ${value.monthlyCapUsd.toFixed(2)}.
          ${value.usedMonthUsd.toFixed(2)} used this month.
        </p>
      </CardHeader>
      <CardBody className="pt-1 space-y-4">
        <UsageBar
          label="Today"
          testid="budget-usage-daily"
          percent={dailyPct}
          tone={dailyTone}
          used={value.usedTodayUsd}
          cap={value.dailyCapUsd}
        />
        {dailyPaused ? (
          <p
            data-testid="budget-paused-notice"
            className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-3 py-2 text-xs text-[var(--destructive)]"
          >
            Daily cap reached. Worker is paused until the cap is raised or the next day
            rolls over.
          </p>
        ) : null}
        <UsageBar
          label="This month"
          testid="budget-usage-monthly"
          percent={monthlyPct}
          tone={monthlyTone}
          used={value.usedMonthUsd}
          cap={value.monthlyCapUsd}
        />

        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
          noValidate
        >
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-2">
            <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
              Daily cap (USD)
              <input
                type="number"
                min={0}
                step={1}
                data-testid="budget-daily-input"
                aria-invalid={Boolean(formState.errors.dailyCapUsd)}
                className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-2 text-sm text-[var(--foreground)]"
                {...register("dailyCapUsd")}
              />
              {formState.errors.dailyCapUsd ? (
                <span
                  role="alert"
                  data-testid="budget-error-daily"
                  className="text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.dailyCapUsd.message}
                </span>
              ) : null}
            </label>
            <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
              Monthly cap (USD)
              <input
                type="number"
                min={0}
                step={1}
                data-testid="budget-monthly-input"
                aria-invalid={Boolean(formState.errors.monthlyCapUsd)}
                className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-2 text-sm text-[var(--foreground)]"
                {...register("monthlyCapUsd")}
              />
              {formState.errors.monthlyCapUsd ? (
                <span
                  role="alert"
                  data-testid="budget-error-monthly"
                  className="text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.monthlyCapUsd.message}
                </span>
              ) : null}
            </label>
          </div>
          <div className="mt-4 flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={!formState.isDirty || formState.isSubmitting || isSaving}
              onClick={() =>
                reset({
                  dailyCapUsd: String(value.dailyCapUsd),
                  monthlyCapUsd: String(value.monthlyCapUsd),
                })
              }
              data-testid="budget-cancel"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={submitDisabled}
              loading={formState.isSubmitting || isSaving}
              data-testid="budget-save"
            >
              Save
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  );
}

interface UsageBarProps {
  label: string;
  testid: string;
  percent: number;
  tone: ReturnType<typeof budgetTone>;
  used: number;
  cap: number;
}

function UsageBar({ label, testid, percent, tone, used, cap }: UsageBarProps) {
  const width = `${Math.max(0, Math.min(100, percent))}%`;
  return (
    <div data-testid={testid} data-tone={tone}>
      <div className="flex items-baseline justify-between text-xs">
        <span className="text-[var(--muted-foreground)]">{label}</span>
        <span className="text-[var(--foreground)]">
          ${used.toFixed(2)} / ${cap.toFixed(2)}
        </span>
      </div>
      <div
        role="progressbar"
        aria-valuenow={Math.round(percent)}
        aria-valuemin={0}
        aria-valuemax={100}
        className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-[var(--muted)]/40"
      >
        <div className={cn("h-full", TONE_BG[tone])} style={{ width }} />
      </div>
    </div>
  );
}
