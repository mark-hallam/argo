"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { CheckCircle2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  RetrainStepCapture,
} from "@/components/skycot/worker/RetrainStepCapture";
import {
  RetrainStepClassify,
} from "@/components/skycot/worker/RetrainStepClassify";
import {
  RetrainStepPropose,
} from "@/components/skycot/worker/RetrainStepPropose";
import {
  RetrainStepConflict,
  type ConflictResolution,
} from "@/components/skycot/worker/RetrainStepConflict";
import {
  RetrainStepRegression,
} from "@/components/skycot/worker/RetrainStepRegression";
import { useRetrainActions } from "@/lib/hooks/use-retrain";
import type {
  RetrainProposal,
  RetrainRecord,
  ReverseTestCase,
} from "@/lib/api/retrain";

/**
 * RetrainDialog — 6-step plain-English correction flow.
 *
 *   1. Capture    — free-text textarea
 *   2. Classify   — spinner while the classifier runs
 *   3. Propose    — plain-English description + "Try it on 3 recent cases"
 *   4. Conflict   — surfaces a conflicting existing rule (skipped when none)
 *   5. Regression — preview of behavior changes on prior cases (skipped when none)
 *   6. Commit     — "✓ Train it in" mutation + success toast + reverse-test offer
 *
 * Step 4 and Step 5 are conditional; the classifier's response
 * carries `conflicts` and `regressionPreview` flags that drive
 * whether the step appears. The dialog uses the standard
 * `@radix-ui/react-dialog` primitive (same as `AddFactDialog`).
 *
 * Persistence lives behind `useRetrainActions`, which wires
 * classify / commit / undo / reverse-test mutations against the
 * shared TanStack-Query cache. v1 falls back to a mock when the
 * gateway endpoint isn't wired (sibling brief
 * `mission-gateway-retrain-endpoints`).
 *
 * Hard caps (per HARD RULE 6): ≤ 450 LOC; each step component ≤
 * its own limit.
 */

export interface RetrainDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workerId: string;
  workerName: string;
  /** Where in the surface the dialog was launched from. */
  triggerContext: "charter" | "voice" | "audit";
  /** Optional callback fired after a successful commit. */
  onCommitted?: (record: RetrainRecord) => void;
}

type Step = 1 | 2 | 3 | 4 | 5 | 6;

interface DialogState {
  step: Step;
  input: string;
  proposal: RetrainProposal | null;
  proposalError: string | null;
  conflictChoice: ConflictResolution | null;
  acceptedCaseIds: string[];
  commitError: string | null;
  committed: RetrainRecord | null;
  reverseTestOpen: boolean;
  reverseCases: ReverseTestCase[] | null;
  reverseError: string | null;
}

const INITIAL: DialogState = {
  step: 1,
  input: "",
  proposal: null,
  proposalError: null,
  conflictChoice: null,
  acceptedCaseIds: [],
  commitError: null,
  committed: null,
  reverseTestOpen: false,
  reverseCases: null,
  reverseError: null,
};

function nextAfterPropose(p: RetrainProposal): Step {
  if ((p.conflicts?.length ?? 0) > 0) return 4;
  if ((p.regressionPreview?.length ?? 0) > 0) return 5;
  return 6;
}

function nextAfterConflict(p: RetrainProposal): Step {
  if ((p.regressionPreview?.length ?? 0) > 0) return 5;
  return 6;
}

export function RetrainDialog({
  open,
  onOpenChange,
  workerId,
  workerName,
  triggerContext,
  onCommitted,
}: RetrainDialogProps) {
  const actions = useRetrainActions(workerId);
  const [state, setState] = useState<DialogState>(INITIAL);

  // Reset state every time the dialog closes so a re-open starts fresh.
  useEffect(() => {
    if (!open) setState(INITIAL);
  }, [open]);

  const runClassify = useCallback(async () => {
    setState((s) => ({ ...s, step: 2, proposalError: null }));
    try {
      const { proposal } = await actions.classify({
        input: state.input,
        triggerContext,
      });
      setState((s) => ({ ...s, proposal, step: 3 }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setState((s) => ({ ...s, proposalError: message }));
    }
  }, [actions, state.input, triggerContext]);

  const runCommit = useCallback(async () => {
    if (!state.proposal) return;
    setState((s) => ({ ...s, commitError: null }));
    try {
      const { record } = await actions.commit({
        proposal: state.proposal,
        acceptedCaseIds: state.acceptedCaseIds,
      });
      setState((s) => ({ ...s, committed: record, step: 6 }));
      onCommitted?.(record);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setState((s) => ({ ...s, commitError: message }));
    }
  }, [actions, state.proposal, state.acceptedCaseIds, onCommitted]);

  const openReverseTest = useCallback(async () => {
    if (!state.committed) return;
    setState((s) => ({ ...s, reverseTestOpen: true, reverseError: null }));
    try {
      const { cases } = await actions.reverseTest(state.committed.retrainId);
      setState((s) => ({ ...s, reverseCases: cases }));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unexpected error";
      setState((s) => ({ ...s, reverseError: message }));
    }
  }, [actions, state.committed]);

  const close = useCallback(() => onOpenChange(false), [onOpenChange]);

  const proposal = state.proposal;
  const stepIndicator = `Step ${state.step} of 6`;

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
          data-testid="retrain-dialog"
          data-step={state.step}
        >
          <DialogPrimitive.Title className="sr-only">
            Adjust {workerName}
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="sr-only">
            Plain-English correction flow for {workerName}.
          </DialogPrimitive.Description>
          <div
            data-testid="retrain-step-indicator"
            className="mb-4 text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]"
          >
            {stepIndicator}
          </div>

          {state.step === 1 ? (
            <RetrainStepCapture
              workerName={workerName}
              triggerContext={triggerContext}
              value={state.input}
              onChange={(input) => setState((s) => ({ ...s, input }))}
              onContinue={runClassify}
              onCancel={close}
            />
          ) : null}

          {state.step === 2 ? (
            <RetrainStepClassify
              workerName={workerName}
              error={state.proposalError}
              onRetry={runClassify}
              onCancel={close}
            />
          ) : null}

          {state.step === 3 && proposal ? (
            <RetrainStepPropose
              workerName={workerName}
              proposal={proposal}
              onContinue={() =>
                setState((s) => ({ ...s, step: nextAfterPropose(proposal) }))
              }
              onBack={() => setState((s) => ({ ...s, step: 1 }))}
              onCancel={close}
            />
          ) : null}

          {state.step === 4 && proposal ? (
            <RetrainStepConflict
              workerName={workerName}
              proposal={proposal}
              onResolve={(choice) =>
                setState((s) => ({
                  ...s,
                  conflictChoice: choice,
                  step: nextAfterConflict(proposal),
                }))
              }
              onBack={() => setState((s) => ({ ...s, step: 3 }))}
              onCancel={close}
            />
          ) : null}

          {state.step === 5 && proposal ? (
            <RetrainStepRegression
              workerName={workerName}
              proposal={proposal}
              onContinue={(acceptedCaseIds) =>
                setState((s) => ({ ...s, acceptedCaseIds, step: 6 }))
              }
              onBack={() =>
                setState((s) => ({
                  ...s,
                  step: (proposal.conflicts?.length ?? 0) > 0 ? 4 : 3,
                }))
              }
              onCancel={close}
            />
          ) : null}

          {state.step === 6 && proposal ? (
            <RetrainStepCommit
              workerName={workerName}
              proposal={proposal}
              committed={state.committed}
              isCommitting={actions.isCommitting}
              commitError={state.commitError}
              onCommit={runCommit}
              onClose={close}
              onShowReverseTest={openReverseTest}
              reverseTestOpen={state.reverseTestOpen}
              reverseCases={state.reverseCases}
              reverseError={state.reverseError}
            />
          ) : null}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

interface RetrainStepCommitProps {
  workerName: string;
  proposal: RetrainProposal;
  committed: RetrainRecord | null;
  isCommitting: boolean;
  commitError: string | null;
  onCommit: () => void;
  onClose: () => void;
  onShowReverseTest: () => void;
  reverseTestOpen: boolean;
  reverseCases: ReverseTestCase[] | null;
  reverseError: string | null;
}

function RetrainStepCommit({
  workerName,
  proposal,
  committed,
  isCommitting,
  commitError,
  onCommit,
  onClose,
  onShowReverseTest,
  reverseTestOpen,
  reverseCases,
  reverseError,
}: RetrainStepCommitProps) {
  if (!committed) {
    return (
      <div data-testid="retrain-step-commit" className="space-y-4">
        <header className="space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            Train {workerName} on this?
          </h2>
          <p className="text-sm text-[var(--muted-foreground)]">
            {proposal.plainEnglish}
          </p>
        </header>
        {commitError ? (
          <p
            data-testid="retrain-commit-error"
            className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-3 py-2 text-sm text-[var(--destructive-foreground)]"
          >
            Couldn&apos;t train: {commitError}
          </p>
        ) : null}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" data-testid="retrain-cancel" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            data-testid="retrain-commit"
            onClick={onCommit}
            loading={isCommitting}
            disabled={isCommitting}
          >
            ✓ Train it in
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div data-testid="retrain-step-committed" className="space-y-4">
      <header className="flex items-start gap-3">
        <div className="grid size-9 place-items-center rounded-full bg-[var(--success)]/15 text-[var(--success-foreground)]">
          <CheckCircle2 className="size-5" aria-hidden />
        </div>
        <div className="space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            Trained {workerName} on this.
          </h2>
          <p
            data-testid="retrain-committed-summary"
            className="text-sm text-[var(--muted-foreground)]"
          >
            Going forward, {proposal.oneLineSummary.toLowerCase()}
          </p>
        </div>
      </header>

      <div className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-4 py-3 text-sm">
        <p className="font-medium text-[var(--foreground)]">
          Want me to re-handle 3 recent conversations with the new rule?
        </p>
        <p className="text-xs text-[var(--muted-foreground)]">
          Nothing gets sent — it&apos;s a preview only.
        </p>
        <div className="mt-3 flex gap-2">
          <Button
            variant="ghost"
            size="sm"
            data-testid="retrain-reverse-test-show"
            onClick={onShowReverseTest}
            disabled={reverseTestOpen && !reverseCases && !reverseError}
          >
            {reverseTestOpen ? "Loading…" : "Show me"}
          </Button>
        </div>
      </div>

      {reverseTestOpen ? (
        <div data-testid="retrain-reverse-test-panel" className="space-y-2">
          {reverseError ? (
            <p className="text-sm text-[var(--destructive-foreground)]">
              Couldn&apos;t load the reverse-test: {reverseError}
            </p>
          ) : null}
          {reverseCases ? (
            <ul className="space-y-2">
              {reverseCases.map((c) => (
                <li
                  key={c.caseId}
                  data-testid={`retrain-reverse-row-${c.caseId}`}
                  className="rounded-[var(--radius-sm)] border border-[var(--border)] px-3 py-2 text-sm"
                >
                  <p className="text-xs text-[var(--muted-foreground)]">{c.context}</p>
                  <div className="mt-1 grid grid-cols-1 gap-2 sm:grid-cols-2">
                    <div>
                      <p className="text-xs font-medium text-[var(--muted-foreground)]">Before</p>
                      <p className="text-[var(--foreground)]">{c.before}</p>
                    </div>
                    <div>
                      <p className="text-xs font-medium text-[var(--muted-foreground)]">After</p>
                      <p className="text-[var(--foreground)]">{c.after}</p>
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      <div className="flex justify-end pt-2">
        <Button variant="primary" data-testid="retrain-close" onClick={onClose}>
          Done
        </Button>
      </div>
    </div>
  );
}
