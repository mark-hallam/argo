"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { RetrainProposal, RetrainRegressionDiff } from "@/lib/api/retrain";

/**
 * Retrain Step 5 — Regression preview.
 *
 * Shows 3-5 prior cases where the new rule would change the
 * worker's output. Each row carries an outcome tag (safe / review /
 * blocked) and a per-row accept toggle. Commit applies the new rule
 * to all "safe" cases by default; "review" / "blocked" rows must be
 * explicitly toggled in.
 */

export interface RetrainStepRegressionProps {
  workerName: string;
  proposal: RetrainProposal;
  onContinue: (acceptedCaseIds: string[]) => void;
  onBack: () => void;
  onCancel: () => void;
}

const OUTCOME_LABEL: Record<RetrainRegressionDiff["outcome"], string> = {
  safe: "safe",
  review: "review",
  blocked: "blocked",
};

const OUTCOME_STYLE: Record<RetrainRegressionDiff["outcome"], string> = {
  safe: "bg-[var(--success)]/15 text-[var(--success-foreground)] border-[var(--success)]/30",
  review: "bg-[var(--warning)]/15 text-[var(--warning-foreground)] border-[var(--warning)]/30",
  blocked: "bg-[var(--destructive)]/15 text-[var(--destructive-foreground)] border-[var(--destructive)]/30",
};

function defaultAccepted(diffs: RetrainRegressionDiff[]): Set<string> {
  return new Set(diffs.filter((d) => d.outcome === "safe").map((d) => d.caseId));
}

export function RetrainStepRegression({
  workerName,
  proposal,
  onContinue,
  onBack,
  onCancel,
}: RetrainStepRegressionProps) {
  const diffs = proposal.regressionPreview ?? [];
  const [accepted, setAccepted] = useState<Set<string>>(() => defaultAccepted(diffs));

  // Re-derive when the proposal swaps (e.g. on re-classify).
  useEffect(() => {
    setAccepted(defaultAccepted(diffs));
  }, [proposal]); // eslint-disable-line react-hooks/exhaustive-deps

  const toggle = (caseId: string) => {
    setAccepted((prev) => {
      const next = new Set(prev);
      if (next.has(caseId)) next.delete(caseId);
      else next.add(caseId);
      return next;
    });
  };

  if (diffs.length === 0) {
    return (
      <div data-testid="retrain-step-regression-empty" className="space-y-3 text-sm">
        <p className="text-[var(--muted-foreground)]">
          No prior cases would change behavior under the new rule.
        </p>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" data-testid="retrain-back" onClick={onBack}>
            Back
          </Button>
          <Button
            variant="primary"
            data-testid="retrain-continue"
            onClick={() => onContinue([])}
          >
            Continue
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div data-testid="retrain-step-regression" className="space-y-4">
      <header className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Re-checking {workerName}&apos;s recent work
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Here&apos;s what changes if the new rule applies. Safe rows
          are pre-selected; review and blocked rows need your call.
        </p>
      </header>

      <ul className="space-y-2" data-testid="retrain-regression-list">
        {diffs.map((d) => {
          const isAccepted = accepted.has(d.caseId);
          return (
            <li
              key={d.caseId}
              data-testid={`retrain-regression-row-${d.caseId}`}
              data-outcome={d.outcome}
              data-accepted={isAccepted ? "true" : "false"}
              className="rounded-[var(--radius-md)] border border-[var(--border)] px-4 py-3"
            >
              <div className="flex items-start justify-between gap-3">
                <p className="text-xs text-[var(--muted-foreground)]">
                  {d.context}
                </p>
                <span
                  className={cn(
                    "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium",
                    OUTCOME_STYLE[d.outcome],
                  )}
                  data-testid={`retrain-regression-outcome-${d.caseId}`}
                >
                  {OUTCOME_LABEL[d.outcome]}
                </span>
              </div>
              <div className="mt-2 grid grid-cols-1 gap-2 sm:grid-cols-2 text-sm">
                <div>
                  <p className="text-xs font-medium text-[var(--muted-foreground)]">Before</p>
                  <p className="text-[var(--foreground)]">{d.before}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-[var(--muted-foreground)]">After</p>
                  <p className="text-[var(--foreground)]">{d.after}</p>
                </div>
              </div>
              <label className="mt-2 flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
                <input
                  type="checkbox"
                  checked={isAccepted}
                  onChange={() => toggle(d.caseId)}
                  data-testid={`retrain-regression-accept-${d.caseId}`}
                />
                Apply new rule to this case
              </label>
            </li>
          );
        })}
      </ul>

      <div className="flex items-center justify-between gap-2 pt-2">
        <Button variant="ghost" data-testid="retrain-back" onClick={onBack}>
          Back
        </Button>
        <div className="flex gap-2">
          <Button variant="ghost" data-testid="retrain-cancel" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            variant="primary"
            data-testid="retrain-continue"
            onClick={() => onContinue(Array.from(accepted))}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
