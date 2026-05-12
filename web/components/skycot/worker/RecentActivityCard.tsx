import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { Card, CardBody, CardFooter, CardHeader, CardTitle } from "@/components/ui/Card";
import type { WorkerActivityRow } from "@/lib/api/worker-overview";

/**
 * Last 5 actions for the worker. Each row formatted
 * `{time} · {verb} · {status}` and routes to `/audit/action/<id>`.
 * The "See all in Activity →" link routes to the Activity tab
 * (E4-c).
 *
 * Inlines the row layout — E5-a's shared `ActivityRow` primitive
 * is the long-term home; this component will refactor to consume
 * that once it lands.
 */

export interface RecentActivityCardProps {
  workerId: string;
  rows: WorkerActivityRow[];
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

export function RecentActivityCard({
  workerId,
  rows,
  error,
  onRetry,
  className,
}: RecentActivityCardProps) {
  return (
    <Card data-testid="card-recent-activity" className={className}>
      <CardHeader>
        <CardTitle>Recent activity</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <RecentActivityErrorState message={error} onRetry={onRetry} />
        ) : rows.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            No activity yet — she&apos;ll show up here once she starts working.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {rows.slice(0, 5).map((row) => (
              <ActivityListRow key={row.id} row={row} />
            ))}
          </ul>
        )}
      </CardBody>
      <CardFooter>
        <Link
          href={`/workers/${workerId}/activity`}
          data-testid="recent-activity-see-all"
          className="inline-flex items-center gap-1 text-sm font-medium text-[var(--primary)] hover:underline underline-offset-4"
        >
          See all in Activity
          <ChevronRight className="size-3.5" aria-hidden />
        </Link>
      </CardFooter>
    </Card>
  );
}

function ActivityListRow({ row }: { row: WorkerActivityRow }) {
  return (
    <li data-testid={`activity-row-${row.id}`} className="py-2">
      <Link
        href={row.href}
        className="block rounded-[var(--radius-sm)] -mx-1 px-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] hover:bg-[var(--accent)]/40"
      >
        <span className="grid grid-cols-[auto_1fr_auto] items-baseline gap-x-3 text-sm">
          <time
            dateTime={row.at}
            className="text-xs tabular-nums text-[var(--muted-foreground)]"
          >
            {formatTime(row.at)}
          </time>
          <span className="truncate text-[var(--foreground)]">{row.verb}</span>
          <span
            data-testid={`activity-status-${row.id}`}
            className="shrink-0 rounded-full bg-[var(--muted)] px-2 py-0.5 text-xs text-[var(--muted-foreground)]"
          >
            {row.status}
          </span>
        </span>
      </Link>
    </li>
  );
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function RecentActivityErrorState({
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

export function RecentActivityCardSkeleton() {
  return (
    <Card data-testid="card-recent-activity-skeleton">
      <CardHeader>
        <div className="h-4 w-32 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {[0, 1, 2, 3, 4].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="h-3 w-10 rounded bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-4 w-20 rounded-full bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
