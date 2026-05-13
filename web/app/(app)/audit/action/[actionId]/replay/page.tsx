"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { AlertOctagon, ChevronLeft, Download, RotateCcw } from "lucide-react";
import { ProvenanceDiffCard } from "@/components/skycot/audit/ProvenanceDiffCard";
import { ReplayDiff } from "@/components/skycot/audit/ReplayDiff";
import { ReplayHeader } from "@/components/skycot/audit/ReplayHeader";
import { ReplayProgress, REPLAY_STAGES } from "@/components/skycot/audit/ReplayProgress";
import { useAuditDetail } from "@/lib/hooks/use-audit-detail";
import { useReplay } from "@/lib/hooks/use-replay";
import type { ReplayOutcome } from "@/lib/api/replay";

/**
 * `/audit/action/<actionId>/replay` — sandboxed replay surface. The
 * E5-c drill-in's "Replay" button routes here; the route mounts,
 * fires the `useReplay` mutation, and renders one of:
 *
 *   success(reproducible|drifted-aligned|drifted-misaligned)
 *   unavailable(model-decommissioned)
 *   failed(reason)
 *   leak-detected(details)
 *
 * Side effects happen server-side; this surface only renders the
 * envelope. Verdict logic (same output AND same provenance =
 * reproducible; different output = drifted-aligned unless the
 * replayed output would fail policy → drifted-misaligned) is owned
 * by the sibling backend brief.
 */

const TOAST_TTL_MS = 2500;
const PROGRESS_TICK_MS = 250;

export default function ReplayPage() {
  return (
    <Suspense fallback={<ReplayFallback />}>
      <ReplayContent />
    </Suspense>
  );
}

function ReplayFallback() {
  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-12">
      <ReplayProgress current={1} total={REPLAY_STAGES.length} />
    </div>
  );
}

function ReplayContent() {
  const params = useParams<{ actionId: string }>();
  const actionId = decodeURIComponent(params?.actionId ?? "");
  const router = useRouter();
  const replay = useReplay(actionId);
  const detail = useAuditDetail(actionId, { enabled: actionId.length > 0 });
  const [toast, setToast] = useState<string | null>(null);
  const [stepIdx, setStepIdx] = useState(1);

  useEffect(() => {
    if (!actionId) return;
    if (replay.isIdle) replay.mutate();
  }, [actionId, replay]);

  useEffect(() => {
    if (!replay.isPending) return;
    const handle = window.setInterval(() => {
      setStepIdx((s) => Math.min(s + 1, REPLAY_STAGES.length));
    }, PROGRESS_TICK_MS);
    return () => window.clearInterval(handle);
  }, [replay.isPending]);

  useEffect(() => {
    if (!toast) return;
    const handle = window.setTimeout(() => setToast(null), TOAST_TTL_MS);
    return () => window.clearTimeout(handle);
  }, [toast]);

  const onBack = useCallback(() => {
    router.push(`/audit/action/${encodeURIComponent(actionId)}`);
  }, [router, actionId]);

  const onExport = useCallback(() => {
    setToast("Replay artifact export ships in E5-e");
  }, []);

  const onRetry = useCallback(() => {
    setStepIdx(1);
    replay.reset();
    replay.mutate();
  }, [replay]);

  const outcome: ReplayOutcome | undefined = replay.isError
    ? { kind: "failed", reason: replay.error?.message ?? "Replay sandbox unreachable" }
    : replay.data;

  return (
    <div
      data-testid="replay-page"
      data-action-id={actionId}
      className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-6 space-y-4"
    >
      <ReplayHeader
        actionId={actionId}
        verdict={outcome?.kind === "success" ? outcome.verdict : undefined}
      />

      {replay.isPending || replay.isIdle ? (
        <ReplayProgress current={stepIdx} total={REPLAY_STAGES.length} />
      ) : outcome ? (
        <OutcomeView
          outcome={outcome}
          actionId={actionId}
          originalOutputAvailable={!!detail.data}
          originalOutputBody={
            detail.data && typeof detail.data.output.body !== "object"
              ? String(detail.data.output.body)
              : detail.data
                ? JSON.stringify(detail.data.output.body, null, 2)
                : undefined
          }
          onRetry={onRetry}
        />
      ) : null}

      <FooterButtons onBack={onBack} onExport={onExport} />

      {toast ? (
        <div
          role="status"
          data-testid="replay-toast"
          className="fixed bottom-6 left-1/2 -translate-x-1/2 z-30 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--foreground)] shadow-md"
        >
          {toast}
        </div>
      ) : null}
    </div>
  );
}

interface OutcomeViewProps {
  outcome: ReplayOutcome;
  actionId: string;
  originalOutputAvailable: boolean;
  originalOutputBody: string | undefined;
  onRetry: () => void;
}

function OutcomeView({
  outcome,
  originalOutputAvailable,
  originalOutputBody,
  onRetry,
}: OutcomeViewProps) {
  if (outcome.kind === "success") {
    return (
      <div className="space-y-4">
        <ReplayDiff result={outcome.result} />
        <ProvenanceDiffCard diff={outcome.result.provenanceDiff} />
      </div>
    );
  }
  if (outcome.kind === "unavailable") {
    return (
      <UnavailableView
        modelName={outcome.modelName}
        originalAvailable={originalOutputAvailable}
        originalBody={originalOutputBody}
      />
    );
  }
  if (outcome.kind === "failed") {
    return <FailedView reason={outcome.reason} onRetry={onRetry} />;
  }
  return <LeakView details={outcome.details} />;
}

function UnavailableView({
  modelName,
  originalAvailable,
  originalBody,
}: {
  modelName: string;
  originalAvailable: boolean;
  originalBody: string | undefined;
}) {
  return (
    <section
      data-testid="replay-unavailable"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <h2 className="text-sm font-semibold text-[var(--foreground)]">
        Replay unavailable
      </h2>
      <p data-testid="replay-unavailable-reason" className="text-sm text-[var(--muted-foreground)]">
        <span className="font-mono text-[var(--foreground)]">{modelName}</span> is no
        longer deployable in the sandbox. Original output preserved below.
      </p>
      {originalAvailable && originalBody ? (
        <pre
          data-testid="replay-unavailable-original"
          className="max-h-96 overflow-auto rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/20 p-3 text-sm font-mono whitespace-pre-wrap break-words text-[var(--foreground)]"
        >
          {originalBody}
        </pre>
      ) : (
        <p className="text-xs text-[var(--muted-foreground)]">
          Original output unavailable for preview.
        </p>
      )}
    </section>
  );
}

function FailedView({ reason, onRetry }: { reason: string; onRetry: () => void }) {
  return (
    <section
      data-testid="replay-failed"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <h2 className="text-sm font-semibold text-[var(--foreground)]">Replay failed</h2>
      <p data-testid="replay-failed-reason" className="text-sm text-[var(--muted-foreground)]">
        {reason}
      </p>
      <button
        type="button"
        data-testid="replay-failed-retry"
        onClick={onRetry}
        className="inline-flex items-center gap-1.5 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
      >
        <RotateCcw aria-hidden className="size-4" /> Try again
      </button>
    </section>
  );
}

function LeakView({ details }: { details: string }) {
  return (
    <section
      role="alert"
      data-testid="replay-leak"
      className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/50 bg-[var(--destructive)]/10 p-5 space-y-3"
    >
      <div className="flex items-start gap-2">
        <AlertOctagon aria-hidden className="size-5 text-[var(--destructive)] mt-0.5" />
        <div className="space-y-2">
          <h2 className="text-sm font-semibold text-[var(--destructive)]">
            Replay aborted — side-effect leak detected
          </h2>
          <p className="text-sm text-[var(--foreground)]">
            This should never happen. A self-audit row has been written.
            Please report to operations.
          </p>
          <p data-testid="replay-leak-details" className="text-xs font-mono text-[var(--muted-foreground)]">
            {details}
          </p>
        </div>
      </div>
    </section>
  );
}

function FooterButtons({ onBack, onExport }: { onBack: () => void; onExport: () => void }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--border)] pt-4">
      <button
        type="button"
        data-testid="replay-back"
        onClick={onBack}
        className="inline-flex items-center gap-1.5 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
      >
        <ChevronLeft aria-hidden className="size-4" /> Back to action
      </button>
      <button
        type="button"
        data-testid="replay-export"
        onClick={onExport}
        className="inline-flex items-center gap-1.5 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-1.5 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
      >
        <Download aria-hidden className="size-4" /> Export replay artifact
      </button>
    </div>
  );
}
