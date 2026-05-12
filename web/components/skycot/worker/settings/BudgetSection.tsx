"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import {
  budgetPercent,
  budgetTone,
  type BudgetConfig,
} from "@/lib/api/worker-settings";
import { cn } from "@/lib/utils";

/**
 * Budget section — daily + monthly cap editors and live usage
 * bars. v1 caps are per-worker only; the Track H brief
 * `mission-enterprise-budget-hierarchical-caps` promotes this to
 * tenant → department → team → worker, with the tighter ceiling
 * winning.
 *
 * Bar color: muted < 50%, default 50-80%, warning 80-95%,
 * destructive ≥ 95%. At ≥ 100% the worker hard-pauses until the
 * cap is raised or the next day rolls over.
 */

const TONE_BG: Record<ReturnType<typeof budgetTone>, string> = {
  muted: "bg-[var(--muted)]",
  default: "bg-[var(--primary)]",
  warning: "bg-amber-500",
  destructive: "bg-[var(--destructive)]",
};

export interface BudgetSectionProps {
  value: BudgetConfig;
  onSave: (next: Pick<BudgetConfig, "dailyCapUsd" | "monthlyCapUsd">) => void;
}

export function BudgetSection({ value, onSave }: BudgetSectionProps) {
  const [daily, setDaily] = useState(String(value.dailyCapUsd));
  const [monthly, setMonthly] = useState(String(value.monthlyCapUsd));

  useEffect(() => {
    setDaily(String(value.dailyCapUsd));
    setMonthly(String(value.monthlyCapUsd));
  }, [value.dailyCapUsd, value.monthlyCapUsd]);

  const dailyNum = Number.parseFloat(daily);
  const monthlyNum = Number.parseFloat(monthly);
  const validDaily = Number.isFinite(dailyNum) && dailyNum > 0;
  const validMonthly = Number.isFinite(monthlyNum) && monthlyNum > 0;
  const valid = validDaily && validMonthly;
  const isDirty =
    valid &&
    (dailyNum !== value.dailyCapUsd || monthlyNum !== value.monthlyCapUsd);

  const dailyPct = budgetPercent(value.usedTodayUsd, value.dailyCapUsd);
  const monthlyPct = budgetPercent(value.usedMonthUsd, value.monthlyCapUsd);
  const dailyTone = budgetTone(dailyPct);
  const monthlyTone = budgetTone(monthlyPct);
  const dailyPaused = dailyPct >= 100;

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

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-2">
          <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
            Daily cap (USD)
            <input
              type="number"
              min={0}
              step={1}
              value={daily}
              onChange={(e) => setDaily(e.target.value)}
              data-testid="budget-daily-input"
              aria-invalid={!validDaily}
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-sm text-[var(--foreground)]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
            Monthly cap (USD)
            <input
              type="number"
              min={0}
              step={1}
              value={monthly}
              onChange={(e) => setMonthly(e.target.value)}
              data-testid="budget-monthly-input"
              aria-invalid={!validMonthly}
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-sm text-[var(--foreground)]"
            />
          </label>
        </div>
        <div className="flex justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!isDirty}
            onClick={() => {
              setDaily(String(value.dailyCapUsd));
              setMonthly(String(value.monthlyCapUsd));
            }}
            data-testid="budget-cancel"
          >
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={!isDirty}
            onClick={() => {
              if (!valid) return;
              onSave({ dailyCapUsd: dailyNum, monthlyCapUsd: monthlyNum });
            }}
            data-testid="budget-save"
          >
            Save
          </Button>
        </div>
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
