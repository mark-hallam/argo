"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight, Check, Copy } from "lucide-react";
import { Avatar } from "@/components/ui/Avatar";
import { ChannelBadge } from "@/components/ui/ChannelBadge";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";
import { useToast } from "@/lib/ui/use-toast";
import {
  formatHashForDisplay,
  redactInlineSentinels,
  relativeTime,
} from "@/components/skycot/ActivityRow";
import type {
  WorkerActivity,
  WorkerActivityHashKind,
  WorkerActivityStatus,
} from "@/lib/api/worker-activity";
import type { ChannelKind } from "@/lib/types/channels";

/**
 * Table-row variant of the audit feed primitive. Renders as `<tr>` +
 * `<td>` cells so screen readers can announce "table, N rows, columns:
 * time, worker, verb, id, status". The shared `<ActivityRow>` is
 * locked at `<Link>`-outer semantics (consumed by cohort drawer +
 * worker activity), so the audit feed has its own table-shaped
 * primitive here.
 *
 * Whole-row navigation is a single spanning overlay `<a>` rendered in
 * the first cell (the `<tr>` is the positioning context). The row
 * itself is NOT a clickable/role-less element — keyboard users tab to
 * the real overlay anchor, which a native `<a href>` activates on
 * Enter, and right-click "open in new tab" works. The inner action-id,
 * copy, hash, and delegated-by controls are lifted above the overlay so
 * they remain independently operable.
 */

type DelegatedBy = { workerId: string; workerName: string };
type WorkerActivityWithDelegation = WorkerActivity & {
  delegatedBy?: DelegatedBy;
  delegated_by?: { worker_id?: string; worker_name?: string };
};

export interface AuditTableRowProps {
  activity: WorkerActivityWithDelegation;
  status?: string;
  actionType?: string;
  workerId?: string;
  /** Channel the action arrived on; renders as a compact icon-only
   *  badge in the verb cell so /audit shows the multi-channel surface
   *  at a glance (Claim 6). */
  channel?: ChannelKind;
}

export function AuditTableRow({
  activity,
  status,
  actionType,
  workerId,
  channel,
}: AuditTableRowProps) {
  const href = `/audit/action/${encodeURIComponent(activity.id)}`;
  const relative = relativeTime(activity.timestamp);
  const tone = STATUS_TONE[activity.status];
  const delegatedBy = normalizedDelegatedBy(activity);

  return (
    <tr
      data-testid={`audit-row-${activity.id}`}
      data-status={status ?? activity.status}
      data-action-type={actionType}
      data-worker-id={workerId}
      className={cn(
        "group relative border-b border-[var(--border)] bg-[var(--card)] text-sm",
        "transition-colors duration-[var(--duration-fast)]",
        "hover:border-[var(--primary)]/40 hover:bg-[var(--accent)]/30",
        "focus-within:border-[var(--primary)]/40 focus-within:bg-[var(--accent)]/30",
      )}
    >
      <td className="relative whitespace-nowrap px-3 py-2.5 align-middle text-xs text-[var(--muted-foreground)]">
        {/*
          Single spanning overlay anchor: positioned against the <tr>
          (its `relative` context) so the whole row is one real link.
          Replaces the prior tabIndex+onClick+onKeyDown <tr>, which was a
          role-less clickable container nesting interactive cells. Inner
          controls (action-id link, copy, hash, delegated-by) sit above it.
        */}
        <Link
          href={href}
          prefetch={false}
          data-testid={`audit-row-overlay-${activity.id}`}
          aria-label={`Open audit detail for ${activity.workerName} — ${activity.verb}`}
          className="absolute inset-0 z-0 rounded-[var(--radius-sm)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-inset"
        />
        <time
          dateTime={activity.timestamp}
          title={activity.timestamp}
          data-testid={`activity-row-time-${activity.id}`}
        >
          {relative}
        </time>
      </td>

      <td className="px-3 py-2.5 align-middle whitespace-nowrap">
        <Avatar
          size="xs"
          alt={activity.workerName}
          fallback={activity.workerInitials}
        />
      </td>

      <td className="px-3 py-2.5 align-middle">
        <p
          className="flex items-center gap-1.5 truncate text-[var(--foreground)]"
          data-testid={`activity-row-verb-${activity.id}`}
        >
          {channel ? (
            <ChannelBadge
              kind={channel}
              compact
              data-testid={`audit-row-channel-${activity.id}`}
            />
          ) : null}
          <span className="truncate">
            {redactInlineSentinels(activity.verb)}
          </span>
        </p>
      </td>

      <td className="px-3 py-2.5 align-middle">
        {/* Lifted above the spanning overlay anchor so the action-id
            link, copy, hash, and delegated-by controls stay clickable. */}
        <div className="relative z-10 flex min-w-0 flex-wrap items-center gap-2">
          <ActionIdLink actionId={activity.id} href={href} />
          {delegatedBy ? (
            <DelegatedByChip actionId={activity.id} delegatedBy={delegatedBy} />
          ) : null}
          {activity.hash ? (
            <HashCell
              hash={activity.hash}
              kind={activity.hashKind ?? "self"}
              actionId={activity.id}
            />
          ) : null}
        </div>
      </td>

      <td className="px-3 py-2.5 align-middle whitespace-nowrap">
        <div className="flex items-center gap-2">
          <StatusPill tone={tone} size="sm">
            {STATUS_LABEL[activity.status]}
          </StatusPill>
          {activity.costUsd && activity.costUsd > 0 ? (
            <CostChip
              costUsd={activity.costUsd}
              breakdown={activity.costBreakdown}
            />
          ) : null}
          <ArrowRight
            aria-hidden
            className="size-4 text-[var(--muted-foreground)] transition-transform group-hover:translate-x-0.5 group-hover:text-[var(--foreground)]"
          />
        </div>
      </td>
    </tr>
  );
}

function normalizedDelegatedBy(
  activity: WorkerActivityWithDelegation,
): DelegatedBy | null {
  if (activity.delegatedBy?.workerId && activity.delegatedBy.workerName) {
    return activity.delegatedBy;
  }
  if (
    activity.delegated_by?.worker_id &&
    activity.delegated_by.worker_name
  ) {
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

function ActionIdLink({ actionId, href }: { actionId: string; href: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <span className="inline-flex items-center gap-1">
      <Link
        href={href}
        prefetch={false}
        data-testid={`activity-row-link-${actionId}`}
        onClick={(e) => e.stopPropagation()}
        className="font-mono text-[11px] text-[var(--muted-foreground)] hover:text-[var(--foreground)] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--ring)] rounded-sm"
      >
        {actionId}
      </Link>
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
        className="group/copy text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
      >
        {copied ? (
          <Check className="size-3" />
        ) : (
          // Reveal on row hover OR when the button itself is keyboard-focused
          // (was opacity-0 + group-hover only — invisible to keyboard users).
          <Copy className="size-3 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible/copy:opacity-100" />
        )}
      </button>
    </span>
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
      className="group/hash inline-flex items-center gap-1 font-mono text-[11px] tabular-nums text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
    >
      <span>{display}</span>
      {copied ? (
        <Check className="size-3 text-[var(--success-foreground,var(--foreground))]" />
      ) : (
        // Reveal on row hover OR when this hash button is keyboard-focused.
        <Copy className="size-3 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible/hash:opacity-100" />
      )}
    </button>
  );
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
