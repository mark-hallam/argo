"use client";

import { useCallback } from "react";
import { AlertCircle } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerSchedules } from "@/lib/hooks/use-worker-schedules";
import { DutiesCard, DutiesCardSkeleton } from "@/components/skycot/worker/DutiesCard";
import { findSampleWorkerOverview } from "@/lib/sample-worker-overview";
import type { WorkerDuty } from "@/lib/api/worker-overview";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";

/**
 * Body of the Schedules tab. Renders the worker's REAL persisted
 * schedules via the live `listDuties` reader (#2742) rather than the
 * overview's fabricated `duties` array — this closes the NO-THEATER
 * Schedules-tab sample-data leak (DEC-007 / DEC-029).
 *
 * The bundled sample duties are passed as the anonymous-dev `fallback`
 * only; `listDuties`' own gate (`mockFallbackAllowed` = `!hasRealSession()`)
 * means a signed-in / demo-live operator NEVER sees fixtures — a gateway
 * 404 / malformed body surfaces an honest error, and an empty rule set
 * renders the honest empty state (`DutiesCard` shows "No schedules yet.").
 *
 * Toggle + body-click stay inert (`canToggleDuty={false}`) until the
 * duty PATCH endpoint lands, matching `WorkerOverviewContent` — see
 * fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.
 */
export function WorkerSchedulesContent({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);

  // Anonymous-dev fallback: the bundled sample duties for this worker,
  // byte-identical to what the Overview tab rendered pre-wire. A real
  // session / demo-live never reaches this (listDuties' NO-THEATER gate).
  const sampleFallback = useCallback(
    (): WorkerDuty[] => findSampleWorkerOverview(workerId)?.duties ?? [],
    [workerId],
  );

  const schedulesQuery = useWorkerSchedules(workerId, sampleFallback);

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;
  const schedulesError = schedulesQuery.isError
    ? (schedulesQuery.error?.message ?? "Couldn't load this section.")
    : null;
  const retry = useCallback(() => {
    schedulesQuery.refetch();
  }, [schedulesQuery]);

  const onAddDuty = useCallback(() => {
    /* analytics seam — dialog mount happens inside DutiesCard */
  }, []);

  if (schedulesQuery.isPending) {
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

  if (schedulesQuery.isError || !schedulesQuery.data) {
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
          {schedulesError ?? "Couldn't load schedules."}
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
        duties={schedulesQuery.data.duties}
        canEdit={permissions.canEditDuties}
        canToggleDuty={false}
        workerId={workerId}
        workerName={workerQuery.data?.name}
        onAddDuty={onAddDuty}
      />
    </div>
  );
}
