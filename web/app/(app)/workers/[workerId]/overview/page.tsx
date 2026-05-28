"use client";

import { CalendarClock, Sparkles, X } from "lucide-react";
import { use, useEffect, useState } from "react";
import { WorkerOverviewContent } from "@/components/skycot/worker/WorkerOverviewContent";
import { RecentChangesPanel } from "@/components/skycot/worker/RecentChangesPanel";
import { WorkerPersonalityCard } from "@/components/skycot/worker/WorkerPersonalityCard";
import { findDemoWorker } from "@/lib/api/demo-roster-store";
import { isDemoMode } from "@/lib/skycot/demo-mode";

/**
 * Overview tab — the default sub-route under `/workers/<id>`.
 *
 * The Next.js convention is for route components to receive `params`
 * as a `Promise`. We unwrap it here with `React.use()` and delegate
 * the four-card body to `<WorkerOverviewContent />` (the unit-test
 * harness mounts that directly with a plain `workerId` string to
 * avoid Promise-suspense flake in jsdom). The `<RecentChangesPanel>`
 * (E4-g) sits below — newest retrain entries with 14-day undo.
 */
export default function WorkerOverviewPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  return (
    <div className="space-y-4">
      <DemoStagedHireBanner workerId={workerId} />
      <JustHiredScheduleCallout workerId={workerId} />
      <WorkerPersonalityCard workerId={workerId} />
      <WorkerOverviewContent workerId={workerId} />
      <RecentChangesPanel workerId={workerId} />
    </div>
  );
}

/**
 * Honest-mocking banner shown when the operator is viewing a worker
 * hired via the demo-mode short-circuit (no live gateway, no real
 * provisioning). Mirrors the green "Mock provider connected" banner
 * on /onboarding/connect-provider so the camera sees the same
 * honest framing across both onboarding fakes. Detection is store-
 * driven (`findDemoWorker`) — the URL stays clean of demo flags.
 */
function DemoStagedHireBanner({ workerId }: { workerId: string }) {
  const [dismissed, setDismissed] = useState(false);
  const [isDemoHire, setIsDemoHire] = useState(false);

  useEffect(() => {
    if (!isDemoMode()) return;
    setIsDemoHire(Boolean(findDemoWorker(workerId)));
  }, [workerId]);

  if (!isDemoHire || dismissed) return null;

  return (
    <div
      role="status"
      data-testid="demo-staged-hire-banner"
      className="flex items-start gap-3 rounded-[var(--radius-lg)] border border-emerald-300/60 bg-emerald-50/70 px-4 py-3 text-emerald-900"
    >
      <Sparkles aria-hidden className="mt-0.5 size-5 shrink-0" />
      <div className="flex-1 text-sm">
        <p className="font-medium">
          Demo · staged hire
        </p>
        <p className="text-emerald-900/80">
          Real provisioning runs against a live workspace. Your description
          is saved as the persona bio.
        </p>
      </div>
      <button
        type="button"
        onClick={() => setDismissed(true)}
        aria-label="Dismiss"
        data-testid="demo-staged-hire-dismiss"
        className="text-emerald-900/60 hover:text-emerald-900"
      >
        <X aria-hidden className="size-4" />
      </button>
    </div>
  );
}

const JUST_HIRED_KEY = "skycot:just-hired";

/**
 * One-shot callout shown immediately after the first-hire wizard
 * redirects here. The first-hire page writes the hired workerId into
 * `sessionStorage[skycot:just-hired]`; we read + clear it on mount and
 * surface a soft CTA deep-linking to the Responsibilities/Schedules
 * card below. Closes the promise gap from the hire-step copy that no
 * longer mentions "this week" — see brief
 * fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed.
 */
function JustHiredScheduleCallout({ workerId }: { workerId: string }) {
  const [show, setShow] = useState(false);

  useEffect(() => {
    let flagged: string | null = null;
    try {
      flagged = sessionStorage.getItem(JUST_HIRED_KEY);
      if (flagged === workerId) sessionStorage.removeItem(JUST_HIRED_KEY);
    } catch {
      /* storage disabled — never show callout */
      return;
    }
    if (flagged === workerId) setShow(true);
  }, [workerId]);

  if (!show) return null;

  const scrollToSchedules = () => {
    const card = document.querySelector('[data-testid="card-duties"]');
    if (card instanceof HTMLElement) {
      card.scrollIntoView({ behavior: "smooth", block: "start" });
      const addBtn = card.querySelector<HTMLButtonElement>(
        '[data-testid="duties-add"]',
      );
      addBtn?.focus({ preventScroll: true });
    }
  };

  return (
    <div
      role="status"
      data-testid="post-hire-schedule-callout"
      className="flex items-start gap-3 rounded-[var(--radius-lg)] border border-[var(--primary)]/30 bg-[var(--primary)]/5 px-4 py-3"
    >
      <CalendarClock
        aria-hidden
        className="mt-0.5 size-5 shrink-0 text-[var(--primary)]"
      />
      <div className="flex-1 text-sm">
        <p className="font-medium text-[var(--foreground)]">
          They're hired. Now set their schedule.
        </p>
        <p className="text-[var(--muted-foreground)]">
          Tell them when to run — daily standups, weekly digests, anything on a
          cadence — in the Responsibilities card below.
        </p>
        <button
          type="button"
          onClick={scrollToSchedules}
          data-testid="post-hire-schedule-cta"
          className="mt-1 inline-flex items-center text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
        >
          Add their first schedule →
        </button>
      </div>
      <button
        type="button"
        onClick={() => setShow(false)}
        aria-label="Dismiss"
        data-testid="post-hire-schedule-dismiss"
        className="text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
      >
        <X aria-hidden className="size-4" />
      </button>
    </div>
  );
}
