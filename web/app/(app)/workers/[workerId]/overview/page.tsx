"use client";

import { use } from "react";
import { WorkerOverviewContent } from "@/components/skycot/worker/WorkerOverviewContent";

/**
 * Overview tab — the default sub-route under `/workers/<id>`.
 *
 * The Next.js convention is for route components to receive `params`
 * as a `Promise`. We unwrap it here with `React.use()` and delegate
 * the entire body to `<WorkerOverviewContent />`, which the unit-test
 * harness mounts directly with a plain `workerId` string to avoid
 * Promise-suspense flake in jsdom.
 */
export default function WorkerOverviewPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  return <WorkerOverviewContent workerId={workerId} />;
}
