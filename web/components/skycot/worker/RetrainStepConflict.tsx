"use client";

import { useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { RetrainConflict, RetrainProposal } from "@/lib/api/retrain";

/**
 * Retrain Step 4 — Conflict resolution.
 *
 * Surfaces the conflict between the proposed rule and an existing
 * one, then offers Replace / Keep both / Cancel. The container
 * advances on Replace or Keep both (recording the choice); Cancel
 * closes the dialog.
 */

export type ConflictResolution = "replace" | "keep-both";

export interface RetrainStepConflictProps {
  workerName: string;
  proposal: RetrainProposal;
  onResolve: (choice: ConflictResolution) => void;
  onBack: () => void;
  onCancel: () => void;
}

function formatRelative(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const days = Math.round(ms / (24 * 60 * 60_000));
  if (days < 1) return "today";
  if (days === 1) return "yesterday";
  if (days < 14) return `${days} days ago`;
  const weeks = Math.round(days / 7);
  return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
}

function specificCasesPhrase(conflict: RetrainConflict): string {
  if (conflict.affectedCaseCount === 1) return "1 case";
  return `${conflict.affectedCaseCount} cases`;
}

export function RetrainStepConflict({
  workerName,
  proposal,
  onResolve,
  onBack,
  onCancel,
}: RetrainStepConflictProps) {
  const [choice, setChoice] = useState<ConflictResolution>("replace");
  const conflict = proposal.conflicts?.[0];

  if (!conflict) {
    return (
      <div data-testid="retrain-step-conflict-empty" className="space-y-3 text-sm">
        <p className="text-[var(--muted-foreground)]">
          No conflicts detected — continue to the next step.
        </p>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" data-testid="retrain-back" onClick={onBack}>
            Back
          </Button>
          <Button
            variant="primary"
            data-testid="retrain-continue"
            onClick={() => onResolve("replace")}
          >
            Continue
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div data-testid="retrain-step-conflict" className="space-y-4">
      <header className="flex items-start gap-3">
        <div className="grid size-9 place-items-center rounded-full bg-[var(--warning)]/15 text-[var(--warning-foreground)]">
          <AlertTriangle className="size-4" aria-hidden />
        </div>
        <div className="flex-1 space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            Heads up — this conflicts with another rule
          </h2>
          <p className="text-sm text-[var(--muted-foreground)]">
            {workerName} already has a rule that says something different.
          </p>
        </div>
      </header>

      <div className="space-y-3">
        <div
          data-testid="retrain-conflict-existing"
          className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-4 py-3"
        >
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
            Existing rule (added {formatRelative(conflict.existingRuleAddedAt)})
          </p>
          <p className="mt-1 text-sm text-[var(--foreground)]">
            {conflict.existingRule}
          </p>
        </div>
        <div
          data-testid="retrain-conflict-new"
          className="rounded-[var(--radius-md)] border border-[var(--primary)]/40 bg-[var(--primary)]/5 px-4 py-3"
        >
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--primary)]">
            New rule
          </p>
          <p className="mt-1 text-sm text-[var(--foreground)]">
            {proposal.plainEnglish}
          </p>
        </div>
      </div>

      <fieldset
        data-testid="retrain-conflict-choices"
        className="space-y-2 rounded-[var(--radius-md)] border border-[var(--border)] px-4 py-3"
      >
        <legend className="px-1 text-xs font-medium text-[var(--muted-foreground)]">
          What should I do?
        </legend>
        <label className="flex items-start gap-2 text-sm">
          <input
            type="radio"
            name="conflict-resolution"
            value="replace"
            checked={choice === "replace"}
            onChange={() => setChoice("replace")}
            data-testid="retrain-conflict-replace"
            className="mt-0.5"
          />
          <span>
            Replace the old rule with the new one.
          </span>
        </label>
        <label className="flex items-start gap-2 text-sm">
          <input
            type="radio"
            name="conflict-resolution"
            value="keep-both"
            checked={choice === "keep-both"}
            onChange={() => setChoice("keep-both")}
            data-testid="retrain-conflict-keep-both"
            className="mt-0.5"
          />
          <span>
            Keep both — the new rule wins for {specificCasesPhrase(conflict)}.
          </span>
        </label>
      </fieldset>

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
            onClick={() => onResolve(choice)}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
