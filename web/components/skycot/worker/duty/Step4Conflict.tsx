"use client";

import { useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { ConflictResolution, DutyProposal } from "@/lib/api/duties";

/**
 * AddResponsibility Step 4 — Conflict resolution.
 *
 * Surfaces an existing-duty overlap (same cron) detected by the
 * classifier and offers Replace / Keep both / Cancel. v1 only
 * surfaces direct cron overlaps — cross-worker / cross-team conflict
 * detection ships in `mission-enterprise-duty-cross-worker-conflicts`.
 */

export interface Step4ConflictProps {
  workerName: string;
  proposal: DutyProposal;
  onResolve: (choice: ConflictResolution, replacedDutyId?: string) => void;
  onBack: () => void;
  onCancel: () => void;
}

export function Step4Conflict({
  workerName,
  proposal,
  onResolve,
  onBack,
  onCancel,
}: Step4ConflictProps) {
  const [choice, setChoice] = useState<ConflictResolution>("replace");
  const conflict = proposal.conflicts?.[0];

  if (!conflict) {
    return (
      <div data-testid="duty-step-conflict-empty" className="space-y-3 text-sm">
        <p className="text-[var(--muted-foreground)]">
          No conflicts detected — continue to the next step.
        </p>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" data-testid="duty-back" onClick={onBack}>
            Back
          </Button>
          <Button
            variant="primary"
            data-testid="duty-continue"
            onClick={() => onResolve("replace")}
          >
            Continue
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div data-testid="duty-step-conflict" className="space-y-4">
      <header className="flex items-start gap-3">
        <div className="grid size-9 place-items-center rounded-full bg-[var(--warning)]/15 text-[var(--warning-foreground)]">
          <AlertTriangle className="size-4" aria-hidden />
        </div>
        <div className="flex-1 space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            This overlaps with one of {workerName}&apos;s existing duties
          </h2>
          <p className="text-sm text-[var(--muted-foreground)]">{conflict.reason}</p>
        </div>
      </header>

      <div className="space-y-3">
        <div
          data-testid="duty-conflict-existing"
          className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-4 py-3"
        >
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
            Existing duty
          </p>
          <p className="mt-1 text-sm text-[var(--foreground)]">
            {conflict.existingDutyLabel} — {conflict.existingSchedule.spec}
          </p>
        </div>
        <div
          data-testid="duty-conflict-new"
          className="rounded-[var(--radius-md)] border border-[var(--primary)]/40 bg-[var(--primary)]/5 px-4 py-3"
        >
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--primary)]">
            New duty
          </p>
          <p className="mt-1 text-sm text-[var(--foreground)]">
            {proposal.label} — {proposal.schedule.spec}
          </p>
        </div>
      </div>

      <fieldset
        data-testid="duty-conflict-choices"
        className="space-y-2 rounded-[var(--radius-md)] border border-[var(--border)] px-4 py-3"
      >
        <legend className="px-1 text-xs font-medium text-[var(--muted-foreground)]">
          What should I do?
        </legend>
        <ChoiceRow
          label="Replace the existing duty with this one."
          value="replace"
          checked={choice === "replace"}
          onSelect={setChoice}
          testId="duty-conflict-replace"
        />
        <ChoiceRow
          label="Keep both — run them in parallel."
          value="keep-both"
          checked={choice === "keep-both"}
          onSelect={setChoice}
          testId="duty-conflict-keep-both"
        />
      </fieldset>

      <div className="flex items-center justify-between gap-2 pt-2">
        <Button variant="ghost" data-testid="duty-back" onClick={onBack}>
          Back
        </Button>
        <div className="flex gap-2">
          <Button variant="ghost" data-testid="duty-cancel" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            variant="primary"
            data-testid="duty-continue"
            onClick={() => onResolve(choice, conflict.existingDutyId)}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}

function ChoiceRow({
  label,
  value,
  checked,
  onSelect,
  testId,
}: {
  label: string;
  value: ConflictResolution;
  checked: boolean;
  onSelect: (next: ConflictResolution) => void;
  testId: string;
}) {
  return (
    <label className="flex items-start gap-2 text-sm">
      <input
        type="radio"
        name="duty-conflict-resolution"
        value={value}
        checked={checked}
        onChange={() => onSelect(value)}
        data-testid={testId}
        className="mt-0.5"
      />
      <span>{label}</span>
    </label>
  );
}
