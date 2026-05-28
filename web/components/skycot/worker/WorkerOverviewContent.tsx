"use client";

import { useCallback, useState } from "react";
import { AlertCircle, Play } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerOverview } from "@/lib/hooks/use-worker-overview";
import {
  WhatTheyDoCard,
  WhatTheyDoCardSkeleton,
} from "@/components/skycot/worker/WhatTheyDoCard";
import { DutiesCard, DutiesCardSkeleton } from "@/components/skycot/worker/DutiesCard";
import {
  ChannelStripCard,
  ChannelStripCardSkeleton,
} from "@/components/skycot/worker/ChannelStripCard";
import { KpiCard, KpiCardSkeleton } from "@/components/skycot/worker/KpiCard";
import {
  RecentActivityCard,
  RecentActivityCardSkeleton,
} from "@/components/skycot/worker/RecentActivityCard";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";
import { Button } from "@/components/ui/Button";
import { useToast } from "@/lib/ui/use-toast";
import { triggerWorkerRun } from "@/lib/api/workers";

/**
 * Body of the Overview tab. Lives outside the Next.js route file so
 * unit tests can mount it directly with a plain `workerId` string
 * (the route page wraps this with `React.use(params)`).
 *
 * Renders four cards in fixed order:
 *
 *   1. WhatTheyDoCard    — charter + inline edit + Adjust (E4-g)
 *   2. DutiesCard        — duty list + toggle + add (E4-h)
 *   3. KpiCard           — 4 vertical-aware KPIs + 7-day sparklines
 *   4. RecentActivityCard — last 5 actions + audit drill-in
 *
 * Both data feeds (Worker + WorkerOverview) read from TanStack Query
 * with cache keys shared across the worker-profile surface, so the
 * hero (which the layout renders via `useWorker`) and the overview
 * body never race-fetch.
 *
 * RetrainDialog + AddScheduleDialog are stubbed as transient
 * inline announcements until E4-g / E4-h ship.
 */
export function WorkerOverviewContent({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);
  const overviewQuery = useWorkerOverview(workerId);
  const toast = useToast();
  const [running, setRunning] = useState(false);

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;

  const overview = overviewQuery.data?.overview;
  const overviewError = overviewQuery.isError
    ? (overviewQuery.error?.message ?? "Couldn't load this section.")
    : null;
  const retryOverview = useCallback(() => {
    overviewQuery.refetch();
  }, [overviewQuery]);

  // The AddScheduleDialog is mounted by DutiesCard now, so onAddDuty
  // is only an analytics seam — no user-facing toast required. The edit-
  // body and toggle paths are intentionally inert until the duty PATCH
  // endpoint lands; we render those affordances as disabled (via
  // `canToggleDuty={false}` below) rather than firing a toast that leaks
  // an engineering brief-ID like "E4-h" into the customer surface. See
  // fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.
  const onAddDuty = useCallback(() => {
    /* analytics seam — dialog mount happens inside DutiesCard */
  }, []);
  const onSaveCharter = useCallback(() => {
    /* charter persistence lands with the worker-charter PATCH endpoint */
  }, []);

  // Run-now (Scene 2 of v1.0 demo runbook): POST to the gateway,
  // toast with a deep-link to the just-emitted audit row. Stays out
  // of the inert-stub trap by routing through a real endpoint that
  // emits a real `worker.run` audit row — see brief
  // `fix-worker-detail-run-now-action-and-endpoint`.
  const onRunNow = useCallback(async () => {
    if (running) return;
    setRunning(true);
    try {
      const result = await triggerWorkerRun(workerId);
      const workerName = workerQuery.data?.name ?? "Worker";
      toast.success(`${workerName} is running now`, {
        description: "Action queued. The audit row is live.",
        action: {
          label: "View in audit",
          onClick: () => {
            window.location.href = `/audit/action/${encodeURIComponent(result.action_id)}`;
          },
        },
      });
    } catch (err) {
      toast.error("Couldn't trigger a run", {
        description: err instanceof Error ? err.message : undefined,
      });
    } finally {
      setRunning(false);
    }
  }, [running, workerId, workerQuery.data?.name, toast]);

  if (overviewQuery.isPending) {
    return (
      <div
        role="tabpanel"
        aria-label="Overview tab"
        data-testid="worker-overview-loading"
        className="space-y-4"
      >
        <WhatTheyDoCardSkeleton />
        <DutiesCardSkeleton />
        <ChannelStripCardSkeleton />
        <KpiCardSkeleton />
        <RecentActivityCardSkeleton />
      </div>
    );
  }

  if (overviewQuery.isError || !overview) {
    return (
      <div
        role="tabpanel"
        aria-label="Overview tab"
        data-testid="worker-overview-error"
        className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-10 px-6 text-center space-y-3"
      >
        <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
          <AlertCircle />
        </div>
        <p className="text-sm text-[var(--foreground)]">
          {overviewError ?? "Couldn't load the Overview."}
        </p>
        <button
          type="button"
          onClick={retryOverview}
          className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Try again
        </button>
      </div>
    );
  }

  return (
    <div
      role="tabpanel"
      aria-label="Overview tab"
      data-testid="worker-tab-panel-overview"
      className="space-y-4"
    >
      <div
        data-testid="worker-overview-actions"
        className="flex items-center justify-end gap-2"
      >
        <Button
          type="button"
          variant="primary"
          size="md"
          onClick={onRunNow}
          loading={running}
          data-testid="worker-overview-run-now"
        >
          <Play aria-hidden="true" />
          Run now
        </Button>
      </div>
      <WhatTheyDoCard
        charter={overview.charter}
        canEdit={permissions.canEditCharter}
        onSave={onSaveCharter}
        workerId={workerId}
        workerName={workerQuery.data?.name}
      />
      <DutiesCard
        duties={overview.duties}
        canEdit={permissions.canEditDuties}
        // Toggle + body click are inert until the duty PATCH endpoint
        // lands. Rendering the toggle as disabled (rather than allowing
        // a local-only visual flip) prevents the "fake save" theater the
        // camera saw before fix-worker-duty-toggle-fake-mutation.
        canToggleDuty={false}
        workerId={workerId}
        workerName={workerQuery.data?.name}
        onAddDuty={onAddDuty}
      />
      <ChannelStripCard
        workerId={workerId}
        workerName={workerQuery.data?.name}
      />
      <KpiCard kpis={overview.kpis} />
      <RecentActivityCard workerId={workerId} rows={overview.recentActivity} />
    </div>
  );
}
