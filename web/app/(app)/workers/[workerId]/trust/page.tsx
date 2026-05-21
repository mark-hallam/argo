"use client";

import { useParams } from "next/navigation";
import { AlertCircle, Loader2, RefreshCw } from "lucide-react";
import { TrustGradeCard } from "@/components/skycot/worker/TrustGradeCard";
import { useWorker } from "@/lib/hooks/use-worker";
import {
  useRecomputeWorkerTrustGrade,
  useWorkerTrustGrade,
} from "@/lib/hooks/use-worker-trust";
import { useViewerPolicy } from "@/lib/hooks/use-viewer-policy";

/**
 * Per-worker Trust tab — `/workers/<id>/trust`. Reads the worker
 * from the same TanStack cache the parent layout primed
 * (`useWorker(workerId)` shares the key), then loads the trust-grade
 * snapshot from the gateway / mock fallback.
 *
 * Operators see the grade, the signals behind it, and the path to
 * the next rung. As of
 * `web-worker-trust-promotion-engine`, the grade actually moves —
 * the `argo-trust-engine` cron loop recomputes every 30 minutes and
 * admin/owner operators can trigger an on-demand recompute via the
 * button below.
 *
 * Sub-route precedence: the static segment (`trust/`) wins over the
 * sibling dynamic `[tab]/page.tsx` placeholder, so this page
 * replaces the E4-a stub without touching that file.
 */

export default function WorkerTrustPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const trustQuery = useWorkerTrustGrade(workerId);
  const recompute = useRecomputeWorkerTrustGrade(workerId);
  const policy = useViewerPolicy();
  const canRecompute = policy.viewerRole === "operator";

  const workerName = workerQuery.data?.name;

  return (
    <div
      role="tabpanel"
      aria-label="Trust grade tab"
      data-testid="worker-tab-panel-trust"
      className="space-y-4"
    >
      {trustQuery.isPending ? (
        <LoadingState />
      ) : trustQuery.isError || !trustQuery.data ? (
        <ErrorState
          message={(trustQuery.error as Error | undefined)?.message ?? "Couldn't load trust grade"}
          onRetry={() => trustQuery.refetch()}
        />
      ) : (
        <TrustGradeCard
          snapshot={trustQuery.data.snapshot}
          workerName={workerName}
        />
      )}

      <p
        data-testid="trust-grade-disclaimer"
        className="text-xs text-[var(--muted-foreground)]"
      >
        Trust grade is recomputed every 30 minutes from observed approvals, escalations, RBAC
        denials, and cross-tenant quality observations. The criteria above describe what would
        unlock the next rung once the worker meets them.
      </p>

      {canRecompute && (
        <div data-testid="trust-grade-recompute" className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => recompute.mutate()}
            disabled={recompute.isPending}
            className="inline-flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)] disabled:opacity-50"
          >
            <RefreshCw
              className={`size-4 ${recompute.isPending ? "animate-spin" : ""}`}
              aria-hidden
            />
            {recompute.isPending ? "Recomputing…" : "Recompute trust grade"}
          </button>
          {recompute.isSuccess && (
            <p className="text-xs text-[var(--muted-foreground)]">
              {recompute.data.decision === "hold"
                ? "No grade change."
                : `Grade ${recompute.data.decision === "promote" ? "promoted" : "demoted"} to ${recompute.data.toGrade}.`}
            </p>
          )}
          {recompute.isError && (
            <p className="text-xs text-[var(--destructive)]">
              Recompute failed: {recompute.error.message}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="trust-grade-loading"
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/60 py-10 text-center"
    >
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading trust grade…</p>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      data-testid="trust-grade-error"
      className="rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-8 px-6 text-center space-y-3"
    >
      <AlertCircle className="size-5 mx-auto text-[var(--destructive)]" aria-hidden />
      <p className="text-sm text-[var(--foreground)]">Couldn&apos;t load trust grade</p>
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
