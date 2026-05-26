"use client";

import { type ReactNode } from "react";
import { AlertCircle, Loader2 } from "lucide-react";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { WorkerHero, type WorkerHeroAction } from "@/components/skycot/WorkerHero";
import { WorkerTabs } from "@/components/skycot/WorkerTabs";
import { WorkerNotFound } from "@/components/skycot/WorkerNotFound";
import { useWorker } from "@/lib/hooks/use-worker";
import { AuthNotReadyError, WorkerNotFoundError } from "@/lib/api/workers";
import { FreezeBanner } from "./FreezeBanner";
import { FreezeButton } from "./FreezeButton";
import type { FreezeState } from "./fetchFreezeState";

/**
 * Client shell rendered by the worker layout. The layout (server
 * component) feeds it the workerId + freeze state it fetched
 * server-side; this shell adds the breadcrumb + freeze banner +
 * hero + tabs that have to wrap every worker-profile sub-route.
 *
 * The worker itself is loaded via `useWorker` — TanStack Query
 * caches it so every tab page reads the same data without
 * refetching. Freeze state lives on the layout so a `router.refresh()`
 * triggered by FreezeButton picks up the updated banner.
 */
interface WorkerLayoutShellProps {
  workerId: string;
  canFreeze: boolean;
  freezeState: FreezeState | null;
  children: ReactNode;
}

export function WorkerLayoutShell({
  workerId,
  canFreeze,
  freezeState,
  children,
}: WorkerLayoutShellProps) {
  const workerQuery = useWorker(workerId);

  // While the cookie-derived bearer token is still hydrating on the
  // client, `fetchWorker` raises AuthNotReadyError and `useWorker`
  // retries. Keep the skeleton up rather than flashing the
  // not-found shell — the recovery is sub-second and the brief's
  // acceptance criteria require we never render "Worker not found"
  // against a real worker on cold navigation.
  const authNotReady = workerQuery.error instanceof AuthNotReadyError;
  if (workerQuery.isPending || authNotReady) return <LoadingState />;

  const notFound =
    workerQuery.error instanceof WorkerNotFoundError ||
    (workerQuery.error as { status?: number } | undefined)?.status === 404;

  if (notFound) {
    return (
      <PageFrame breadcrumb={<TeamBreadcrumb />}>
        <WorkerNotFound workerId={workerId} />
      </PageFrame>
    );
  }

  if (workerQuery.isError || !workerQuery.data) {
    return (
      <PageFrame breadcrumb={<TeamBreadcrumb />}>
        <ErrorState
          message={(workerQuery.error as Error)?.message ?? "Couldn't load the worker"}
          onRetry={() => workerQuery.refetch()}
        />
      </PageFrame>
    );
  }

  const worker = workerQuery.data;
  const isFrozen = freezeState !== null;
  const freezePermissions = {
    can_freeze: readCanFreeze(worker.permissions) ?? canFreeze,
  };

  return (
    <PageFrame breadcrumb={<TeamBreadcrumb workerName={worker.name} />}>
      {freezeState ? <FreezeBanner state={freezeState} /> : null}
      <div className="flex justify-end">
        <FreezeButton
          workerId={workerId}
          isFrozen={isFrozen}
          permissions={freezePermissions}
        />
      </div>
      <WorkerHero worker={worker} onAction={handleHeroAction} />
      <WorkerTabs workerId={workerId} />
      <div className="pt-6">{children}</div>
    </PageFrame>
  );
}

function readCanFreeze(permissions: unknown): boolean | undefined {
  if (!permissions || typeof permissions !== "object") return undefined;
  const source = permissions as { can_freeze?: unknown; canFreeze?: unknown };
  if (typeof source.can_freeze === "boolean") return source.can_freeze;
  if (typeof source.canFreeze === "boolean") return source.canFreeze;
  return undefined;
}

function PageFrame({ children, breadcrumb }: { children: ReactNode; breadcrumb?: ReactNode }) {
  return (
    <>
      {breadcrumb ? (
        <div className="border-b border-[var(--border)] bg-[var(--background)]/85">
          <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-3">{breadcrumb}</div>
        </div>
      ) : null}
      <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-6 space-y-5">{children}</div>
    </>
  );
}

function TeamBreadcrumb({ workerName }: { workerName?: string }) {
  return (
    <Breadcrumb>
      <BreadcrumbItem href="/team">Team</BreadcrumbItem>
      {workerName ? (
        <>
          <BreadcrumbSeparator />
          <BreadcrumbItem current>{workerName}</BreadcrumbItem>
        </>
      ) : null}
    </Breadcrumb>
  );
}

function handleHeroAction(action: WorkerHeroAction) {
  // Pause / disable / archive land in E3-b + E4-f; the menu shape is
  // locked here. Until those briefs ship, log so the hook-up is
  // discoverable in the console.
  // eslint-disable-next-line no-console
  console.warn(`[E4-a] worker hero action '${action}' pending wiring (E3-b/E4-f)`);
}

function LoadingState() {
  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-16 text-center">
      <Loader2 className="size-6 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-3 text-sm text-[var(--muted-foreground)]">Loading worker…</p>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 py-12 px-6 text-center space-y-4">
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle />
      </div>
      <div className="space-y-1">
        <h1 className="text-base font-semibold text-[var(--foreground)]">Couldn&apos;t load worker</h1>
        <p className="text-sm text-[var(--muted-foreground)]">{message}</p>
      </div>
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
