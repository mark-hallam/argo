import type { ReactNode } from "react";
import { auth } from "@/auth";
import { fetchFreezeState } from "./fetchFreezeState";
import { WorkerLayoutShell } from "./WorkerLayoutShell";

/**
 * Worker-scoped layout under /(app)/workers/[workerId]. Runs
 * server-side so it can read the NextAuth session and fetch the
 * worker's freeze state before painting. The actual chrome (hero,
 * tabs, breadcrumb, freeze banner, freeze button) is rendered by
 * the client shell.
 *
 * Freeze state lives here (rather than in the now-redirected index
 * page) so the red banner stays visible on every tab — overview,
 * activity, memory, skills, trust, settings.
 */
export default async function WorkerLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = await params;
  const session = await auth();
  const canFreeze = Boolean(session?.gatewayToken);
  const freezeState = await fetchFreezeState(workerId, session?.gatewayToken);

  return (
    <WorkerLayoutShell
      workerId={workerId}
      canFreeze={canFreeze}
      freezeState={freezeState}
    >
      {children}
    </WorkerLayoutShell>
  );
}
