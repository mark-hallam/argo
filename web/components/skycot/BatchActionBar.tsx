"use client";

import { Check, Layers, RotateCcw, Undo2 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { actionNoun, type ApprovalGroup } from "@/lib/similarity/group-approvals";
import { cn } from "@/lib/utils";

/**
 * Banner above the first card in a similarity group. Additive only —
 * each card keeps its own approve/refuse buttons. The bar offers a
 * one-tap "Approve all N" and an inline "Review each" carousel.
 *
 * Refuse-all is intentionally not offered (per journey doc: refusal
 * requires per-case judgment, batch is approve-only).
 *
 * Spec: docs/build-plan/streams/E1-d-approvals-batch-decide.md.
 */

export type BatchActionBarPhase = "idle" | "loading" | "settled";

export interface BatchActionBarProps {
  group: ApprovalGroup;
  /** Workflow phase — drives copy + button availability. */
  phase: BatchActionBarPhase;
  /** Fired when the approver taps "Approve all N". */
  onApproveAll: () => void;
  /** Fired when the approver taps "Review each". */
  onReviewEach: () => void;
  /** Live count of resolved-OK mutations during loading. */
  completedCount?: number;
  /** Live count of failed mutations during loading / after settle. */
  failedCount?: number;
  /** When phase = settled and undo is still in window. */
  onUndoAll?: () => void;
  /** When phase = settled with partial failures. */
  onRetryFailed?: () => void;
  className?: string;
}

export function BatchActionBar({
  group,
  phase,
  onApproveAll,
  onReviewEach,
  completedCount = 0,
  failedCount = 0,
  onUndoAll,
  onRetryFailed,
  className,
}: BatchActionBarProps) {
  const total = group.items.length;
  const [, plural] = actionNoun(group.actionType);
  const headline = `${group.worker.name} proposed ${total} similar ${plural}`;

  return (
    <div
      role="region"
      aria-label={`Batch actions for ${group.worker.name}'s ${plural}`}
      data-testid={`batch-action-bar-${group.key}`}
      className={cn(
        "flex flex-wrap items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] " +
          "bg-[var(--accent)]/40 px-4 py-3 text-sm",
        className,
      )}
    >
      <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)]">
        <Layers className="size-4" />
      </span>

      <div className="flex-1 min-w-0 leading-snug">
        <div className="font-medium text-[var(--foreground)] truncate">{headline}</div>
        <BatchStatusLine
          phase={phase}
          total={total}
          completed={completedCount}
          failed={failedCount}
          worker={group.worker.name}
          plural={plural}
        />
      </div>

      {phase === "idle" ? (
        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            size="sm"
            onClick={onApproveAll}
            data-testid={`batch-approve-all-${group.key}`}
          >
            <Check />
            Approve all {total}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={onReviewEach}
            data-testid={`batch-review-each-${group.key}`}
          >
            Review each
          </Button>
        </div>
      ) : null}

      {phase === "loading" ? (
        <Button variant="primary" size="sm" disabled loading data-testid={`batch-approving-${group.key}`}>
          Approving {Math.max(0, total - completedCount - failedCount)}…
        </Button>
      ) : null}

      {phase === "settled" ? (
        <div className="flex items-center gap-2">
          {failedCount > 0 && onRetryFailed ? (
            <Button
              variant="secondary"
              size="sm"
              onClick={onRetryFailed}
              data-testid={`batch-retry-${group.key}`}
            >
              <RotateCcw />
              Retry {failedCount}
            </Button>
          ) : null}
          {onUndoAll ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={onUndoAll}
              data-testid={`batch-undo-${group.key}`}
            >
              <Undo2 />
              Undo
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function BatchStatusLine({
  phase,
  total,
  completed,
  failed,
  worker,
  plural,
}: {
  phase: BatchActionBarPhase;
  total: number;
  completed: number;
  failed: number;
  worker: string;
  plural: string;
}) {
  if (phase === "idle") {
    return (
      <span className="text-xs text-[var(--muted-foreground)]">
        Tap to approve in one go, or step through them.
      </span>
    );
  }
  if (phase === "loading") {
    const settled = completed + failed;
    return (
      <span className="text-xs text-[var(--muted-foreground)]">
        {settled} of {total} settled
      </span>
    );
  }
  // settled
  if (failed === 0) {
    return (
      <span className="text-xs text-[var(--success)]">
        Approved {completed} of {worker}&apos;s {plural}
      </span>
    );
  }
  return (
    <span className="text-xs text-[var(--destructive)]">
      Approved {completed} — {failed} failed
    </span>
  );
}
