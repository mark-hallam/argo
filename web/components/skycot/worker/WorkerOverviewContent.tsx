"use client";

import { useCallback, useEffect, useState } from "react";
import { AlertCircle } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerOverview } from "@/lib/hooks/use-worker-overview";
import {
  WhatSheDoesCard,
  WhatSheDoesCardSkeleton,
} from "@/components/skycot/worker/WhatSheDoesCard";
import { DutiesCard, DutiesCardSkeleton } from "@/components/skycot/worker/DutiesCard";
import { KpiCard, KpiCardSkeleton } from "@/components/skycot/worker/KpiCard";
import {
  RecentActivityCard,
  RecentActivityCardSkeleton,
} from "@/components/skycot/worker/RecentActivityCard";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";

/**
 * Body of the Overview tab. Lives outside the Next.js route file so
 * unit tests can mount it directly with a plain `workerId` string
 * (the route page wraps this with `React.use(params)`).
 *
 * Renders four cards in fixed order:
 *
 *   1. WhatSheDoesCard   — charter + inline edit + Adjust (E4-g)
 *   2. DutiesCard        — duty list + toggle + add (E4-h)
 *   3. KpiCard           — 4 vertical-aware KPIs + 7-day sparklines
 *   4. RecentActivityCard — last 5 actions + audit drill-in
 *
 * Both data feeds (Worker + WorkerOverview) read from TanStack Query
 * with cache keys shared across the worker-profile surface, so the
 * hero (which the layout renders via `useWorker`) and the overview
 * body never race-fetch.
 *
 * RetrainDialog + AddResponsibilityDialog are stubbed as transient
 * inline announcements until E4-g / E4-h ship.
 */
export function WorkerOverviewContent({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);
  const overviewQuery = useWorkerOverview(workerId);

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;

  const overview = overviewQuery.data?.overview;
  const overviewError = overviewQuery.isError
    ? (overviewQuery.error?.message ?? "Couldn't load this section.")
    : null;
  const retryOverview = useCallback(() => {
    overviewQuery.refetch();
  }, [overviewQuery]);

  const [announcement, setAnnouncement] = useState<string | null>(null);
  useEffect(() => {
    if (!announcement) return;
    const t = setTimeout(() => setAnnouncement(null), 3200);
    return () => clearTimeout(t);
  }, [announcement]);

  const onAdjust = useCallback(
    () => setAnnouncement("Adjust dialog ships in E4-g"),
    [],
  );
  const onAddDuty = useCallback(
    () => setAnnouncement("Add responsibility dialog ships in E4-h"),
    [],
  );
  const onEditDuty = useCallback(
    () => setAnnouncement("Edit duty ships in E4-h"),
    [],
  );
  const onSaveCharter = useCallback((next: string) => {
    setAnnouncement("Charter changes persist in E4-g");
    // eslint-disable-next-line no-console
    console.warn("[E4-b] charter save pending wiring (E4-g):", next);
  }, []);
  const onToggleDuty = useCallback((dutyId: string, enabled: boolean) => {
    // eslint-disable-next-line no-console
    console.warn(
      `[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h`,
    );
  }, []);

  if (overviewQuery.isPending) {
    return (
      <div
        role="tabpanel"
        aria-label="Overview tab"
        data-testid="worker-overview-loading"
        className="space-y-4"
      >
        <WhatSheDoesCardSkeleton />
        <DutiesCardSkeleton />
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
      <WhatSheDoesCard
        charter={overview.charter}
        canEdit={permissions.canEditCharter}
        onSave={onSaveCharter}
        onAdjust={onAdjust}
      />
      <DutiesCard
        duties={overview.duties}
        canEdit={permissions.canEditDuties}
        onToggle={onToggleDuty}
        onEditDuty={onEditDuty}
        onAddDuty={onAddDuty}
      />
      <KpiCard kpis={overview.kpis} />
      <RecentActivityCard workerId={workerId} rows={overview.recentActivity} />

      <div
        role="status"
        aria-live="polite"
        data-testid="overview-announcement"
        className="sr-only"
      >
        {announcement ?? ""}
      </div>
      {announcement ? (
        <div
          data-testid="overview-toast"
          className="fixed bottom-6 right-6 z-40 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--card-foreground)] shadow-lg"
        >
          {announcement}
        </div>
      ) : null}
    </div>
  );
}
