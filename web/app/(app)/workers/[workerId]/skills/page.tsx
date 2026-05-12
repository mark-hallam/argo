"use client";

import { useCallback, useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { AlertCircle } from "lucide-react";
import {
  SkillsActiveCard,
  SkillsActiveCardSkeleton,
} from "@/components/skycot/worker/SkillsActiveCard";
import {
  SkillsLibraryCard,
  SkillsLibraryCardSkeleton,
} from "@/components/skycot/worker/SkillsLibraryCard";
import {
  SkillsLearnedCard,
  SkillsLearnedCardSkeleton,
} from "@/components/skycot/worker/SkillsLearnedCard";
import { useWorker } from "@/lib/hooks/use-worker";
import {
  useSkillsActions,
  useWorkerSkills,
} from "@/lib/hooks/use-worker-skills";

/**
 * Per-worker Skills tab — `/workers/<id>/skills`. Three sub-sections
 * rendered in fixed order: Active → Library → Learned. The static
 * route overrides the sibling `[tab]/page.tsx` stub from E4-a
 * (HARD RULE 5 respected — the dynamic placeholder remains read-only).
 *
 * Data flows through `useWorkerSkills(workerId)`; mutations bundle
 * via `useSkillsActions(workerId)` which exposes optimistic
 * disable / try-it / promote / discard callbacks. Inline toasts
 * announce stubbed actions (View source) until the future surface
 * lands.
 */

export default function WorkerSkillsPage() {
  const routeParams = useParams<{ workerId: string }>();
  const workerId = routeParams?.workerId ?? "";
  const workerQuery = useWorker(workerId);
  const skillsQuery = useWorkerSkills(workerId);
  const actions = useSkillsActions(workerId);

  const workerName = workerQuery.data?.name ?? "This worker";

  const [announcement, setAnnouncement] = useState<string | null>(null);
  useEffect(() => {
    if (!announcement) return;
    const t = setTimeout(() => setAnnouncement(null), 3200);
    return () => clearTimeout(t);
  }, [announcement]);

  const onViewSource = useCallback((skillId: string) => {
    setAnnouncement(`Skill source viewer ships with the marketplace surface (skill: ${skillId})`);
  }, []);

  if (skillsQuery.isPending) {
    return (
      <div
        role="tabpanel"
        aria-label="Skills tab"
        data-testid="worker-skills-loading"
        className="space-y-4"
      >
        <SkillsActiveCardSkeleton />
        <SkillsLibraryCardSkeleton />
        <SkillsLearnedCardSkeleton />
      </div>
    );
  }

  if (skillsQuery.isError || !skillsQuery.data) {
    return (
      <ErrorState
        message={skillsQuery.error?.message ?? "Couldn't load skills"}
        onRetry={() => skillsQuery.refetch()}
      />
    );
  }

  const { active, library, learned } = skillsQuery.data.skills;

  return (
    <div
      role="tabpanel"
      aria-label="Skills tab"
      data-testid="worker-tab-panel-skills"
      className="space-y-4"
    >
      <SkillsActiveCard
        workerId={workerId}
        workerName={workerName}
        skills={active}
        onDisable={actions.onDisable}
        onViewSource={onViewSource}
      />
      <SkillsLibraryCard skills={library} onTry={actions.onTry} />
      <SkillsLearnedCard
        workerName={workerName}
        patterns={learned}
        onPromote={actions.onPromote}
        onDiscard={actions.onDiscard}
      />

      <div
        role="status"
        aria-live="polite"
        data-testid="skills-announcement"
        className="sr-only"
      >
        {announcement ?? ""}
      </div>
      {announcement ? (
        <div
          data-testid="skills-toast"
          className="fixed bottom-6 right-6 z-40 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--card-foreground)] shadow-lg"
        >
          {announcement}
        </div>
      ) : null}
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      role="tabpanel"
      aria-label="Skills tab"
      data-testid="worker-skills-error"
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

