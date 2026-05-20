"use client";

import { Check, ChevronDown, Clock, MessageSquare, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { RedactedText } from "@/components/skycot/RedactedText";
import { ConfidenceDonut } from "@/components/skycot/approvals/ConfidenceDonut";
import { DollarAtRisk } from "@/components/skycot/approvals/DollarAtRisk";
import { SeverityPill, type Severity } from "@/components/skycot/approvals/SeverityPill";
import { SLABar } from "@/components/skycot/approvals/SLABar";
import { POLICY_SHOW_ALL, type RedactionPolicy } from "@/lib/redaction/redact";
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
  /**
   * Optional executor-side draft body — PRD text, email body, document
   * preview — surfaced in a default-collapsed `<details>` region below
   * `detail`. Spec: `docs/skycot/ux-research/delegator.md:51-53` ("see
   * exactly what the worker is about to do BEFORE it does it"). Long
   * drafts (>2KB) scroll internally so the action bar stays on-screen.
   */
  draftContent?: string;
  /** Channel the request arrived on; surfaced as small chrome. */
  channel?: "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp";
  /** ISO timestamp of when the request landed. Surfaced as relative time. */
  receivedAt?: string;
  /** Called with the user's decision. Host is responsible for idempotency + audit. */
  onDecide: (decision: ApprovalDecision) => void | Promise<void>;
  /** Called when the user undoes a decision inside the window. */
  onUndo?: () => void | Promise<void>;
  /** Undo window in milliseconds; default 30s. Set 0 to disable. */
  undoWindowMs?: number;
  /** Reflects host-side state; once non-null the card is in post-decision mode. */
  decision?: ApprovalDecision | null;
  /**
   * Client-side PII redaction policy for `detail`, `consequenceIfYes`,
   * `consequenceIfNo`, and `reason`. Defaults to `POLICY_SHOW_ALL`
   * (no redaction) so card-only consumers and the existing test
   * fixtures keep rendering raw prose. The approvals page wires
   * `useRedactionPolicy()` and pushes the resolved policy down.
   * Defense-in-depth — see `lib/redaction/redact.ts`.
   */
  redactionPolicy?: RedactionPolicy;

  /**
   * Three-tier severity classification rendered as a pill at the top
   * of the card. Drives the destructive / warning / muted accent
   * row-tint via the `data-severity` attribute. Omit to hide the pill
   * (Phase A backward-compat — older fixtures don't carry severity).
   */
  severity?: Severity;
  /** Dollar amount at risk if the action runs. Omit / null for non-money actions. */
  dollarAtRisk?: number | null;
  /** ISO 4217 currency code. Defaults to USD. */
  currency?: string;
  /** Worker self-reported confidence in the proposed action, in [0, 1]. */
  confidence?: number;
  /** ISO timestamp when the approval times out. Drives the SLA bar. */
  slaExpiresAt?: string;
  className?: string;
}

const CHANNEL_LABEL: Record<NonNullable<ApprovalCardProps["channel"]>, string> = {
  slack: "Slack",
  imessage: "iMessage",
  sms: "SMS",
  push: "Push",
  web: "Web inbox",
  voice: "Voice",
  whatsapp: "WhatsApp",
};

function toRelative(iso: string): string {
  const diff = Math.round((Date.now() - new Date(iso).getTime()) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.round(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)}h ago`;
  return `${Math.round(diff / 86400)}d ago`;
}

/**
 * Client-only relative-time renderer. Avoids SSR/CSR hydration
 * mismatch on `Date.now()`-derived strings without resorting to
 * suppressHydrationWarning (which doesn't cover attribute values).
 */
function RelativeTime({ iso }: { iso: string }) {
  const [label, setLabel] = useState<string | null>(null);
  useEffect(() => {
    setLabel(toRelative(iso));
    const tick = setInterval(() => setLabel(toRelative(iso)), 30_000);
    return () => clearInterval(tick);
  }, [iso]);
  if (label == null) return null;
  return (
    <time dateTime={iso} title={iso}>
      {label}
    </time>
  );
}

export function ApprovalCard({
  worker,
  verb,
  detail,
  consequenceIfYes,
  consequenceIfNo,
  reason,
  draftContent,
  channel,
  receivedAt,
  onDecide,
  onUndo,
  undoWindowMs = 30_000,
  decision: controlledDecision,
  redactionPolicy = POLICY_SHOW_ALL,
  severity,
  dollarAtRisk,
  currency,
  confidence,
  slaExpiresAt,
  className,
}: ApprovalCardProps) {
  const [showReason, setShowReason] = useState(false);
  const [draftOpen, setDraftOpen] = useState(false);
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

  const accentCls =
    severity === "p0"
      ? "border-l-4 border-l-[var(--destructive)]"
      : severity === "p1"
        ? "border-l-4 border-l-[var(--warning)]"
        : "";

  const showMetrics =
    slaExpiresAt != null ||
    (dollarAtRisk != null && dollarAtRisk !== 0) ||
    confidence != null;

  return (
    <Card
      className={cn(
        "w-full overflow-hidden shadow-sm hover:shadow-md transition-shadow duration-[var(--duration-base)]",
        accentCls,
        className,
      )}
      role="region"
      aria-label={`Approval request from ${worker.name}`}
      data-severity={severity}
    >
      <div className="flex items-start gap-3 px-5 pt-5">
        <Avatar alt={worker.name} src={worker.avatarUrl} size="md" status="waiting" />
        <div className="flex-1 min-w-0 pt-0.5">
          <div className="flex items-start justify-between gap-2">
            <div className="text-[15px] leading-snug min-w-0">
              <span className="font-semibold text-[var(--foreground)]">{worker.name}</span>{" "}
              <span className="text-[var(--muted-foreground)]">{verb}</span>
            </div>
            {severity ? <SeverityPill severity={severity} className="shrink-0" /> : null}
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] text-[var(--muted-foreground)]">
            {channel ? (
              <span className="inline-flex items-center gap-1">
                <MessageSquare className="size-3" />
                {CHANNEL_LABEL[channel]}
              </span>
            ) : null}
            {channel && receivedAt ? <span aria-hidden>·</span> : null}
            {receivedAt ? <RelativeTime iso={receivedAt} /> : null}
          </div>
        </div>
      </div>

      <div className="px-5 py-4 text-[14px] leading-relaxed text-[var(--foreground)]">
        <RedactedText policy={redactionPolicy}>{detail}</RedactedText>
      </div>

      {draftContent ? (
        <details
          className="mx-5 mb-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 text-[13px] leading-relaxed"
          open={draftOpen}
          onToggle={(e) => setDraftOpen((e.currentTarget as HTMLDetailsElement).open)}
          data-testid="approval-draft"
        >
          <summary
            className="cursor-pointer select-none px-4 py-2 font-medium text-[var(--foreground)] flex items-center gap-2"
            data-testid="approval-draft-toggle"
          >
            <ChevronDown
              className={cn(
                "size-3.5 shrink-0 transition-transform duration-[var(--duration-fast)]",
                draftOpen && "rotate-180",
              )}
              aria-hidden
            />
            <span>{draftOpen ? "Hide draft" : "Show draft"}</span>
          </summary>
          <div
            className="px-4 pb-3 pt-1 max-h-[30vh] overflow-y-auto whitespace-pre-wrap text-[var(--muted-foreground)]"
            data-testid="approval-draft-body"
          >
            <RedactedText policy={redactionPolicy}>{draftContent}</RedactedText>
          </div>
        </details>
      ) : null}

      <div className="mx-5 mb-4 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/40 px-4 py-3 text-[13px] leading-relaxed space-y-1.5">
        <div className="flex gap-2.5">
          <span className="font-semibold text-[var(--success)] shrink-0 tabular-nums">If YES →</span>
          <span className="text-[var(--muted-foreground)]">
            <RedactedText policy={redactionPolicy}>{consequenceIfYes}</RedactedText>
          </span>
        </div>
        <div className="flex gap-2.5">
          <span className="font-semibold text-[var(--muted-foreground)] shrink-0 tabular-nums">If NO →</span>
          <span className="text-[var(--muted-foreground)]">
            <RedactedText policy={redactionPolicy}>{consequenceIfNo}</RedactedText>
          </span>
        </div>
      </div>

      {showMetrics ? (
        <div
          className="mx-5 mb-4 flex flex-wrap items-center gap-x-5 gap-y-2"
          data-testid="approval-metrics"
        >
          {slaExpiresAt ? (
            <div className="min-w-[140px] flex-1">
              <SLABar expiresAt={slaExpiresAt} />
            </div>
          ) : null}
          {dollarAtRisk != null && dollarAtRisk !== 0 ? (
            <div className="flex items-baseline gap-2">
              <span className="text-[10px] uppercase tracking-[0.1em] text-[var(--muted-foreground)] font-semibold">
                $ at risk
              </span>
              <DollarAtRisk amount={dollarAtRisk} currency={currency ?? "USD"} />
            </div>
          ) : null}
          {confidence != null ? (
            <ConfidenceDonut value={confidence} animate />
          ) : null}
        </div>
      ) : null}

      {decision == null ? (
        <div className="flex items-center gap-2 px-5 pb-5">
          <Button
            variant="primary"
            size="md"
            className="flex-1 shadow-sm"
            onClick={() => handle("approve")}
            loading={busy && pendingDecision === "approve"}
            disabled={busy}
            data-testid="approval-approve"
          >
            <Check />
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
            <X />
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
          className="border-t border-[var(--border)] px-5 py-3 text-xs text-[var(--muted-foreground)] leading-relaxed bg-[var(--muted)]/30"
        >
          <RedactedText policy={redactionPolicy}>{reason}</RedactedText>
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
