"use client";

import Link from "next/link";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { cn } from "@/lib/utils";
import type { RecentDecision } from "@/lib/api/worker-memory";

/**
 * Material decisions made in the last 14 days. v1 surfaces 5-10 per
 * worker; the seed dataset stays under that ceiling. Each row links
 * to the audit drill-in for the decision record.
 */

export interface RecentDecisionsCardProps {
  decisions: RecentDecision[];
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

const MAX_ROWS = 10;

export function RecentDecisionsCard({
  decisions,
  error,
  onRetry,
  className,
}: RecentDecisionsCardProps) {
  const visible = decisions.slice(0, MAX_ROWS);

  return (
    <Card data-testid="card-recent-decisions" className={className}>
      <CardHeader>
        <CardTitle>Recent decisions</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <DecisionsErrorState message={error} onRetry={onRetry} />
        ) : visible.length === 0 ? (
          <p
            data-testid="decisions-empty"
            className="text-sm text-[var(--muted-foreground)]"
          >
            No material decisions in the last 14 days.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {visible.map((d) => (
              <DecisionRow key={d.decisionId} decision={d} />
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

function DecisionRow({ decision }: { decision: RecentDecision }) {
  return (
    <li data-testid={`decision-row-${decision.decisionId}`} className="py-2">
      <Link
        href={decision.href}
        className="block rounded-[var(--radius-sm)] -mx-1 px-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] hover:bg-[var(--accent)]/40"
      >
        <div className="grid grid-cols-[1fr_auto_auto] items-baseline gap-x-3 text-sm">
          <span className="truncate text-[var(--foreground)]">{decision.label}</span>
          <span
            data-testid={`decision-outcome-${decision.decisionId}`}
            className={cn(
              "shrink-0 rounded-full px-2 py-0.5 text-xs",
              decision.outcome === "approved"
                ? "bg-[var(--success)]/15 text-[var(--success-foreground)]"
                : "bg-[var(--destructive)]/15 text-[var(--destructive-foreground)]",
            )}
          >
            {decision.outcome}
          </span>
          <time
            dateTime={decision.decidedAt}
            className="shrink-0 text-xs tabular-nums text-[var(--muted-foreground)]"
          >
            {formatRelative(decision.decidedAt)}
          </time>
        </div>
      </Link>
    </li>
  );
}

function formatRelative(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  const deltaMs = Math.max(0, Date.now() - t);
  const minutes = Math.round(deltaMs / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

function DecisionsErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}

export function RecentDecisionsCardSkeleton() {
  return (
    <Card data-testid="card-recent-decisions-skeleton">
      <CardHeader>
        <div className="h-4 w-36 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {[0, 1, 2].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-3 w-16 rounded-full bg-[var(--muted)]" />
            <div className="h-3 w-10 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
