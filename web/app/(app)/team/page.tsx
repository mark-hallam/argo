"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ChangeEvent,
} from "react";
import { ChevronDown, Plus, Search, X } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { Toast, ToastProvider, ToastViewport } from "@/components/ui/Toast";
import { TeamEmptyState } from "@/components/skycot/TeamEmptyState";
import { TeammateCard, type TeammateCardToast } from "@/components/skycot/TeammateCard";
import { TeammateGrid } from "@/components/skycot/TeammateGrid";
import { TeammateCardSkeleton } from "@/components/skycot/TeammateCardSkeleton";
import { TeamFilterChipRail } from "@/components/skycot/TeamFilterChipRail";
import { useTeam } from "@/lib/hooks/use-team";
import {
  applyTeamFilters,
  parseTeamFilters,
  type TeamFilters,
} from "@/lib/filters/team";
import {
  WORKER_STATUSES,
  WORKER_STATUS_LABELS,
  compareWorkersForGrid,
  type Worker,
  type WorkerStatus,
} from "@/lib/types/worker";

const LOADING_SKELETON_COUNT = 6;
const URL_DEBOUNCE_MS = 150;

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
  const [toasts, setToasts] = useState<TeammateCardToast[]>([]);

  const showToast = useCallback((toast: TeammateCardToast) => {
    setToasts((prev) => [...prev, toast]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const filters = useMemo<TeamFilters>(
    () => parseTeamFilters(searchParams ?? new URLSearchParams("")),
    [searchParams],
  );

  // Local input mirror — updates immediately so typing feels responsive,
  // then the effect below pushes the trimmed value into the URL after a
  // 150ms debounce. Back/forward navigation flows through `filters.q`.
  const [inputQuery, setInputQuery] = useState(filters.q);

  useEffect(() => {
    setInputQuery(filters.q);
  }, [filters.q]);

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

  useEffect(() => {
    const trimmed = inputQuery.trim();
    if (trimmed === filters.q) return;
    const t = setTimeout(
      () => updateParam("q", trimmed.length > 0 ? trimmed : null),
      URL_DEBOUNCE_MS,
    );
    return () => clearTimeout(t);
  }, [inputQuery, filters.q, updateParam]);

  const onSearchChange = (e: ChangeEvent<HTMLInputElement>) =>
    setInputQuery(e.target.value);
  const onSearchClear = () => {
    setInputQuery("");
    updateParam("q", null);
  };
  const onStatusChange = (e: ChangeEvent<HTMLSelectElement>) =>
    updateParam("status", e.target.value === "all" ? null : e.target.value);

  const onClearDimension = useCallback(
    (dimension: keyof TeamFilters) => {
      if (dimension === "status") {
        updateParam("status", null);
      } else {
        setInputQuery("");
        updateParam("q", null);
      }
    },
    [updateParam],
  );

  const onClearAll = useCallback(() => {
    setInputQuery("");
    const params = new URLSearchParams(searchParams?.toString() ?? "");
    params.delete("q");
    params.delete("status");
    const qs = params.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
  }, [pathname, router, searchParams]);

  const workers = teamQuery.data?.workers ?? [];
  const total = workers.length;

  const filtered = useMemo<Worker[]>(
    () => [...applyTeamFilters(workers, filters)].sort(compareWorkersForGrid),
    [workers, filters],
  );

  return (
    <ToastProvider swipeDirection="right" duration={5000}>
      <PageHeader
        total={total}
        filteredCount={filtered.length}
        filters={filters}
        inputQuery={inputQuery}
        onSearchChange={onSearchChange}
        onSearchClear={onSearchClear}
        onStatusChange={onStatusChange}
        onClearDimension={onClearDimension}
        onClearAll={onClearAll}
      />

      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
        {teamQuery.isPending ? (
          <LoadingGrid />
        ) : total === 0 ? (
          <TeamEmptyState />
        ) : filtered.length === 0 ? (
          <NoMatchesState filters={filters} onClearAll={onClearAll} />
        ) : (
          <TeammateGrid>
            {filtered.map((w) => (
              <TeammateCard key={w.id} worker={w} onShowToast={showToast} />
            ))}
          </TeammateGrid>
        )}
      </div>

      {toasts.map((t) => (
        <Toast
          key={t.id}
          tone={t.tone}
          title={t.title}
          data-testid={`team-toast-${t.id}`}
          onOpenChange={(open) => {
            if (!open) dismissToast(t.id);
          }}
        />
      ))}
      <ToastViewport />
    </ToastProvider>
  );
}

interface PageHeaderProps {
  total: number;
  filteredCount: number;
  filters: TeamFilters;
  inputQuery: string;
  onSearchChange: (e: ChangeEvent<HTMLInputElement>) => void;
  onSearchClear: () => void;
  onStatusChange: (e: ChangeEvent<HTMLSelectElement>) => void;
  onClearDimension: (dimension: keyof TeamFilters) => void;
  onClearAll: () => void;
}

function PageHeader({
  total,
  filteredCount,
  filters,
  inputQuery,
  onSearchChange,
  onSearchClear,
  onStatusChange,
  onClearDimension,
  onClearAll,
}: PageHeaderProps) {
  const filterActive = filters.status !== null || filters.q.length > 0;
  const subtitleCount = filterActive ? filteredCount : total;
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
                {subtitleCount} {subtitleCount === 1 ? "teammate" : "teammates"} working
              </p>
            ) : null}
          </div>

          <div className="flex items-center gap-1.5 sm:gap-2">
            <StatusFilter value={filters.status} onChange={onStatusChange} />
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
            type="text"
            value={inputQuery}
            onChange={onSearchChange}
            placeholder="Filter by name..."
            aria-label="Filter teammates by name"
            data-testid="team-search-input"
            className="w-full h-10 pl-9 pr-9 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
          />
          {inputQuery.length > 0 ? (
            <button
              type="button"
              onClick={onSearchClear}
              aria-label="Clear search"
              data-testid="team-search-clear"
              className="absolute right-2 top-1/2 -translate-y-1/2 grid size-6 place-items-center rounded-full text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--muted)] transition-colors"
            >
              <X className="size-3.5" />
            </button>
          ) : null}
        </div>

        <TeamFilterChipRail
          filters={filters}
          onClearDimension={onClearDimension}
          onClearAll={onClearAll}
        />
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
  const active = value !== null;
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
      {active ? (
        <span
          aria-hidden
          data-testid="team-status-filter-dot"
          className="pointer-events-none absolute -top-0.5 -right-0.5 size-2 rounded-full bg-[var(--primary)] ring-2 ring-[var(--background)]"
        />
      ) : null}
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

function NoMatchesState({
  filters,
  onClearAll,
}: {
  filters: TeamFilters;
  onClearAll: () => void;
}) {
  if (filters.q.length > 0) {
    return (
      <div
        data-testid="team-no-search-matches"
        className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
      >
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          No matching teammates
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Nothing matches &ldquo;{filters.q}&rdquo;. Try a different search or clear the filter.
        </p>
        <ClearAllButton onClick={onClearAll} />
      </div>
    );
  }

  return (
    <div
      data-testid="team-no-status-matches"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
    >
      <p className="text-sm text-[var(--muted-foreground)]">
        No teammates in this state right now.
        {filters.status ? ` (${WORKER_STATUS_LABELS[filters.status]})` : null}
      </p>
      <ClearAllButton onClick={onClearAll} />
    </div>
  );
}

function ClearAllButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      data-testid="team-no-matches-clear-all"
      className="text-xs text-[var(--primary)] underline-offset-4 hover:underline"
    >
      Clear all filters
    </button>
  );
}
