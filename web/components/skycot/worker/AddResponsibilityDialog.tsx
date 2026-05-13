"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { CheckCircle2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  CRON_PRESETS,
  Step1Capture,
  type CronPreset,
} from "@/components/skycot/worker/duty/Step1Capture";
import { Step2Classify } from "@/components/skycot/worker/duty/Step2Classify";
import { cronGloss, Step3Draft } from "@/components/skycot/worker/duty/Step3Draft";
import { Step4Conflict } from "@/components/skycot/worker/duty/Step4Conflict";
import { useDutyAdd } from "@/lib/hooks/use-duty-add";
import type {
  CommitDutyResult,
  ConflictResolution,
  DutyProposal,
} from "@/lib/api/duties";

/**
 * AddResponsibilityDialog — 6-step plain-English duty-add flow.
 *
 *   1. Capture     — textarea + 5 cron-preset chips
 *   2. Classify    — spinner / error while the classifier runs
 *   3. Draft       — proposal + cron preview + 3 samples
 *   4. Conflict    — overlapping-duty resolution (conditional)
 *   5. Commit      — "✓ Add this duty"
 *   6. Confirmation toast — handled by the parent via onCommitted
 *
 * Step 6 happens after the dialog closes — `onCommitted` carries the
 * commit payload (duty + next-fire) so the parent can render the
 * acceptance toast + "Re-run on recent cases" CTA.
 *
 * Mock fallback for all three mutations lives in `lib/api/duties.ts`.
 */

export interface AddResponsibilityDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workerId: string;
  workerName: string;
  onCommitted?: (result: CommitDutyResult) => void;
}

type Step = 1 | 2 | 3 | 4 | 5;

interface DialogState {
  step: Step;
  input: string;
  presetId: string | null;
  cronSpec: string;
  proposal: DutyProposal | null;
  proposalError: string | null;
  conflictChoice: ConflictResolution | null;
  replacedDutyId: string | null;
  committed: CommitDutyResult | null;
  commitError: string | null;
}

const INITIAL_PRESET = CRON_PRESETS[0];

const INITIAL: DialogState = {
  step: 1,
  input: "",
  presetId: INITIAL_PRESET.id,
  cronSpec: INITIAL_PRESET.spec,
  proposal: null,
  proposalError: null,
  conflictChoice: null,
  replacedDutyId: null,
  committed: null,
  commitError: null,
};

function nextAfterDraft(p: DutyProposal): Step {
  if ((p.conflicts?.length ?? 0) > 0) return 4;
  return 5;
}

export function AddResponsibilityDialog({
  open,
  onOpenChange,
  workerId,
  workerName,
  onCommitted,
}: AddResponsibilityDialogProps) {
  const actions = useDutyAdd(workerId);
  const [state, setState] = useState<DialogState>(INITIAL);

  useEffect(() => {
    if (!open) setState(INITIAL);
  }, [open]);

  const runClassify = useCallback(async () => {
    setState((s) => ({ ...s, step: 2, proposalError: null }));
    try {
      const { proposal } = await actions.classify({
        input: state.input,
        schedule: { kind: "cron", spec: state.cronSpec },
      });
      setState((s) => ({ ...s, proposal, step: 3 }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setState((s) => ({ ...s, proposalError: message }));
    }
  }, [actions, state.input, state.cronSpec]);

  const runCommit = useCallback(async () => {
    if (!state.proposal) return;
    setState((s) => ({ ...s, commitError: null }));
    try {
      const result = await actions.commit({
        proposal: state.proposal,
        conflictResolution: state.conflictChoice ?? undefined,
        replacedDutyId: state.replacedDutyId ?? undefined,
      });
      setState((s) => ({ ...s, committed: result }));
      onCommitted?.(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setState((s) => ({ ...s, commitError: message }));
    }
  }, [
    actions,
    state.proposal,
    state.conflictChoice,
    state.replacedDutyId,
    onCommitted,
  ]);

  const close = useCallback(() => onOpenChange(false), [onOpenChange]);

  const handleSelectPreset = useCallback(
    (preset: CronPreset | null, customSpec: string) =>
      setState((s) => ({
        ...s,
        presetId: preset?.id ?? null,
        cronSpec: preset ? preset.spec : customSpec,
      })),
    [],
  );

  const totalSteps = state.proposal && (state.proposal.conflicts?.length ?? 0) > 0 ? 6 : 5;
  const visibleStep = state.committed ? 6 : Math.min(state.step + (state.step >= 4 ? 0 : 0), totalSteps);
  const stepIndicator = `Step ${visibleStep} of ${totalSteps === 5 ? 5 : 6}`;

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
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-xl -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none max-h-[85vh] overflow-y-auto",
          )}
          data-testid="duty-dialog"
          data-step={state.committed ? 6 : state.step}
        >
          <DialogPrimitive.Title className="sr-only">
            Add a responsibility for {workerName}
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="sr-only">
            Plain-English duty add flow for {workerName}.
          </DialogPrimitive.Description>
          <div
            data-testid="duty-step-indicator"
            className="mb-4 text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]"
          >
            {stepIndicator}
          </div>

          {state.step === 1 ? (
            <Step1Capture
              workerName={workerName}
              value={state.input}
              cronSpec={state.cronSpec}
              presetId={state.presetId}
              onChangeValue={(input) => setState((s) => ({ ...s, input }))}
              onSelectPreset={handleSelectPreset}
              onContinue={runClassify}
              onCancel={close}
            />
          ) : null}

          {state.step === 2 ? (
            <Step2Classify
              workerName={workerName}
              error={state.proposalError}
              onRetry={runClassify}
              onCancel={close}
            />
          ) : null}

          {state.step === 3 && state.proposal ? (
            <Step3Draft
              workerName={workerName}
              proposal={state.proposal}
              onAdjust={() => setState((s) => ({ ...s, step: 1 }))}
              onContinue={() =>
                setState((s) => ({ ...s, step: nextAfterDraft(state.proposal!) }))
              }
              onCancel={close}
            />
          ) : null}

          {state.step === 4 && state.proposal ? (
            <Step4Conflict
              workerName={workerName}
              proposal={state.proposal}
              onResolve={(choice, replacedDutyId) =>
                setState((s) => ({
                  ...s,
                  conflictChoice: choice,
                  replacedDutyId: replacedDutyId ?? null,
                  step: 5,
                }))
              }
              onBack={() => setState((s) => ({ ...s, step: 3 }))}
              onCancel={close}
            />
          ) : null}

          {state.step === 5 && state.proposal ? (
            <Step5Commit
              workerName={workerName}
              proposal={state.proposal}
              committed={state.committed}
              isCommitting={actions.isCommitting}
              commitError={state.commitError}
              onCommit={runCommit}
              onClose={close}
            />
          ) : null}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

interface Step5CommitProps {
  workerName: string;
  proposal: DutyProposal;
  committed: CommitDutyResult | null;
  isCommitting: boolean;
  commitError: string | null;
  onCommit: () => void;
  onClose: () => void;
}

function Step5Commit({
  workerName,
  proposal,
  committed,
  isCommitting,
  commitError,
  onCommit,
  onClose,
}: Step5CommitProps) {
  if (!committed) {
    return (
      <div data-testid="duty-step-commit" className="space-y-4">
        <header className="space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            Add this duty to {workerName}?
          </h2>
          <p className="text-sm text-[var(--muted-foreground)]">{proposal.description}</p>
          <p className="text-xs text-[var(--muted-foreground)]">
            {proposal.schedule.kind === "cron"
              ? `${proposal.schedule.spec} — ${cronGloss(proposal.schedule.spec)}`
              : `event: ${proposal.schedule.spec}`}
          </p>
        </header>
        {commitError ? (
          <p
            data-testid="duty-commit-error"
            className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-3 py-2 text-sm text-[var(--destructive-foreground)]"
          >
            Couldn&apos;t add: {commitError}
          </p>
        ) : null}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" data-testid="duty-cancel" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            data-testid="duty-commit"
            onClick={onCommit}
            loading={isCommitting}
            disabled={isCommitting}
          >
            ✓ Add this duty
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div data-testid="duty-step-committed" className="space-y-4">
      <header className="flex items-start gap-3">
        <div className="grid size-9 place-items-center rounded-full bg-[var(--success)]/15 text-[var(--success-foreground)]">
          <CheckCircle2 className="size-5" aria-hidden />
        </div>
        <div className="space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            Added the new responsibility.
          </h2>
          <p
            data-testid="duty-committed-summary"
            className="text-sm text-[var(--muted-foreground)]"
          >
            {committed.duty.label}.
          </p>
        </div>
      </header>
      <div className="flex justify-end pt-2">
        <Button variant="primary" data-testid="duty-close" onClick={onClose}>
          Done
        </Button>
      </div>
    </div>
  );
}
