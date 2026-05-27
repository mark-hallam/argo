"use client";

import { use } from "react";
import { WorkerSchedulesContent } from "@/components/skycot/worker/WorkerSchedulesContent";

/**
 * Schedules tab — `/workers/<id>/schedules`. Hosts the same duty
 * list the Overview surfaces (via {@link WorkerSchedulesContent}),
 * promoted to its own tab so Scene 2 of the v1.0 demo runbook has
 * a deterministic "click Schedules" target. File-system code keeps
 * the `duty`/`duties` vocabulary; the visible surface reads
 * "Schedules" per the operator-locked vocab rule.
 */
export default function WorkerSchedulesPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  return <WorkerSchedulesContent workerId={workerId} />;
}
