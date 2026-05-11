"use client";

import { ChevronDown, Clock } from "lucide-react";
import { useEffect, useState } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { cn } from "@/lib/utils";

/**
 * The load-bearing artifact of the SPA. Spec lives in
 * docs/skycot/ux-research/approver.md and
 * docs/skycot/ux-research/journey-approve-on-mobile.md.
 *
 * Structural contract (DO NOT REORDER):
 *   1. Who is asking + verb (one line)
 *   2. The thing they want to do (glanceable)
 *   3. The consequence (if YES / if NO)
 *   4. Three actions max: Approve · Refuse · Why?
 *
 * Optional: a 30-second undo countdown after a decision.
 */

export type ApprovalDecision = "approve" | "refuse";

export interface ApprovalCardProps {
  /** Worker initiating the request. */
  worker: { name: string; avatarUrl?: string };
  /** "wants to send this listing", "wants to counter-offer at $90", etc. Past-tense-after-decision handled by the host. */
  verb: string;
  /** The thing — single short paragraph, glanceable in <3s. */
  detail: string;
  /** What happens on YES. One sentence. */
  consequenceIfYes: string;
  /** What happens on NO. One sentence. */
  consequenceIfNo: string;
  /** Why am I being asked? One sentence; expanded on demand. */
  reason?: string;
  /** Channel the request arrived on; surfaced as small chrome. */
  channel?: "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp";
  /** Called with the user's decision. Host is responsible for idempotency + audit. */
  onDecide: (decision: ApprovalDecision) => void | Promise<void>;
  /** Called when the user undoes a decision inside the window. */
  onUndo?: () => void | Promise<void>;
  /** Undo window in milliseconds; default 30s. Set 0 to disable. */
  undoWindowMs?: number;
  /** Reflects host-side state; once non-null the card is in post-decision mode. */
  decision?: ApprovalDecision | null;
  className?: string;
}

export function ApprovalCard({
  worker,
  verb,
  detail,
  consequenceIfYes,
  consequenceIfNo,
  reason,
  channel,
  onDecide,
  onUndo,
  undoWindowMs = 30_000,
  decision: controlledDecision,
  className,
}: ApprovalCardProps) {
  const [showReason, setShowReason] = useState(false);
  const [pendingDecision, setPendingDecision] = useState<ApprovalDecision | null>(null);
  const decision = controlledDecision ?? pendingDecision;
  const [undoRemaining, setUndoRemaining] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (decision == null || undoWindowMs <= 0) {
      setUndoRemaining(null);
      return;
    }
    const start = Date.now();
    setUndoRemaining(undoWindowMs);
    const tick = setInterval(() => {
      const remaining = Math.max(0, undoWindowMs - (Date.now() - start));
      setUndoRemaining(remaining);
      if (remaining <= 0) clearInterval(tick);
    }, 200);
    return () => clearInterval(tick);
  }, [decision, undoWindowMs]);

  async function handle(d: ApprovalDecision) {
    if (busy || decision != null) return;
    setBusy(true);
    try {
      setPendingDecision(d);
      await onDecide(d);
    } finally {
      setBusy(false);
    }
  }

  async function handleUndo() {
    if (!onUndo) return;
    setBusy(true);
    try {
      await onUndo();
      setPendingDecision(null);
      setUndoRemaining(null);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card className={cn("max-w-md w-full", className)} role="region" aria-label={`Approval request from ${worker.name}`}>
      <div className="flex items-center gap-3 px-5 pt-5">
        <Avatar alt={worker.name} src={worker.avatarUrl} size="sm" status="waiting" />
        <div className="flex-1 min-w-0">
          <div className="text-sm">
            <span className="font-semibold text-[var(--foreground)]">{worker.name}</span>{" "}
            <span className="text-[var(--muted-foreground)]">{verb}</span>
          </div>
          {channel ? (
            <div className="text-[11px] text-[var(--muted-foreground)] mt-0.5 capitalize">
              via {channel}
            </div>
          ) : null}
        </div>
      </div>

      <div className="px-5 py-4 text-sm leading-relaxed text-[var(--foreground)]">{detail}</div>

      <div className="mx-5 mb-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)] px-4 py-3 text-xs leading-relaxed">
        <div className="text-[var(--foreground)]">
          <span className="font-medium">If YES → </span>
          <span className="text-[var(--muted-foreground)]">{consequenceIfYes}</span>
        </div>
        <div className="mt-1 text-[var(--foreground)]">
          <span className="font-medium">If NO → </span>
          <span className="text-[var(--muted-foreground)]">{consequenceIfNo}</span>
        </div>
      </div>

      {decision == null ? (
        <div className="flex items-center gap-2 px-5 pb-5">
          <Button
            variant="primary"
            size="md"
            className="flex-1"
            onClick={() => handle("approve")}
            loading={busy && pendingDecision === "approve"}
            disabled={busy}
            data-testid="approval-approve"
          >
            Approve
          </Button>
          <Button
            variant="secondary"
            size="md"
            className="flex-1"
            onClick={() => handle("refuse")}
            loading={busy && pendingDecision === "refuse"}
            disabled={busy}
            data-testid="approval-refuse"
          >
            Refuse
          </Button>
          {reason ? (
            <Button
              variant="ghost"
              size="md"
              onClick={() => setShowReason((v) => !v)}
              aria-expanded={showReason}
              aria-controls="approval-why"
              data-testid="approval-why"
            >
              Why?
              <ChevronDown
                className={cn(
                  "transition-transform duration-[var(--duration-fast)]",
                  showReason && "rotate-180",
                )}
              />
            </Button>
          ) : null}
        </div>
      ) : (
        <PostDecisionBar
          decision={decision}
          undoRemaining={undoRemaining}
          undoWindowMs={undoWindowMs}
          onUndo={onUndo ? handleUndo : undefined}
          busy={busy}
        />
      )}

      {reason && showReason ? (
        <div
          id="approval-why"
          className="border-t border-[var(--border)] px-5 py-3 text-xs text-[var(--muted-foreground)] leading-relaxed"
        >
          {reason}
        </div>
      ) : null}
    </Card>
  );
}

function PostDecisionBar({
  decision,
  undoRemaining,
  undoWindowMs,
  onUndo,
  busy,
}: {
  decision: ApprovalDecision;
  undoRemaining: number | null;
  undoWindowMs: number;
  onUndo?: () => void;
  busy: boolean;
}) {
  const seconds = undoRemaining == null ? 0 : Math.ceil(undoRemaining / 1000);
  const undoActive = onUndo != null && undoWindowMs > 0 && undoRemaining != null && undoRemaining > 0;
  const verb = decision === "approve" ? "Approved" : "Refused";
  const tone =
    decision === "approve"
      ? "text-[var(--success)] border-[var(--success)]/30 bg-[var(--success)]/10"
      : "text-[var(--muted-foreground)] border-[var(--border)] bg-[var(--muted)]";

  return (
    <div className={cn("mx-5 mb-5 flex items-center gap-3 rounded-[var(--radius-md)] border px-4 py-3", tone)}>
      <span className="text-sm font-medium flex-1">{verb}</span>
      {undoActive ? (
        <>
          <span className="inline-flex items-center gap-1 text-xs text-current/80" aria-live="polite">
            <Clock className="size-3.5" />
            {seconds}s
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={onUndo}
            disabled={busy}
            data-testid="approval-undo"
          >
            Undo
          </Button>
        </>
      ) : null}
    </div>
  );
}
