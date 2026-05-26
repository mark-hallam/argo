"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight, Check, Copy } from "lucide-react";
import { Avatar } from "@/components/ui/Avatar";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";
import { useToast } from "@/lib/ui/use-toast";
import type {
  WorkerActivity,
  WorkerActivityHashKind,
  WorkerActivityStatus,
} from "@/lib/api/worker-activity";

/**
 * Shared row primitive for audit-style feeds. E4-c ships it first;
 * E5-a (cross-worker audit timeline) re-uses it. Click → `/audit/
 * action/<id>` (E5-c). Redaction: brief specifies the E1-e
 * `<RedactedText>` wrapper on `verb`; E1-e is BLOCKED, so this
 * file inlines `redactInlineSentinels()` with byte-identical
 * sentinels (`[EMAIL]`/`[PHONE]`/`[CARD]`) — swap-in is a one-line
 * change when E1-e lands.
 */

export interface ActivityRowProps {
  activity: WorkerActivityWithDelegation;
  /** Hide the worker avatar (E5-a may decide to dedupe by worker). */
  hideWorker?: boolean;
  className?: string;
}

type DelegatedBy = { workerId: string; workerName: string };
type WorkerActivityWithDelegation = WorkerActivity & {
  delegatedBy?: DelegatedBy;
  delegated_by?: { worker_id?: string; worker_name?: string };
};

export function ActivityRow({ activity, hideWorker, className }: ActivityRowProps) {
  const href = `/audit/action/${encodeURIComponent(activity.id)}`;
  const relative = relativeTime(activity.timestamp);
  const tone = STATUS_TONE[activity.status];
  const delegatedBy = normalizedDelegatedBy(activity);

  return (
    <Link
      href={href}
      role="listitem"
      data-testid={`activity-row-${activity.id}`}
      className={cn(
        "group grid grid-cols-[auto_auto_1fr_auto_auto_auto] items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2.5",
        "text-sm transition-colors duration-[var(--duration-fast)]",
        "hover:border-[var(--primary)]/40 hover:bg-[var(--accent)]/30",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--background)]",
        className,
      )}
    >
      <time
        dateTime={activity.timestamp}
        title={activity.timestamp}
        className="whitespace-nowrap text-xs text-[var(--muted-foreground)]"
        data-testid={`activity-row-time-${activity.id}`}
      >
        {relative}
      </time>

      {hideWorker ? (
        <span aria-hidden />
      ) : (
        <Avatar
          size="xs"
          alt={activity.workerName}
          fallback={activity.workerInitials}
        />
      )}

      <div className="min-w-0">
        <p className="truncate text-[var(--foreground)]" data-testid={`activity-row-verb-${activity.id}`}>
          {redactInlineSentinels(activity.verb)}
        </p>
        <div className="mt-0.5 flex min-w-0 flex-wrap items-center gap-2">
          <ActionIdCopy actionId={activity.id} />
          {delegatedBy ? (
            <DelegatedByChip actionId={activity.id} delegatedBy={delegatedBy} />
          ) : null}
        </div>
      </div>

      <div className="flex items-center gap-2">
        <StatusPill tone={tone} size="sm">
          {STATUS_LABEL[activity.status]}
        </StatusPill>
        {activity.costUsd && activity.costUsd > 0 ? (
          <CostChip costUsd={activity.costUsd} breakdown={activity.costBreakdown} />
        ) : null}
      </div>

      {activity.hash ? (
        <HashCell
          hash={activity.hash}
          kind={activity.hashKind ?? "self"}
          actionId={activity.id}
        />
      ) : (
        <span aria-hidden />
      )}

      <ArrowRight
        aria-hidden
        className="size-4 text-[var(--muted-foreground)] transition-transform group-hover:translate-x-0.5 group-hover:text-[var(--foreground)]"
      />
    </Link>
  );
}

function normalizedDelegatedBy(activity: WorkerActivityWithDelegation): DelegatedBy | null {
  if (activity.delegatedBy?.workerId && activity.delegatedBy.workerName) {
    return activity.delegatedBy;
  }
  if (activity.delegated_by?.worker_id && activity.delegated_by.worker_name) {
    return {
      workerId: activity.delegated_by.worker_id,
      workerName: activity.delegated_by.worker_name,
    };
  }
  return null;
}

const STATUS_TONE: Record<WorkerActivityStatus, StatusPillProps["tone"]> = {
  succeeded: "success",
  failed: "error",
  refused: "warning",
  pending: "info",
};

const STATUS_LABEL: Record<WorkerActivityStatus, string> = {
  succeeded: "Succeeded",
  failed: "Failed",
  refused: "Refused",
  pending: "Pending",
};

function ActionIdCopy({ actionId }: { actionId: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      data-testid={`activity-row-copy-${actionId}`}
      aria-label={`Copy action id ${actionId}`}
      title={copied ? "Copied" : "Copy action id"}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        void copyToClipboard(actionId).then((ok) => {
          if (!ok) return;
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="mt-0.5 inline-flex items-center gap-1 font-mono text-[11px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
    >
      <span>{actionId}</span>
      {copied ? <Check className="size-3" /> : <Copy className="size-3 opacity-0 group-hover:opacity-100" />}
    </button>
  );
}

function DelegatedByChip({
  actionId,
  delegatedBy,
}: {
  actionId: string;
  delegatedBy: DelegatedBy;
}) {
  const href = `/workers/${encodeURIComponent(delegatedBy.workerId)}/overview`;
  return (
    <button
      type="button"
      data-testid={`activity-row-delegated-by-${actionId}`}
      title={`Delegated by ${delegatedBy.workerName}`}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        window.location.assign(href);
      }}
      className="inline-flex items-center rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)]/40 px-1.5 py-0.5 text-[11px] font-medium text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
    >
      via {delegatedBy.workerName}
    </button>
  );
}

interface CostChipProps {
  costUsd: number;
  breakdown?: WorkerActivity["costBreakdown"];
}

function CostChip({ costUsd, breakdown }: CostChipProps) {
  const tip = breakdown
    ? `${breakdown.tokensIn} in / ${breakdown.tokensOut} out — ${breakdown.model}`
    : undefined;
  return (
    <span
      data-testid="activity-row-cost"
      title={tip}
      className="inline-flex items-center rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)] px-2 py-0.5 text-[11px] font-medium tabular-nums text-[var(--muted-foreground)]"
    >
      ${costUsd.toFixed(3)}
    </span>
  );
}

interface HashCellProps {
  hash: string;
  kind: WorkerActivityHashKind;
  actionId: string;
}

/** Per-row chain-hash cell — surfaces the "forensic-grade audit chain
 *  on every action" claim on the feed (Etherscan parity). Renders a
 *  truncated `0xabc1…2345` form by default; hover tooltip + aria-label
 *  expose the full value to assistive tech. Click copies the FULL hash
 *  to the clipboard and fires a brief Sonner toast. Stops nav so the
 *  click doesn't drill into `/audit/action/<id>`. */
function HashCell({ hash, kind, actionId }: HashCellProps) {
  const [copied, setCopied] = useState(false);
  const { success } = useToast();
  const display = formatHashForDisplay(hash);
  const label =
    kind === "sample"
      ? "Sample row — not part of the production audit chain"
      : "Self-hash";
  const title = copied ? "Copied" : `${label} · click to copy\n${hash}`;
  return (
    <button
      type="button"
      data-testid={`activity-row-hash-${actionId}`}
      data-hash={hash}
      data-hash-kind={kind}
      aria-label={`${label} ${hash}, click to copy`}
      title={title}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        void copyToClipboard(hash).then((ok) => {
          if (!ok) return;
          setCopied(true);
          success(kind === "sample" ? "Sample hash copied" : "Hash copied", {
            description: hash,
          });
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="inline-flex items-center gap-1 font-mono text-[11px] tabular-nums text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
    >
      <span>{display}</span>
      {copied ? (
        <Check className="size-3 text-[var(--success-foreground,var(--foreground))]" />
      ) : (
        <Copy className="size-3 opacity-0 transition-opacity group-hover:opacity-100" />
      )}
    </button>
  );
}

/** Etherscan-style truncation: `0x` prefix + first 4 + ellipsis +
 *  last 4 hex chars. Short hashes (<10 chars) render unchanged so
 *  test fixtures with stub values stay readable. */
export function formatHashForDisplay(hash: string): string {
  const body = hash.startsWith("0x") ? hash.slice(2) : hash;
  if (body.length <= 10) return `0x${body}`;
  return `0x${body.slice(0, 4)}…${body.slice(-4)}`;
}

async function copyToClipboard(value: string): Promise<boolean> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(value);
      return true;
    }
  } catch {
    /* fall through */
  }
  return false;
}

/** Compact relative-time formatter — "5m ago", "3h ago", "2d ago".
 *  Stable across SSR/CSR provided the input timestamp is floored. */
export function relativeTime(iso: string, now: number = Date.now()): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  const seconds = Math.max(0, Math.round((now - t) / 1000));
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.round(days / 30);
  if (months < 12) return `${months}mo ago`;
  const years = Math.round(months / 12);
  return `${years}y ago`;
}

/** Belt-and-braces PII masking; placeholder for E1-e's
 *  `<RedactedText>`. Low-recall by design: phone pattern requires
 *  a leading `+` so invoice/ticket ids don't false-positive. */
export function redactInlineSentinels(text: string): string {
  return text
    .replace(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi, "[EMAIL]")
    .replace(/\b\d{4}[ -]?\d{4}[ -]?\d{4}[ -]?\d{4}\b/g, "[CARD]")
    .replace(/\+\d[\d ()-]{6,}\d/g, "[PHONE]");
}
