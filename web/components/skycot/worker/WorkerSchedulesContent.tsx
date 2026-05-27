"use client";

import { useCallback } from "react";
import { AlertCircle } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerOverview } from "@/lib/hooks/use-worker-overview";
import { DutiesCard, DutiesCardSkeleton } from "@/components/skycot/worker/DutiesCard";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";

/**
 * Body of the Schedules tab. Reuses the existing `DutiesCard` so the
 * tab is data-equivalent to the overview's duty list — the
 * tab-strip / route addition is the structural fix; the card itself
 * is unchanged. Lives outside the route file so unit tests can mount
 * it directly with a plain `workerId` string.
 *
 * Toggle + body-click stay inert (`canToggleDuty={false}`) until the
 * duty PATCH endpoint lands, matching `WorkerOverviewContent` — see
 * fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.
 */
export function WorkerSchedulesContent({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);
  const overviewQuery = useWorkerOverview(workerId);

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;
  const overview = overviewQuery.data?.overview;
  const overviewError = overviewQuery.isError
    ? (overviewQuery.error?.message ?? "Couldn't load this section.")
    : null;
  const retry = useCallback(() => {
    overviewQuery.refetch();
  }, [overviewQuery]);

  const onAddDuty = useCallback(() => {
    /* analytics seam — dialog mount happens inside DutiesCard */
  }, []);

  if (overviewQuery.isPending) {
    return (
      <div
        role="tabpanel"
        aria-label="Schedules tab"
        data-testid="worker-schedules-loading"
        className="space-y-4"
      >
        <DutiesCardSkeleton />
      </div>
    );
  }

  if (overviewQuery.isError || !overview) {
    return (
      <div
        role="tabpanel"
        aria-label="Schedules tab"
        data-testid="worker-schedules-error"
        className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-10 px-6 text-center space-y-3"
      >
        <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
          <AlertCircle />
        </div>
        <p className="text-sm text-[var(--foreground)]">
          {overviewError ?? "Couldn't load schedules."}
        </p>
        <button
          type="button"
          onClick={retry}
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
      aria-label="Schedules tab"
      data-testid="worker-tab-panel-schedules"
      className="space-y-4"
    >
      <DutiesCard
        duties={overview.duties}
        canEdit={permissions.canEditDuties}
        canToggleDuty={false}
        workerId={workerId}
        workerName={workerQuery.data?.name}
        onAddDuty={onAddDuty}
      />
    </div>
  );
}
