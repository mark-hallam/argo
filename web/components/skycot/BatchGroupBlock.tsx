"use client";

import dynamic from "next/dynamic";
import { useMemo } from "react";
import { ApprovalCard, type ApprovalDecision } from "@/components/skycot/ApprovalCard";
import { BatchActionBar } from "@/components/skycot/BatchActionBar";
import {
  actionNoun,
  groupApprovals as groupBySimilarity,
  type ApprovalGroup,
} from "@/lib/similarity/group-approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";
import type { RedactionPolicy } from "@/lib/redaction/redact";

// Carousel is opt-in — only renders when the approver chooses
// "Review each". Code-split to keep the inbox's First Load JS lean;
// the carousel pulls ApprovalCard, swipe handlers, and the chip
// strip together.
const BatchCarousel = dynamic(
  () => import("@/components/skycot/BatchCarousel").then((m) => ({ default: m.BatchCarousel })),
  { ssr: false },
);

/**
 * The "additive batch" surface for a similarity group:
 * - Renders `BatchActionBar` above each member card.
 * - Each card keeps its individual approve/refuse buttons.
 * - If "Review each" is active, replaces the stack with `BatchCarousel`.
 *
 * Lifted out of `app/(app)/page.tsx` to keep that file under the
 * 400-LOC brief budget. State (ui, decisions) is host-owned.
 */

export type BatchPhase = "idle" | "loading" | "settled";

export interface BatchUiState {
  phase: BatchPhase;
  completed: number;
  failed: number;
  failedIds: string[];
  carouselOpen: boolean;
}

export const IDLE_BATCH_UI: BatchUiState = {
  phase: "idle",
  completed: 0,
  failed: 0,
  failedIds: [],
  carouselOpen: false,
};

export interface BatchGroupBlockProps {
  group: ApprovalGroup;
  ui: BatchUiState;
  decisionsById: Record<string, ApprovalDecision | null | undefined>;
  redactionPolicy?: RedactionPolicy;
  onDecide: (id: string, decision: ApprovalDecision) => void;
  onUndo: (id: string) => void;
  onApproveAll: (group: ApprovalGroup, idsOverride?: string[]) => void;
  onReviewEach: (group: ApprovalGroup) => void;
  onCloseCarousel: () => void;
  /** Undoes every approval the batch produced. Only fired from the
   * settled-state Undo affordance on the bar. */
  onUndoBatch: () => void;
}

export function BatchGroupBlock({
  group,
  ui,
  decisionsById,
  redactionPolicy,
  onDecide,
  onUndo,
  onApproveAll,
  onReviewEach,
  onCloseCarousel,
  onUndoBatch,
}: BatchGroupBlockProps) {
  const [, plural] = actionNoun(group.actionType);
  const headline = `${group.worker.name} proposed ${group.items.length} similar ${plural}`;
  const failedIdSet = useMemo(() => new Set(ui.failedIds), [ui.failedIds]);

  if (ui.carouselOpen) {
    return (
      <BatchCarousel
        items={group.items}
        decisionsById={decisionsById}
        redactionPolicy={redactionPolicy}
        onDecide={onDecide}
        onUndo={onUndo}
        onClose={onCloseCarousel}
        headline={headline}
      />
    );
  }

  return (
    <div className="space-y-3">
      <BatchActionBar
        group={group}
        phase={ui.phase}
        completedCount={ui.completed}
        failedCount={ui.failed}
        onApproveAll={() => onApproveAll(group)}
        onReviewEach={() => onReviewEach(group)}
        onRetryFailed={ui.failedIds.length > 0 ? () => onApproveAll(group, ui.failedIds) : undefined}
        onUndoAll={ui.phase === "settled" && ui.completed > 0 ? onUndoBatch : undefined}
      />
      {group.items.map((item) => (
        <BatchGroupItem
          key={item.id}
          item={item}
          failed={failedIdSet.has(item.id)}
          redactionPolicy={redactionPolicy}
          onDecide={onDecide}
          onUndo={onUndo}
        />
      ))}
    </div>
  );
}

export interface ApprovalDaySectionProps {
  label: string;
  items: ApprovalItem[];
  decisionsById: Record<string, ApprovalDecision | null | undefined>;
  batchUi: Record<string, BatchUiState>;
  redactionPolicy?: RedactionPolicy;
  onDecide: (id: string, decision: ApprovalDecision) => void;
  onUndo: (id: string) => void;
  onApproveAll: (group: ApprovalGroup, idsOverride?: string[]) => void;
  onReviewEach: (group: ApprovalGroup) => void;
  onCloseCarousel: (groupKey: string) => void;
  onUndoBatch: (groupKey: string) => void;
}

/**
 * A "Today" / "Yesterday" / "Earlier" section. Applies similarity
 * grouping WITHIN the day's items (batches don't span days). Each
 * group renders as a `BatchGroupBlock`; everything else as a plain
 * `ApprovalCard`.
 */
export function ApprovalDaySection({
  label,
  items,
  decisionsById,
  batchUi,
  redactionPolicy,
  onDecide,
  onUndo,
  onApproveAll,
  onReviewEach,
  onCloseCarousel,
  onUndoBatch,
}: ApprovalDaySectionProps) {
  if (items.length === 0) return null;
  const { units } = groupBySimilarity(items);
  const count = items.length;

  return (
    <section className="space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          {label}
        </h2>
        <div className="flex-1 h-px bg-[var(--border)]" />
        <span className="text-xs text-[var(--muted-foreground)]">
          {count} {count === 1 ? "request" : "requests"}
        </span>
      </div>
      <div className="space-y-3">
        {units.map((unit) =>
          unit.kind === "single" ? (
            <ApprovalCard
              key={unit.item.id}
              className="max-w-full"
              worker={unit.item.worker}
              verb={unit.item.verb}
              detail={unit.item.detail}
              consequenceIfYes={unit.item.consequenceIfYes}
              consequenceIfNo={unit.item.consequenceIfNo}
              reason={unit.item.reason}
              channel={unit.item.channel}
              receivedAt={unit.item.receivedAt}
              decision={unit.item.decision ?? null}
              redactionPolicy={redactionPolicy}
              onDecide={(d) => onDecide(unit.item.id, d)}
              onUndo={() => onUndo(unit.item.id)}
            />
          ) : (
            <BatchGroupBlock
              key={unit.group.key}
              group={unit.group}
              ui={batchUi[unit.group.key] ?? IDLE_BATCH_UI}
              decisionsById={decisionsById}
              redactionPolicy={redactionPolicy}
              onDecide={onDecide}
              onUndo={onUndo}
              onApproveAll={onApproveAll}
              onReviewEach={onReviewEach}
              onCloseCarousel={() => onCloseCarousel(unit.group.key)}
              onUndoBatch={() => onUndoBatch(unit.group.key)}
            />
          ),
        )}
      </div>
    </section>
  );
}

function BatchGroupItem({
  item,
  failed,
  redactionPolicy,
  onDecide,
  onUndo,
}: {
  item: ApprovalItem;
  failed: boolean;
  redactionPolicy?: RedactionPolicy;
  onDecide: (id: string, decision: ApprovalDecision) => void;
  onUndo: (id: string) => void;
}) {
  return (
    <div className="space-y-1.5">
      <ApprovalCard
        className="max-w-full"
        worker={item.worker}
        verb={item.verb}
        detail={item.detail}
        consequenceIfYes={item.consequenceIfYes}
        consequenceIfNo={item.consequenceIfNo}
        reason={item.reason}
        channel={item.channel}
        receivedAt={item.receivedAt}
        decision={item.decision ?? null}
        redactionPolicy={redactionPolicy}
        onDecide={(d) => onDecide(item.id, d)}
        onUndo={() => onUndo(item.id)}
      />
      {failed ? (
        <div
          data-testid={`batch-error-chip-${item.id}`}
          className="ml-2 inline-flex items-center gap-1.5 rounded-full border border-[var(--destructive)]/30 bg-[var(--destructive)]/10 px-2 py-0.5 text-xs text-[var(--destructive)]"
        >
          Couldn&apos;t submit — Retry above.
        </div>
      ) : null}
    </div>
  );
}
