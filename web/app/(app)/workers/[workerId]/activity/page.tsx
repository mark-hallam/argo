"use client";

import { useCallback, useMemo } from "react";
import Link from "next/link";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { ActivityFilterBar } from "@/components/skycot/ActivityFilterBar";
import { ActivityRow } from "@/components/skycot/ActivityRow";
import {
  DEFAULT_RANGE,
  WORKER_ACTIVITY_ACTION_TYPES,
  WORKER_ACTIVITY_RANGES,
  WORKER_ACTIVITY_STATUSES,
  type WorkerActivityActionType,
  type WorkerActivityFilters,
  type WorkerActivityRange,
  type WorkerActivityStatus,
} from "@/lib/api/worker-activity";
import { useWorkerActivity } from "@/lib/hooks/use-worker-activity";
import { useWorker } from "@/lib/hooks/use-worker";

/**
 * Per-worker Activity tab — `/workers/<id>/activity`. Reads the
 * worker from the same TanStack cache the parent layout primed
 * (`useWorker(workerId)` shares the key), then renders a filtered
 * paginated feed below the layout's hero + tab strip.
 *
 * URL is the source of truth for filters: `?range=…&type=…&status=…`.
 * Filter changes route back into this page with the next query
 * string, which re-derives the `WorkerActivityFilters` object and
 * re-keys the infinite-query.
 *
 * Sub-route precedence: a static segment (`activity/`) wins over the
 * sibling dynamic `[tab]/page.tsx` placeholder, so this page
 * replaces the E4-a "coming in E4-c" stub without touching that
 * file (HARD RULE 5 respected — `[tab]/page.tsx` is read-only).
 */

export default function WorkerActivityPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const router = useRouter();
  const searchParams = useSearchParams();
  const workerQuery = useWorker(workerId);
  const filters = parseFilters(searchParams);
  const activity = useWorkerActivity(workerId, filters);

  const updateFilters = useCallback(
    (next: WorkerActivityFilters) => {
      const qs = serializeFilters(next);
      router.replace(qs ? `?${qs}` : "?", { scroll: false });
    },
    [router],
  );

  const clearFilters = useCallback(() => {
    router.replace("?", { scroll: false });
  }, [router]);

  const rows = useMemo(
    () => activity.data?.pages.flatMap((p) => p.rows) ?? [],
    [activity.data],
  );

  const totalPages = activity.data?.pages[0]?.totalPages;
  const currentPageCount = activity.data?.pages.length ?? 0;
  const isPristine = filters.range === DEFAULT_RANGE && !filters.actionType && !filters.status;
  const workerName = workerQuery.data?.name ?? "This worker";
  const auditTimelineHref = `/audit?worker=${encodeURIComponent(workerId)}&range=${filters.range}`;

  return (
    <div
      role="tabpanel"
      aria-label="Activity tab"
      data-testid="worker-tab-panel-activity"
      className="space-y-4"
    >
      <ActivityFilterBar
        filters={filters}
        onChange={updateFilters}
        disabled={activity.isFetching && !activity.isFetchingNextPage}
      />

      {activity.isPending ? (
        <LoadingState />
      ) : activity.isError ? (
        <ErrorState
          message={activity.error?.message ?? "Couldn't load activity"}
          onRetry={() => activity.refetch()}
        />
      ) : rows.length === 0 ? (
        <EmptyState
          isPristine={isPristine}
          workerName={workerName}
          onClearFilters={clearFilters}
        />
      ) : (
        <div data-testid="activity-feed" role="list" className="space-y-2">
          {rows.map((row) => (
            <ActivityRow key={row.id} activity={row} />
          ))}
        </div>
      )}

      {activity.hasNextPage ? (
        <div className="flex justify-center pt-2">
          <button
            type="button"
            data-testid="activity-load-more"
            disabled={activity.isFetchingNextPage}
            onClick={() => activity.fetchNextPage()}
            className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] disabled:cursor-not-allowed disabled:opacity-60"
          >
            {activity.isFetchingNextPage
              ? "Loading…"
              : `Load more — page ${currentPageCount + 1}${totalPages ? ` of ${totalPages}` : ""}`}
          </button>
        </div>
      ) : null}

      <div className="flex justify-end pt-2">
        <Link
          href={auditTimelineHref}
          data-testid="activity-open-audit"
          className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Open in Audit timeline →
        </Link>
      </div>
    </div>
  );
}

function parseFilters(params: URLSearchParams | null): WorkerActivityFilters {
  const rawRange = params?.get("range") ?? "";
  const rawType = params?.get("type") ?? "";
  const rawStatus = params?.get("status") ?? "";
  const range: WorkerActivityRange = (WORKER_ACTIVITY_RANGES as readonly string[]).includes(rawRange)
    ? (rawRange as WorkerActivityRange)
    : DEFAULT_RANGE;
  const actionType: WorkerActivityActionType | undefined = (WORKER_ACTIVITY_ACTION_TYPES as readonly string[]).includes(rawType)
    ? (rawType as WorkerActivityActionType)
    : undefined;
  const status: WorkerActivityStatus | undefined = (WORKER_ACTIVITY_STATUSES as readonly string[]).includes(rawStatus)
    ? (rawStatus as WorkerActivityStatus)
    : undefined;
  return { range, actionType, status };
}

function serializeFilters(filters: WorkerActivityFilters): string {
  const params = new URLSearchParams();
  if (filters.range !== DEFAULT_RANGE) params.set("range", filters.range);
  if (filters.actionType) params.set("type", filters.actionType);
  if (filters.status) params.set("status", filters.status);
  return params.toString();
}

function LoadingState() {
  return (
    <div data-testid="activity-loading" className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/60 py-10 text-center">
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading activity…</p>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      data-testid="activity-error"
      className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-8 px-6 text-center space-y-3"
    >
      <p className="text-sm text-[var(--foreground)]">Couldn&apos;t load activity</p>
      <p className="text-xs text-[var(--muted-foreground)]">{message}</p>
      <button
        type="button"
        onClick={onRetry}
        className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Try again
      </button>
    </div>
  );
}

function EmptyState({
  isPristine,
  workerName,
  onClearFilters,
}: {
  isPristine: boolean;
  workerName: string;
  onClearFilters: () => void;
}) {
  if (isPristine) {
    return (
      <div
        data-testid="activity-empty-pristine"
        className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-12 px-6 text-center"
      >
        <p className="text-sm font-medium text-[var(--foreground)]">No activity yet.</p>
        <p className="mt-1 text-sm text-[var(--muted-foreground)]">
          {workerName} hasn&apos;t taken any actions yet. Once she&apos;s working, you&apos;ll see her activity here.
        </p>
      </div>
    );
  }
  return (
    <div
      data-testid="activity-empty-filtered"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">No matching activity</p>
      <p className="text-sm text-[var(--muted-foreground)]">
        Try widening the time range or removing filters.
      </p>
      <button
        type="button"
        data-testid="activity-clear-filters"
        onClick={onClearFilters}
        className="inline-flex items-center rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
      >
        Clear filters
      </button>
    </div>
  );
}
