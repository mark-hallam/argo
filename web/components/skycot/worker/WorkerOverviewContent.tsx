"use client";

import { useCallback, useEffect, useState } from "react";
import { AlertCircle, Play, ShieldAlert } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { useWorkerOverview } from "@/lib/hooks/use-worker-overview";
import { useToggleDuty } from "@/lib/hooks/use-duties";
import {
  WhatTheyDoCard,
  WhatTheyDoCardSkeleton,
} from "@/components/skycot/worker/WhatTheyDoCard";
import { DutiesCard, DutiesCardSkeleton } from "@/components/skycot/worker/DutiesCard";
import { EditDutyDialog } from "@/components/skycot/worker/EditDutyDialog";
import {
  ChannelStripCard,
  ChannelStripCardSkeleton,
} from "@/components/skycot/worker/ChannelStripCard";
import { KpiCard, KpiCardSkeleton } from "@/components/skycot/worker/KpiCard";
import { WorkerCostChip } from "@/components/skycot/worker/WorkerCostChip";
import {
  RecentActivityCard,
  RecentActivityCardSkeleton,
} from "@/components/skycot/worker/RecentActivityCard";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";
import { ProbationApprovalCard } from "@/components/skycot/worker/ProbationApprovalCard";
import { CapabilityGrantCard } from "@/components/skycot/worker/CapabilityGrantCard";
import { saveLastRunSummary, loadLastRunSummary } from "@/components/skycot/worker/last-run-summary";
import { Button } from "@/components/ui/Button";
import { useToast } from "@/lib/ui/use-toast";
import { triggerWorkerRun } from "@/lib/api/workers";
import { authorizeConnectorScopes } from "@/lib/api/connectors";
import { hasRealSession } from "@/lib/skycot/demo-mode";
import type {
  CapabilityGrantRequest,
  GrantSelection,
} from "@/lib/types/capability-grant";
import type { WorkerDuty } from "@/lib/api/worker-overview";

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
  const [editingDuty, setEditingDuty] = useState<WorkerDuty | null>(null);
  // Probation state derived ONLY from the last run response (the
  // worker-detail / overview payloads don't expose probation — see
  // `lib/api/worker-overview.ts`; we don't invent backend fields
  // beyond the additive ones on the run envelope). Only the held tool
  // list is needed: approval now rides ON the next run
  // (`approve_tools`), not on the finished run's session.
  const [heldTools, setHeldTools] = useState<string[] | null>(null);
  const [inProbation, setInProbation] = useState(false);
  // Connect-on-demand: a pending capability/connector grant-request the
  // run surfaced (additive `capability_request` field on the run
  // envelope — see `lib/api/workers.ts`; absent until the backend
  // emission sibling brief lands). Mock-gated on render so it never
  // shows against a fixture; `connecting` drives the card's busy state.
  const [grantRequest, setGrantRequest] =
    useState<CapabilityGrantRequest | null>(null);
  const [connecting, setConnecting] = useState(false);
  const toggleMutation = useToggleDuty(workerId);

  // Rehydrate the held-tools / probation state from the per-worker
  // last-run summary on mount. The page remounts right after a Run-now,
  // dropping the in-memory response — without this the approval card the
  // toast pointed at ("approve the held tools below") never re-renders.
  // The Output card rehydrates the same summary's `job_id` itself (it's a
  // sibling in the route, not a child here). No summary → honest empty
  // states stay (a never-run worker).
  useEffect(() => {
    const summary = loadLastRunSummary(workerId);
    if (!summary) return;
    setHeldTools(summary.heldTools.length > 0 ? summary.heldTools : null);
    setInProbation(summary.probation);
  }, [workerId]);

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;

  const overview = overviewQuery.data?.overview;
  const overviewError = overviewQuery.isError
    ? (overviewQuery.error?.message ?? "Couldn't load this section.")
    : null;
  const retryOverview = useCallback(() => {
    overviewQuery.refetch();
  }, [overviewQuery]);

  // The AddScheduleDialog is mounted by DutiesCard, so onAddDuty is
  // only an analytics seam — no user-facing toast required.
  const onAddDuty = useCallback(() => {
    /* analytics seam — dialog mount happens inside DutiesCard */
  }, []);
  const onSaveCharter = useCallback(() => {
    /* charter persistence lands with the worker-charter PATCH endpoint */
  }, []);

  const onToggleDuty = useCallback(
    (dutyId: string, nextEnabled: boolean) => {
      toggleMutation.mutate(
        { dutyId, enabled: nextEnabled },
        {
          onError: () => {
            toast.error("Couldn't save schedule. Try again.");
          },
        },
      );
    },
    [toggleMutation, toast],
  );

  const onEditDuty = useCallback(
    (dutyId: string) => {
      const target = overview?.duties.find((d) => d.id === dutyId);
      if (target) setEditingDuty(target);
    },
    [overview],
  );

  const closeEditDialog = useCallback(() => setEditingDuty(null), []);
  const onEditSaveError = useCallback(
    (_err: Error) => {
      toast.error("Couldn't save schedule. Try again.");
    },
    [toast],
  );

  // Run-now (Scene 2 of v1.0 demo runbook): POST to the gateway,
  // toast with a deep-link to the just-emitted audit row. Stays out
  // of the inert-stub trap by routing through a real endpoint that
  // emits a real `worker.run` audit row — see brief
  // `fix-worker-detail-run-now-action-and-endpoint`.
  //
  // `approveTools` is the keystone "Approve & run" path: ONE call that
  // runs the teammate WITH the operator's per-tool one-time OK riding
  // on it (`approve_tools` → one-shot grants on the NEW run's session),
  // instead of approving the finished run's dead session and watching
  // the next run hold the same tools again.
  const runNow = useCallback(
    async (approveTools?: string[]) => {
      if (running) return;
      setRunning(true);
      try {
        const result = await triggerWorkerRun(workerId, undefined, approveTools);
        const workerName = workerQuery.data?.name ?? "Worker";
        const approved = approveTools !== undefined && approveTools.length > 0;
        // Day-1 unblock FIX 1 (NO-THEATER): the run envelope's additive
        // `held_tools` / `probation` fields say whether the 24h probation
        // window held the teammate's tools. A green "running now" toast
        // over a held run is theater — surface the honest state and the
        // in-SPA approval card instead. Both fields are optional so older
        // gateways keep the previous behavior.
        setInProbation(Boolean(result.probation));
        // Connect-on-demand: if the run hit a connector the teammate
        // isn't authorized for, the gateway returns a
        // `capability_request` — surface the grant card. Real backend
        // data, but mock-gated so a fixture-driven session never shows
        // a connect prompt it can't honestly fulfil.
        setGrantRequest(result.capability_request ?? null);
        const held = result.held_tools ?? [];
        // Persist the run summary so the Output card + the approval card
        // survive the post-run remount (they live in sibling subtrees, so
        // a shared parent state can't bridge them — sessionStorage does).
        saveLastRunSummary(workerId, {
          jobId: result.job_id || null,
          heldTools: held,
          probation: Boolean(result.probation),
          at: Date.now(),
        });
        if (held.length > 0) {
          // Progressive approval: the approved tools fired and the
          // teammate reached a NEW set — the card re-renders with it.
          setHeldTools(held);
          toast.warning(
            approved
              ? `${workerName} got further, but reached more held tools`
              : `${workerName} ran, but their tools are held by the probation window`,
            {
              description: approved
                ? "Approve the new tools below to keep going — it's a one-time OK."
                : "Approve the held tools below — your OK rides on the next run.",
            },
          );
          return;
        }
        setHeldTools(null);
        toast.success(
          approved
            ? `${workerName} did the work`
            : `${workerName} is running now`,
          {
            description: approved
              ? "Approved tools ran — see the audit trail and approvals for the full record."
              : "Action queued. The audit row is live.",
            action: {
              label: "View in audit",
              onClick: () => {
                window.location.href = `/audit/action/${encodeURIComponent(result.action_id)}`;
              },
            },
          },
        );
      } catch (err) {
        toast.error("Couldn't trigger a run", {
          description: err instanceof Error ? err.message : undefined,
        });
      } finally {
        setRunning(false);
      }
    },
    [running, workerId, workerQuery.data?.name, toast],
  );
  const onRunNow = useCallback(() => runNow(), [runNow]);

  // Connect-on-demand "Connect" → authorize the connector with the
  // operator's chosen per-scope access. Honest surface: a failed
  // authorize keeps the card up and toasts the real error (no silent
  // "connected" theater); success clears the card and points at audit.
  const onConnectGrant = useCallback(
    async (scopes: GrantSelection[]) => {
      if (connecting || !grantRequest) return;
      setConnecting(true);
      try {
        await authorizeConnectorScopes({
          workerId,
          connectorId: grantRequest.connector.id,
          scopes,
        });
        const connName = grantRequest.connector.name;
        setGrantRequest(null);
        toast.success(`Connected ${connName}`, {
          description:
            "The teammate can use it now — every action stays logged in the audit trail.",
        });
      } catch (err) {
        toast.error("Couldn't connect", {
          description:
            err instanceof Error ? err.message : "The connect didn't go through.",
        });
      } finally {
        setConnecting(false);
      }
    },
    [connecting, grantRequest, workerId, toast],
  );
  const onDismissGrant = useCallback(() => setGrantRequest(null), []);

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
        className="flex items-center justify-between gap-2"
      >
        {overview.genesis_plan_id ? (
          <span
            data-testid="worker-overview-plan-provenance"
            data-plan-id={overview.genesis_plan_id}
            title={`Synthesised by Genesis as ${overview.genesis_plan_id}`}
            className="inline-flex items-center gap-1 rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2.5 py-1 text-xs font-medium text-[var(--muted-foreground)]"
          >
            <span aria-hidden="true">✨</span>
            Genesis plan:{" "}
            <code className="font-mono text-[11px]">
              {overview.genesis_plan_id}
            </code>
          </span>
        ) : (
          <span />
        )}
        <span className="inline-flex items-center gap-2">
          {inProbation ? (
            <span
              data-testid="worker-probation-chip"
              title="New teammates work under a 24-hour probation window — every tool needs your one-time OK."
              className="inline-flex items-center gap-1 rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2.5 py-1 text-xs font-medium text-[var(--muted-foreground)]"
            >
              <ShieldAlert aria-hidden="true" className="size-3.5" />
              24h probation
            </span>
          ) : null}
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
        </span>
      </div>
      {heldTools && heldTools.length > 0 ? (
        <ProbationApprovalCard
          workerName={workerQuery.data?.name ?? "This teammate"}
          heldTools={heldTools}
          onApproveAndRun={(tools) => runNow(tools)}
        />
      ) : null}
      {grantRequest && grantRequest.scopes.length > 0 && hasRealSession() ? (
        <CapabilityGrantCard
          request={grantRequest}
          onConnect={onConnectGrant}
          onDismiss={onDismissGrant}
          busy={connecting}
        />
      ) : null}
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
        workerId={workerId}
        workerName={workerQuery.data?.name}
        onAddDuty={onAddDuty}
        onToggle={onToggleDuty}
        onEditDuty={onEditDuty}
      />
      <ChannelStripCard
        workerId={workerId}
        workerName={workerQuery.data?.name}
      />
      <div className="flex items-center justify-end">
        <WorkerCostChip workerId={workerId} />
      </div>
      <KpiCard kpis={overview.kpis} />
      <RecentActivityCard workerId={workerId} rows={overview.recentActivity} />
      <EditDutyDialog
        open={editingDuty !== null}
        workerId={workerId}
        duty={editingDuty}
        onOpenChange={(next) => {
          if (!next) closeEditDialog();
        }}
        onError={onEditSaveError}
      />
    </div>
  );
}
