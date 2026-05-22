"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { Check, Copy, Download, Play } from "lucide-react";
import { Avatar } from "@/components/ui/Avatar";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { RedactedText } from "@/components/skycot/RedactedText";
import { WorkflowRollbackDiff } from "@/components/skycot/audit/WorkflowRollbackDiff";
import { parseRollbackPayload } from "@/lib/diff/workflow-version-diff";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import type { AuditDetail } from "@/lib/types/audit-detail";
import {
  DECIDER_ROLE_LABELS,
  type AuditStatus,
  type DeciderRole,
} from "@/lib/types/audit";

/**
 * Summary hero card for the audit drill-in surface. Renders worker
 * avatar + verb + approver/channel/timestamp + monospace action id
 * with copy button + Replay + Export buttons + "View related
 * actions" link.
 *
 * PII redaction is applied to the verb via `<RedactedText>` — the
 * page-level surface resolves the redaction policy once and threads
 * it down so the hero stays presentation-only.
 */

export interface ActionSummaryHeroProps {
  detail: AuditDetail;
  policy: RedactionPolicy;
  onReplay: () => void;
  onExport: () => void;
}

export function ActionSummaryHero({
  detail,
  policy,
  onReplay,
  onExport,
}: ActionSummaryHeroProps) {
  const tone = STATUS_TONE[detail.status];
  const initials = workerInitials(detail.worker.name);
  const sameHourHref = relatedActionsHref(detail);
  const rollbackPayload = useMemo(() => parseRollbackPayload(detail), [detail]);

  return (
    <section
      data-testid="audit-summary-hero"
      data-action-id={detail.id}
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-3 min-w-0 flex-1">
          <Avatar
            size="md"
            alt={detail.worker.name}
            src={detail.worker.avatarUrl}
            fallback={initials}
          />
          <div className="min-w-0 space-y-1">
            <p
              className="text-sm text-[var(--muted-foreground)]"
              data-testid="audit-hero-worker"
            >
              {detail.worker.name}
            </p>
            <h1
              className="text-base font-medium text-[var(--foreground)]"
              data-testid="audit-hero-verb"
            >
              <RedactedText policy={policy}>{detail.verb}</RedactedText>
            </h1>
            <HeroMeta detail={detail} />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <StatusPill tone={tone} size="md">
            {STATUS_LABEL[detail.status]}
          </StatusPill>
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--border)] pt-3">
        <ActionIdCopy actionId={detail.id} />

        <div className="flex items-center gap-2">
          <button
            type="button"
            data-testid="audit-hero-replay"
            onClick={onReplay}
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            <Play aria-hidden className="size-4" /> Replay
          </button>
          <button
            type="button"
            data-testid="audit-hero-export"
            onClick={onExport}
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            <Download aria-hidden className="size-4" /> Export
          </button>
        </div>
      </div>

      {rollbackPayload ? (
        <WorkflowRollbackDiff
          payload={rollbackPayload}
          approverName={detail.approver?.name}
        />
      ) : null}

      <div className="text-sm">
        <Link
          href={sameHourHref}
          data-testid="audit-hero-related"
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          View related actions →
        </Link>
      </div>
    </section>
  );
}

function HeroMeta({ detail }: { detail: AuditDetail }) {
  const tail: string[] = [];
  if (detail.channel) tail.push(`Channel: ${detail.channel}`);
  tail.push(formatTimestamp(detail.timestamp));
  return (
    <p
      data-testid="audit-hero-meta"
      className="text-xs text-[var(--muted-foreground)]"
    >
      {detail.approver ? (
        <>
          <span>Approver: </span>
          <ApproverWithRole
            name={detail.approver.name}
            role={detail.decider_role}
          />
          {tail.length > 0 ? <span> · </span> : null}
        </>
      ) : null}
      {tail.join(" · ")}
    </p>
  );
}

/** Render "<Role> <name>" (e.g. "Admin Mark") when `decider_role` is
 *  present; bare name otherwise. The role label is plain inline text
 *  per the audit page's restrained register — no gradient, no
 *  Audiowide, no badge chrome — just a slightly weighted muted span
 *  that the eye reads as a prefix on the name. */
function ApproverWithRole({
  name,
  role,
}: {
  name: string;
  role?: DeciderRole;
}) {
  if (!role) {
    return <span data-testid="audit-hero-approver">{name}</span>;
  }
  return (
    <span data-testid="audit-hero-approver">
      <span
        data-testid="audit-hero-approver-role"
        data-role={role}
        className="font-medium text-[var(--foreground)]"
      >
        {DECIDER_ROLE_LABELS[role]}
      </span>
      <span> {name}</span>
    </span>
  );
}

function ActionIdCopy({ actionId }: { actionId: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      data-testid="audit-hero-copy-id"
      aria-label={`Copy action id ${actionId}`}
      onClick={() => {
        void copyToClipboard(actionId).then((ok) => {
          if (!ok) return;
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="inline-flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-1 font-mono text-[12px] text-[var(--foreground)] hover:bg-[var(--muted)]/70"
    >
      <span>{actionId}</span>
      {copied ? <Check className="size-3 text-[var(--success)]" /> : <Copy className="size-3" />}
      <span className="sr-only">{copied ? "Copied" : "Copy"}</span>
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

function workerInitials(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length === 0) return "??";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

const STATUS_TONE: Record<AuditStatus, StatusPillProps["tone"]> = {
  succeeded: "success",
  failed: "error",
  refused: "warning",
  pending: "info",
  autonomous: "success",
};

const STATUS_LABEL: Record<AuditStatus, string> = {
  succeeded: "Succeeded",
  failed: "Failed",
  refused: "Refused",
  pending: "Pending",
  autonomous: "Autonomous",
};

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: false,
  });
}

function relatedActionsHref(detail: AuditDetail): string {
  const t = Date.parse(detail.timestamp);
  if (Number.isNaN(t)) return `/audit?worker=${encodeURIComponent(detail.worker.id)}`;
  const floor = new Date(t);
  floor.setMinutes(0, 0, 0);
  const ceil = new Date(floor.getTime() + 60 * 60_000);
  const params = new URLSearchParams({
    worker: detail.worker.id,
    from: floor.toISOString(),
    to: ceil.toISOString(),
  });
  return `/audit?${params.toString()}`;
}
