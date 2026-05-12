"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useMemo, type ChangeEvent } from "react";
import { ChevronDown, Plus, Search, Users } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { TeammateGrid } from "@/components/skycot/TeammateGrid";
import { TeammateCardSkeleton } from "@/components/skycot/TeammateCardSkeleton";
import { useTeam } from "@/lib/hooks/use-team";
import {
  WORKER_STATUSES,
  WORKER_STATUS_LABELS,
  compareWorkersForGrid,
  type Worker,
  type WorkerStatus,
} from "@/lib/types/worker";

const LOADING_SKELETON_COUNT = 6;

function isWorkerStatus(value: string): value is WorkerStatus {
  return (WORKER_STATUSES as readonly string[]).includes(value);
}

function readStatusParam(value: string | null): WorkerStatus | null {
  if (!value) return null;
  return isWorkerStatus(value) ? value : null;
}

function readQueryParam(value: string | null): string {
  return (value ?? "").trim();
}

function workerMatchesQuery(worker: Worker, query: string): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    worker.name.toLowerCase().includes(q) ||
    worker.role.tagline.toLowerCase().includes(q)
  );
}

export default function TeamPage() {
  return (
    <Suspense fallback={<TeamPageFallback />}>
      <TeamPageContent />
    </Suspense>
  );
}

function TeamPageFallback() {
  return (
    <>
      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
          <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">Team</h1>
        </div>
      </div>
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
        <LoadingGrid />
      </div>
    </>
  );
}

function TeamPageContent() {
  const teamQuery = useTeam();
  const router = useRouter();
  const pathname = usePathname() ?? "/team";
  const searchParams = useSearchParams();

  const query = readQueryParam(searchParams?.get("q") ?? null);
  const status = readStatusParam(searchParams?.get("status") ?? null);

  const updateParam = useCallback(
    (key: "q" | "status", next: string | null) => {
      const params = new URLSearchParams(searchParams?.toString() ?? "");
      if (next && next.length > 0) params.set(key, next);
      else params.delete(key);
      const qs = params.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [pathname, router, searchParams],
  );

  const onSearchChange = (e: ChangeEvent<HTMLInputElement>) =>
    updateParam("q", e.target.value);
  const onStatusChange = (e: ChangeEvent<HTMLSelectElement>) =>
    updateParam("status", e.target.value === "all" ? null : e.target.value);

  const workers = teamQuery.data?.workers ?? [];
  const total = workers.length;

  const filtered = useMemo<Worker[]>(() => {
    const list = workers
      .filter((w) => (status ? w.status === status : true))
      .filter((w) => workerMatchesQuery(w, query));
    return [...list].sort(compareWorkersForGrid);
  }, [workers, query, status]);

  return (
    <>
      <PageHeader
        total={total}
        query={query}
        status={status}
        onSearchChange={onSearchChange}
        onStatusChange={onStatusChange}
      />

      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
        {teamQuery.isPending ? (
          <LoadingGrid />
        ) : total === 0 ? (
          <EmptyTeamState />
        ) : filtered.length === 0 ? (
          <NoMatchesState query={query} status={status} />
        ) : (
          <TeammateGrid>
            {filtered.map((w) => (
              <TeammateCardSkeleton key={w.id} data-testid={`teammate-${w.id}`} />
            ))}
          </TeammateGrid>
        )}
      </div>
    </>
  );
}

interface PageHeaderProps {
  total: number;
  query: string;
  status: WorkerStatus | null;
  onSearchChange: (e: ChangeEvent<HTMLInputElement>) => void;
  onStatusChange: (e: ChangeEvent<HTMLSelectElement>) => void;
}

function PageHeader({
  total,
  query,
  status,
  onSearchChange,
  onStatusChange,
}: PageHeaderProps) {
  return (
    <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Team
            </h1>
            {total > 0 ? (
              <p
                className="text-sm text-[var(--muted-foreground)]"
                data-testid="team-subtitle"
              >
                {total} {total === 1 ? "teammate" : "teammates"} working
              </p>
            ) : null}
          </div>

          <div className="flex items-center gap-1.5 sm:gap-2">
            <StatusFilter value={status} onChange={onStatusChange} />
            <Link
              href="/hire"
              aria-label="Hire teammate"
              data-testid="team-hire-cta"
              className={buttonVariants({ variant: "primary", size: "md" }) + " shadow-sm"}
            >
              <Plus />
              <span className="hidden sm:inline">Hire</span>
            </Link>
          </div>
        </div>

        <div className="mt-4 relative">
          <Search
            aria-hidden
            className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]"
          />
          <input
            type="search"
            value={query}
            onChange={onSearchChange}
            placeholder="Filter by name..."
            aria-label="Filter teammates by name"
            data-testid="team-search-input"
            className="w-full h-10 pl-9 pr-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
          />
        </div>
      </div>
    </div>
  );
}

function StatusFilter({
  value,
  onChange,
}: {
  value: WorkerStatus | null;
  onChange: (e: ChangeEvent<HTMLSelectElement>) => void;
}) {
  return (
    <label className="relative inline-flex items-center text-sm">
      <span className="sr-only">Filter by status</span>
      <select
        value={value ?? "all"}
        onChange={onChange}
        data-testid="team-status-filter"
        aria-label="Status"
        className="appearance-none h-9 pl-3 pr-8 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
      >
        <option value="all">Status</option>
        {WORKER_STATUSES.map((s) => (
          <option key={s} value={s}>
            {WORKER_STATUS_LABELS[s]}
          </option>
        ))}
      </select>
      <ChevronDown
        aria-hidden
        className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]"
      />
    </label>
  );
}

function LoadingGrid() {
  return (
    <TeammateGrid>
      {Array.from({ length: LOADING_SKELETON_COUNT }, (_, i) => (
        <TeammateCardSkeleton key={i} data-testid={`teammate-loading-${i}`} />
      ))}
    </TeammateGrid>
  );
}

function EmptyTeamState() {
  return (
    <div
      data-testid="team-empty-state"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-4"
    >
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]">
        <Users />
      </div>
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          No teammates yet.
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          What do you need help with day-to-day?
        </p>
      </div>
      <div className="flex flex-col items-center gap-1.5">
        <Link
          href="/hire"
          data-testid="team-empty-hire-cta"
          className={buttonVariants({ variant: "primary", size: "md" }) + " shadow-sm"}
        >
          <Plus />
          <span>Hire your first teammate</span>
        </Link>
        <p className="text-xs text-[var(--muted-foreground)]">
          It takes about 5 minutes.
        </p>
      </div>
    </div>
  );
}

function NoMatchesState({
  query,
  status,
}: {
  query: string;
  status: WorkerStatus | null;
}) {
  if (query) {
    return (
      <div
        data-testid="team-no-search-matches"
        className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-2"
      >
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          No matching teammates
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Nothing matches &ldquo;{query}&rdquo;. Try a different search or clear the filter.
        </p>
      </div>
    );
  }

  return (
    <div
      data-testid="team-no-status-matches"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center"
    >
      <p className="text-sm text-[var(--muted-foreground)]">
        No teammates in this state right now.
        {status ? ` (${WORKER_STATUS_LABELS[status]})` : null}
      </p>
    </div>
  );
}
