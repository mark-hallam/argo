"use client";

import { useState } from "react";
import { Button } from "@/components/ui/Button";
import type { RetrainProposal } from "@/lib/api/retrain";

/**
 * Retrain Step 3 — Propose.
 *
 * Renders the classifier's plain-English description of the change
 * plus a `[Try it on 3 recent cases]` side panel. Continue button
 * advances to the next applicable step (4 if conflicts, else 5 if
 * regressions, else 6).
 */

export interface RetrainStepProposeProps {
  workerName: string;
  proposal: RetrainProposal;
  onContinue: () => void;
  onBack: () => void;
  onCancel: () => void;
}

const KIND_LABEL: Record<RetrainProposal["classification"], string> = {
  memory: "Memory",
  policy: "Policy",
  skill: "Voice",
  vocabulary: "Vocabulary",
  "action-ban": "Action ban",
};

export function RetrainStepPropose({
  workerName,
  proposal,
  onContinue,
  onBack,
  onCancel,
}: RetrainStepProposeProps) {
  const [showSamples, setShowSamples] = useState(false);

  return (
    <div data-testid="retrain-step-propose" className="space-y-4">
      <header className="space-y-1">
        <p
          data-testid="retrain-classification-chip"
          className="inline-flex items-center rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-0.5 text-xs font-medium text-[var(--foreground)]"
        >
          {KIND_LABEL[proposal.classification]}
        </p>
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Here&apos;s how {workerName} understood that
        </h2>
      </header>

      <p
        data-testid="retrain-plain-english"
        className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-4 py-3 text-sm leading-relaxed text-[var(--foreground)]"
      >
        {proposal.plainEnglish}
      </p>

      <div className="flex items-center justify-between gap-3 text-sm">
        <Button
          variant="ghost"
          size="sm"
          data-testid="retrain-try-samples"
          onClick={() => setShowSamples((s) => !s)}
        >
          {showSamples ? "Hide samples" : "Try it on 3 recent cases"}
        </Button>
        {proposal.conflicts && proposal.conflicts.length > 0 ? (
          <span
            data-testid="retrain-propose-conflict-hint"
            className="text-xs text-[var(--warning-foreground)]"
          >
            Heads up — this conflicts with an existing rule.
          </span>
        ) : null}
      </div>

      {showSamples ? (
        <div
          data-testid="retrain-samples-panel"
          className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-3 text-sm"
        >
          <p className="text-xs font-medium text-[var(--muted-foreground)]">
            With this rule applied to 3 recent cases:
          </p>
          <ul className="mt-2 space-y-2">
            {(proposal.trySamples ?? []).map((s) => (
              <li
                key={s.caseId}
                data-testid={`retrain-sample-${s.caseId}`}
                className="space-y-1 rounded-[var(--radius-sm)] border border-[var(--border)]/60 px-3 py-2"
              >
                <p className="text-xs text-[var(--muted-foreground)]">{s.context}</p>
                <p className="text-sm text-[var(--foreground)]">{s.newResponse}</p>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

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
            onClick={onContinue}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
