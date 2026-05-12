"use client";

import { useParams } from "next/navigation";
import { AlertCircle, Loader2 } from "lucide-react";
import { AdvancedSection } from "@/components/skycot/worker/settings/AdvancedSection";
import { AutonomySection } from "@/components/skycot/worker/settings/AutonomySection";
import { BudgetSection } from "@/components/skycot/worker/settings/BudgetSection";
import { ChannelsSection } from "@/components/skycot/worker/settings/ChannelsSection";
import { DangerZone } from "@/components/skycot/worker/settings/DangerZone";
import { QuietHoursSection } from "@/components/skycot/worker/settings/QuietHoursSection";
import { useWorker } from "@/lib/hooks/use-worker";
import {
  useSettingsActions,
  useWorkerSettings,
} from "@/lib/hooks/use-worker-settings";
import { DEFAULT_DELEGATOR_PERMISSIONS } from "@/lib/types/worker";

/**
 * Per-worker Settings tab — `/workers/<id>/settings`. Six sections
 * in a fixed visual order: Channels → Quiet hours → Budget →
 * Autonomy → Advanced (operator-only) → Danger zone. The static
 * route overrides the `[tab]/page.tsx` stub from E4-a — HARD RULE 5
 * respected, the placeholder remains untouched.
 *
 * Data flows through `useWorkerSettings(workerId)`; mutations are
 * exposed through `useSettingsActions(workerId)` which optimistically
 * patches the cache on every save.
 */

export default function WorkerSettingsPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const settingsQuery = useWorkerSettings(workerId);
  const actions = useSettingsActions(workerId);

  if (settingsQuery.isPending || workerQuery.isPending) {
    return <LoadingState />;
  }

  if (settingsQuery.isError || !settingsQuery.data) {
    return (
      <ErrorState
        message={settingsQuery.error?.message ?? "Couldn't load settings"}
        onRetry={() => settingsQuery.refetch()}
      />
    );
  }

  const worker = workerQuery.data;
  const workerName = worker?.name ?? "This worker";
  const isPaused = worker?.status === "paused";
  const permissions = worker?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;
  const settings = settingsQuery.data.settings;

  return (
    <div
      role="tabpanel"
      aria-label="Settings tab"
      data-testid="worker-tab-panel-settings"
      className="space-y-4"
    >
      <ChannelsSection
        value={settings.channels}
        workerName={workerName}
        onSave={actions.onUpdateChannels}
      />
      <QuietHoursSection
        value={settings.quietHours}
        workerName={workerName}
        onSave={actions.onUpdateQuietHours}
      />
      <BudgetSection value={settings.budget} onSave={actions.onUpdateBudget} />
      <AutonomySection
        value={settings.autonomy}
        workerName={workerName}
        onSave={actions.onUpdateAutonomy}
      />
      {permissions.canViewSettings ? (
        <AdvancedSection workerId={workerId} advanced={settings.advanced} />
      ) : null}
      <DangerZone
        workerName={workerName}
        permissions={permissions}
        isPaused={isPaused}
        onPause={actions.onPause}
        onArchive={actions.onArchive}
        onDelete={actions.onDelete}
      />
    </div>
  );
}

function LoadingState() {
  return (
    <div
      role="tabpanel"
      aria-label="Settings tab"
      data-testid="worker-settings-loading"
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/60 py-10 text-center"
    >
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading settings…</p>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      role="tabpanel"
      aria-label="Settings tab"
      data-testid="worker-settings-error"
      className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-10 px-6 text-center space-y-3"
    >
      <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle />
      </div>
      <p className="text-sm text-[var(--foreground)]">{message}</p>
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
