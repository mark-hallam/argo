"use client";

import { use, type ReactNode } from "react";
import { AlertCircle, Loader2 } from "lucide-react";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { WorkerHero, type WorkerHeroAction } from "@/components/skycot/WorkerHero";
import { WorkerTabs } from "@/components/skycot/WorkerTabs";
import { WorkerNotFound } from "@/components/skycot/WorkerNotFound";
import { useWorker } from "@/lib/hooks/use-worker";
import { WorkerNotFoundError } from "@/lib/api/workers";

/**
 * Worker-scoped layout under /(app)/workers/[workerId]. Fetches the
 * worker once (TanStack Query caches it; sub-tabs read the same
 * cache via `useWorker(id)`), renders breadcrumb + hero + tabs, then
 * delegates body to the tab page.
 *
 * The AppShell ancestor renders the sidebar; this layout sits inside
 * it and overrides nothing.
 */
export default function WorkerLayout({
  children,
  params,
}: {
  children: ReactNode;
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  const workerQuery = useWorker(workerId);

  if (workerQuery.isPending) return <LoadingState />;

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

  return (
    <PageFrame breadcrumb={<TeamBreadcrumb workerName={worker.name} />}>
      <WorkerHero worker={worker} onAction={handleHeroAction} />
      <WorkerTabs workerId={workerId} />
      <div className="pt-6">{children}</div>
    </PageFrame>
  );
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
