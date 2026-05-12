"use client";

import { useCallback, useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { AlertCircle } from "lucide-react";
import { AddFactDialog } from "@/components/skycot/worker/AddFactDialog";
import {
  ActiveConversationsCard,
  ActiveConversationsCardSkeleton,
} from "@/components/skycot/worker/ActiveConversationsCard";
import {
  MemoryFactsCard,
  MemoryFactsCardSkeleton,
} from "@/components/skycot/worker/MemoryFactsCard";
import {
  RecentDecisionsCard,
  RecentDecisionsCardSkeleton,
} from "@/components/skycot/worker/RecentDecisionsCard";
import {
  VoiceProfileSummary,
  VoiceProfileSummarySkeleton,
} from "@/components/skycot/worker/VoiceProfileSummary";
import {
  useWorkerMemory,
  useWorkerMemoryActions,
} from "@/lib/hooks/use-worker-memory";
import { useWorker } from "@/lib/hooks/use-worker";
import { useRedactionPolicy } from "@/lib/hooks/use-redaction-policy";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type WorkerPermissions,
} from "@/lib/types/worker";

/**
 * Memory tab — `/workers/<id>/memory`.
 *
 * Renders four cards in fixed order (Facts → Active conversations →
 * Recent decisions → Voice profile) plus an operator-only namespace
 * badge at the top. Memory mutations (add / edit / delete fact)
 * flow through `useWorkerMemoryActions`, which wires optimistic
 * cache updates against the shared TanStack query key. The Adjust
 * button surfaces a transient toast pointing at E4-g; the
 * RetrainDialog lands there.
 *
 * Sub-route precedence: this static segment wins over the sibling
 * `[tab]/page.tsx` placeholder, replacing E4-a's "coming in E4-d"
 * stub without touching that file (HARD RULE 5 — `[tab]/page.tsx`
 * is read-only here).
 */

export default function WorkerMemoryPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const memoryQuery = useWorkerMemory(workerId);
  const actions = useWorkerMemoryActions(workerId);
  const redactionPolicy = useRedactionPolicy();

  const permissions: WorkerPermissions =
    workerQuery.data?.permissions ?? DEFAULT_DELEGATOR_PERMISSIONS;
  const workerName = workerQuery.data?.name ?? "This worker";

  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [announcement, setAnnouncement] = useState<string | null>(null);

  useEffect(() => {
    if (!announcement) return;
    const t = setTimeout(() => setAnnouncement(null), 3200);
    return () => clearTimeout(t);
  }, [announcement]);

  // RetrainDialog (E4-g) mounts inside VoiceProfileSummary. Kept as
  // an analytics seam for callers; the dialog itself opens internally.
  const onAdjustVoice = useCallback(() => {
    /* no-op — dialog mount lives in VoiceProfileSummary */
  }, []);
  const onRequestAdd = useCallback(() => setAddDialogOpen(true), []);
  const onSaveFact = useCallback(
    async (text: string) => {
      await actions.onAdd(text);
      setAnnouncement("Fact saved");
    },
    [actions],
  );
  const onEditFact = useCallback(
    async (factId: string, text: string) => {
      await actions.onEdit(factId, text);
      setAnnouncement("Fact updated");
    },
    [actions],
  );
  const onDeleteFact = useCallback(
    async (factId: string) => {
      await actions.onDelete(factId);
      setAnnouncement("Fact removed");
    },
    [actions],
  );
  const onRetry = useCallback(() => {
    memoryQuery.refetch();
  }, [memoryQuery]);

  if (memoryQuery.isPending) {
    return (
      <div
        role="tabpanel"
        aria-label="Memory tab"
        data-testid="worker-memory-loading"
        className="space-y-4"
      >
        <MemoryFactsCardSkeleton />
        <ActiveConversationsCardSkeleton />
        <RecentDecisionsCardSkeleton />
        <VoiceProfileSummarySkeleton />
      </div>
    );
  }

  if (memoryQuery.isError || !memoryQuery.data) {
    const message =
      memoryQuery.error?.message ?? "Couldn't load this worker's memory.";
    return (
      <div
        role="tabpanel"
        aria-label="Memory tab"
        data-testid="worker-memory-error"
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

  const memory = memoryQuery.data.memory;

  return (
    <div
      role="tabpanel"
      aria-label="Memory tab"
      data-testid="worker-tab-panel-memory"
      className="space-y-4"
    >
      {permissions.canViewSettings ? (
        <div
          data-testid="memory-namespace-badge"
          className="flex flex-wrap items-center gap-2 text-xs"
        >
          <span className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-1 font-mono text-[var(--muted-foreground)]">
            Namespace: {memory.namespaceId}
          </span>
          <span className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-1 font-mono text-[var(--muted-foreground)]">
            {memory.tenantScope}
          </span>
        </div>
      ) : null}

      <MemoryFactsCard
        workerName={workerName}
        facts={memory.facts}
        canEdit={permissions.canEditCharter}
        onAdd={onSaveFact}
        onEdit={onEditFact}
        onDelete={onDeleteFact}
        onRequestAdd={onRequestAdd}
        redactionPolicy={redactionPolicy}
      />

      <ActiveConversationsCard
        conversations={memory.activeConversations}
        redactionPolicy={redactionPolicy}
      />

      <RecentDecisionsCard decisions={memory.recentDecisions} />

      <VoiceProfileSummary
        voice={memory.voiceProfile}
        onAdjust={onAdjustVoice}
        workerId={workerId}
        workerName={workerQuery.data?.name}
      />

      <AddFactDialog
        open={addDialogOpen}
        onOpenChange={setAddDialogOpen}
        workerName={workerName}
        onSave={onSaveFact}
      />

      <div
        role="status"
        aria-live="polite"
        data-testid="memory-announcement"
        className="sr-only"
      >
        {announcement ?? ""}
      </div>
      {announcement ? (
        <div
          data-testid="memory-toast"
          className="fixed bottom-6 right-6 z-40 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--card-foreground)] shadow-lg"
        >
          {announcement}
        </div>
      ) : null}
    </div>
  );
}
