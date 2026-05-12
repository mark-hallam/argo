import { notFound } from "next/navigation";
import {
  WORKER_TAB_LABELS,
  isWorkerTabId,
  type WorkerTabId,
} from "@/components/skycot/WorkerTabs";

/**
 * Dynamic per-tab page under `/workers/<id>/<tab>`. Renders a thin
 * placeholder pointing at the brief that will fill the tab; the
 * layout above already painted the hero + tab strip.
 *
 * Invalid tab segments fall through `notFound()` → `not-found.tsx`
 * (rendered inside the AppShell).
 */

const TAB_BRIEFS: Record<WorkerTabId, string> = {
  overview: "E4-b",
  activity: "E4-c",
  memory: "E4-d",
  skills: "E4-e",
  trust: "mission-trust-grade-ui",
  settings: "E4-f",
};

export default async function WorkerTabPage({
  params,
}: {
  params: Promise<{ workerId: string; tab: string }>;
}) {
  const { tab } = await params;
  if (!isWorkerTabId(tab)) notFound();
  const tabId = tab;
  const label = WORKER_TAB_LABELS[tabId];
  const brief = TAB_BRIEFS[tabId];

  return (
    <div
      role="tabpanel"
      aria-label={`${label} tab`}
      data-testid={`worker-tab-panel-${tabId}`}
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center"
    >
      <p className="text-sm text-[var(--muted-foreground)]">
        {label} — coming in {brief}.
      </p>
    </div>
  );
}
