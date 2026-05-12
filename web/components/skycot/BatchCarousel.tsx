"use client";

import { ChevronLeft, ChevronRight, Check, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ApprovalCard, type ApprovalDecision } from "@/components/skycot/ApprovalCard";
import { Button } from "@/components/ui/Button";
import type { ApprovalItem } from "@/lib/sample-approvals";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import { cn } from "@/lib/utils";

/**
 * The "Review each" surface. Replaces a similarity group's stack of
 * cards with a single-card-at-a-time view: prev/next arrows, decided
 * cards collapse to a chip strip at the top, Escape closes.
 *
 * Closes automatically once every card has a decision (controlled by
 * the host via `decisionsById`).
 */

export interface BatchCarouselProps {
  items: ApprovalItem[];
  /** Host-side decision map, keyed by approval id. */
  decisionsById: Record<string, ApprovalDecision | null | undefined>;
  /** Redaction policy threaded through to each rendered card. */
  redactionPolicy?: RedactionPolicy;
  /** Fired when the user resolves a card inside the carousel. */
  onDecide: (approvalId: string, decision: ApprovalDecision) => void;
  /** Fired when the user undoes a card inside the carousel. */
  onUndo: (approvalId: string) => void;
  /** Fired when the user dismisses the carousel. */
  onClose: () => void;
  /** Headline for the carousel — typically the BatchActionBar copy. */
  headline: string;
  className?: string;
}

export function BatchCarousel({
  items,
  decisionsById,
  redactionPolicy,
  onDecide,
  onUndo,
  onClose,
  headline,
  className,
}: BatchCarouselProps) {
  const [index, setIndex] = useState(0);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const safeIndex = Math.min(index, Math.max(0, items.length - 1));
  const current = items[safeIndex];

  const allDecided = useMemo(
    () => items.every((it) => decisionsById[it.id] != null),
    [items, decisionsById],
  );

  useEffect(() => {
    if (allDecided) onClose();
  }, [allDecided, onClose]);

  const goPrev = useCallback(() => {
    setIndex((i) => Math.max(0, i - 1));
  }, []);
  const goNext = useCallback(() => {
    setIndex((i) => Math.min(items.length - 1, i + 1));
  }, [items.length]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "ArrowLeft") {
        goPrev();
      } else if (e.key === "ArrowRight") {
        goNext();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, goPrev, goNext]);

  // Touch swipe (light-touch — no extra dep for v1).
  const touchStartX = useRef<number | null>(null);
  function onTouchStart(e: React.TouchEvent) {
    touchStartX.current = e.touches[0]?.clientX ?? null;
  }
  function onTouchEnd(e: React.TouchEvent) {
    const start = touchStartX.current;
    touchStartX.current = null;
    if (start == null) return;
    const end = e.changedTouches[0]?.clientX;
    if (end == null) return;
    const dx = end - start;
    if (Math.abs(dx) < 40) return;
    if (dx > 0) goPrev();
    else goNext();
  }

  if (!current) return null;

  return (
    <div
      ref={containerRef}
      role="dialog"
      aria-label={headline}
      data-testid="batch-carousel"
      className={cn(
        "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] " +
          "shadow-sm overflow-hidden",
        className,
      )}
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      <div className="flex items-center justify-between gap-3 px-4 py-3 border-b border-[var(--border)] bg-[var(--muted)]/40">
        <div className="text-sm font-medium text-[var(--foreground)] truncate">{headline}</div>
        <Button
          variant="ghost"
          size="sm"
          onClick={onClose}
          aria-label="Close carousel"
          data-testid="batch-carousel-close"
        >
          <X />
        </Button>
      </div>

      <ProgressStrip
        items={items}
        decisionsById={decisionsById}
        currentIndex={safeIndex}
        onJump={setIndex}
      />

      <div className="px-3 pb-4 pt-3 flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          aria-label="Previous"
          onClick={goPrev}
          disabled={safeIndex === 0}
          data-testid="batch-carousel-prev"
        >
          <ChevronLeft />
        </Button>

        <div className="flex-1 min-w-0">
          <ApprovalCard
            className="max-w-full"
            worker={current.worker}
            verb={current.verb}
            detail={current.detail}
            consequenceIfYes={current.consequenceIfYes}
            consequenceIfNo={current.consequenceIfNo}
            reason={current.reason}
            channel={current.channel}
            receivedAt={current.receivedAt}
            decision={decisionsById[current.id] ?? null}
            redactionPolicy={redactionPolicy}
            onDecide={(d) => {
              onDecide(current.id, d);
              // Auto-advance to the next un-decided card.
              const next = findNextUndecided(items, decisionsById, safeIndex);
              if (next != null) setIndex(next);
            }}
            onUndo={() => onUndo(current.id)}
          />
        </div>

        <Button
          variant="ghost"
          size="icon"
          aria-label="Next"
          onClick={goNext}
          disabled={safeIndex >= items.length - 1}
          data-testid="batch-carousel-next"
        >
          <ChevronRight />
        </Button>
      </div>
    </div>
  );
}

function findNextUndecided(
  items: ApprovalItem[],
  decisionsById: Record<string, ApprovalDecision | null | undefined>,
  from: number,
): number | null {
  for (let i = from + 1; i < items.length; i++) {
    if (decisionsById[items[i].id] == null) return i;
  }
  for (let i = 0; i < from; i++) {
    if (decisionsById[items[i].id] == null) return i;
  }
  return null;
}

function ProgressStrip({
  items,
  decisionsById,
  currentIndex,
  onJump,
}: {
  items: ApprovalItem[];
  decisionsById: Record<string, ApprovalDecision | null | undefined>;
  currentIndex: number;
  onJump: (i: number) => void;
}) {
  return (
    <div
      role="list"
      aria-label="Progress"
      className="flex flex-wrap items-center gap-1.5 px-4 py-2 border-b border-[var(--border)] bg-[var(--background)]"
    >
      {items.map((item, i) => {
        const d = decisionsById[item.id];
        const isCurrent = i === currentIndex;
        const tone =
          d === "approve"
            ? "bg-[var(--success)]/15 text-[var(--success)] border-[var(--success)]/30"
            : d === "refuse"
              ? "bg-[var(--destructive)]/10 text-[var(--destructive)] border-[var(--destructive)]/30"
              : "bg-[var(--muted)] text-[var(--muted-foreground)] border-[var(--border)]";
        return (
          <button
            key={item.id}
            role="listitem"
            type="button"
            onClick={() => onJump(i)}
            data-testid={`batch-carousel-chip-${item.id}`}
            className={cn(
              "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs",
              tone,
              isCurrent && "ring-2 ring-[var(--ring)] ring-offset-1 ring-offset-[var(--background)]",
            )}
          >
            {d === "approve" ? <Check className="size-3" /> : d === "refuse" ? <X className="size-3" /> : null}
            <span>#{i + 1}</span>
          </button>
        );
      })}
    </div>
  );
}
