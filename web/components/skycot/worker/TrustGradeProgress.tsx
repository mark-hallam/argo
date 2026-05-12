"use client";

import { Check, X } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  TRUST_GRADE_ORDER,
  type TrustGradeCriterion,
  type TrustGradeId,
} from "@/lib/api/worker-trust";

/**
 * Trust-grade progress visualisation for the Trust tab. Renders:
 *
 * 1. A 5-step horizontal ladder showing the worker's current grade
 *    on the LearnedUnreviewed → Builtin scale, with the active rung
 *    highlighted and the next rung outlined.
 * 2. A criteria-met list — one row per criterion returned by the
 *    backend, each marked with a check / cross icon and showing
 *    `current_value → target_value`.
 *
 * Read-only. The brief explicitly forbids an edit affordance — there
 * is no "promote" button anywhere in this view; the criteria list
 * communicates what the operator would need to do to upgrade.
 */

export interface TrustGradeProgressProps {
  currentGrade: TrustGradeId;
  currentGradeIndex: number;
  nextGrade: TrustGradeId | null;
  nextGradeLabel: string | null;
  atMaxGrade: boolean;
  criteria: TrustGradeCriterion[];
  className?: string;
}

const GRADE_SHORT_LABEL: Record<TrustGradeId, string> = {
  learned_unreviewed: "Learned",
  published_community: "Community",
  tenant: "Tenant",
  published_verified: "Verified",
  builtin: "Builtin",
};

export function TrustGradeProgress({
  currentGrade,
  currentGradeIndex,
  nextGrade,
  nextGradeLabel,
  atMaxGrade,
  criteria,
  className,
}: TrustGradeProgressProps) {
  const totalRungs = TRUST_GRADE_ORDER.length;
  const progressPct = Math.min(100, Math.max(0, ((currentGradeIndex + 1) / totalRungs) * 100));
  const metCount = criteria.filter((c) => c.met).length;
  const totalCount = criteria.length;

  return (
    <section
      data-testid="trust-grade-progress"
      aria-label="Trust grade progress"
      className={cn("space-y-5", className)}
    >
      <div className="space-y-3">
        <div
          className="relative h-2 w-full rounded-full bg-[var(--muted)]"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={totalRungs}
          aria-valuenow={currentGradeIndex + 1}
          aria-valuetext={`${currentGradeIndex + 1} of ${totalRungs}`}
        >
          <div
            data-testid="trust-grade-progress-bar"
            className="absolute inset-y-0 left-0 rounded-full bg-[var(--primary)] transition-[width] duration-[var(--duration-base)]"
            style={{ width: `${progressPct}%` }}
          />
        </div>
        <ol className="grid grid-cols-5 gap-1 text-[11px] leading-tight">
          {TRUST_GRADE_ORDER.map((rung, idx) => {
            const isActive = rung === currentGrade;
            const isNext = rung === nextGrade;
            const isPassed = idx < currentGradeIndex;
            return (
              <li
                key={rung}
                data-testid={`trust-grade-rung-${rung}`}
                data-active={isActive || undefined}
                data-next={isNext || undefined}
                className={cn(
                  "text-center font-medium tracking-tight",
                  isActive && "text-[var(--foreground)]",
                  !isActive && isPassed && "text-[var(--muted-foreground)]",
                  !isActive && !isPassed && "text-[var(--muted-foreground)]/70",
                  isNext && !isActive && "text-[var(--primary)]",
                )}
              >
                {GRADE_SHORT_LABEL[rung]}
              </li>
            );
          })}
        </ol>
      </div>

      {atMaxGrade ? (
        <p
          data-testid="trust-grade-at-max"
          className="rounded-[var(--radius-md)] border border-[var(--success)]/30 bg-[var(--success)]/10 px-4 py-3 text-sm text-[var(--success)]"
        >
          This worker is at the top of the trust ladder. No further grade is available.
        </p>
      ) : (
        <div className="space-y-3">
          <div className="flex items-baseline justify-between">
            <h3 className="text-sm font-semibold text-[var(--foreground)]">
              Path to {nextGradeLabel ?? "next grade"}
            </h3>
            <span
              data-testid="trust-grade-met-summary"
              className="text-xs text-[var(--muted-foreground)]"
            >
              {metCount} of {totalCount} met
            </span>
          </div>
          {totalCount === 0 ? (
            <p className="text-sm text-[var(--muted-foreground)]">
              No criteria configured yet.
            </p>
          ) : (
            <ul
              data-testid="trust-grade-criteria-list"
              className="divide-y divide-[var(--border)] rounded-[var(--radius-md)] border border-[var(--border)]"
            >
              {criteria.map((c) => (
                <li
                  key={c.name}
                  data-testid="trust-grade-criterion"
                  data-met={c.met || undefined}
                  className="flex items-center gap-3 px-3 py-2 text-sm"
                >
                  <span
                    aria-hidden
                    className={cn(
                      "inline-grid size-5 place-items-center rounded-full",
                      c.met
                        ? "bg-[var(--success)]/15 text-[var(--success)]"
                        : "bg-[var(--muted)] text-[var(--muted-foreground)]",
                    )}
                  >
                    {c.met ? <Check className="size-3" /> : <X className="size-3" />}
                  </span>
                  <span className="flex-1 text-[var(--foreground)]">{c.name}</span>
                  <span className="text-xs text-[var(--muted-foreground)] whitespace-nowrap">
                    {c.currentValue} → {c.targetValue}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
