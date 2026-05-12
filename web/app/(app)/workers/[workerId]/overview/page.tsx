"use client";

import { use } from "react";
import { WorkerOverviewContent } from "@/components/skycot/worker/WorkerOverviewContent";
import { RecentChangesPanel } from "@/components/skycot/worker/RecentChangesPanel";

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
      <WorkerOverviewContent workerId={workerId} />
      <RecentChangesPanel workerId={workerId} />
    </div>
  );
}
