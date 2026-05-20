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
import { TeamEmptyState } from "@/components/skycot/TeamEmptyState";
import { TeammateCard } from "@/components/skycot/TeammateCard";
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
          <h3 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">Team</h3>
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

  // Team hero stats — surface aggregate workforce KPIs in the
  // brand-loud register the C2 holographic-showcase design calls for.
  // Static derivations from the loaded roster; richer aggregations
  // (real action counts, actual hours-saved) wire in a follow-up.
  const workingCount = workers.filter((w) => w.status === "working").length;
  const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
  const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;

  return (
    <>
      <TeamHero
        total={total}
        working={workingCount}
        weeklyActions={weeklyActions}
        hoursSaved={hoursSaved}
      />
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
              <TeammateCard key={w.id} worker={w} />
            ))}
          </TeammateGrid>
        )}
      </div>
    </>
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
            <h3 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Team
            </h3>
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

/**
 * Team hero — C2 holographic-showcase brand-loud banner.
 *
 * Renders above the filter/list header. Provides the "your AI workforce
 * is alive" register the locked C2 design memory calls for: gradient
 * banner, Audiowide hero, weight-900 stat numbers, animated shine across
 * the gradient. Three derived KPIs (active, weekly actions, hours saved)
 * are surfaced; richer aggregations land in a follow-up.
 */
function TeamHero({
  total,
  working,
  weeklyActions,
  hoursSaved,
}: {
  total: number;
  working: number;
  weeklyActions: number;
  hoursSaved: number;
}) {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .team-hero {
          position: relative;
          margin: 24px 16px 0;
          padding: 28px 32px;
          border-radius: 18px;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 50%, #ff6b6b 100%);
          color: white;
          overflow: hidden;
          box-shadow: 0 16px 40px -16px rgba(139, 92, 246, 0.45);
        }
        .team-hero::before {
          content: "";
          position: absolute;
          inset: 0;
          background: linear-gradient(
            115deg,
            transparent 30%,
            rgba(255, 255, 255, 0.18) 50%,
            transparent 70%
          );
          transform: translateX(-100%);
          animation: team-hero-shine 4.5s ease-in-out infinite;
          pointer-events: none;
        }
        @keyframes team-hero-shine {
          0%, 80%, 100% {
            transform: translateX(-100%);
          }
          90% {
            transform: translateX(100%);
          }
        }
        .team-hero-grid {
          position: relative;
          display: flex;
          align-items: flex-end;
          justify-content: space-between;
          gap: 32px;
          flex-wrap: wrap;
        }
        .team-hero-left {
          flex: 1;
          min-width: 260px;
        }
        .team-hero-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          color: rgba(255, 255, 255, 0.85);
          padding: 4px 12px;
          border-radius: 999px;
          background: rgba(255, 255, 255, 0.16);
          backdrop-filter: blur(8px);
          margin-bottom: 14px;
        }
        .team-hero-eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: #f0fdf4;
          box-shadow: 0 0 6px rgba(255, 255, 255, 0.9);
          animation: team-hero-pulse 1.6s ease-in-out infinite;
        }
        @keyframes team-hero-pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.45; }
        }
        .team-hero-title {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 36px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0.005em;
          margin: 0;
          color: white;
        }
        .team-hero-sub {
          margin-top: 8px;
          font-size: 14px;
          font-weight: 400;
          color: rgba(255, 255, 255, 0.85);
          max-width: 420px;
        }
        .team-hero-kpis {
          display: flex;
          gap: 28px;
        }
        .team-kpi {
          text-align: right;
        }
        .team-kpi-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 600;
          color: rgba(255, 255, 255, 0.75);
          margin-bottom: 4px;
        }
        .team-kpi-value {
          font-family: "Inter", sans-serif;
          font-size: 36px;
          font-weight: 900;
          letter-spacing: -0.02em;
          color: white;
          line-height: 1;
        }
        @media (max-width: 720px) {
          .team-hero {
            margin: 16px 12px 0;
            padding: 22px 22px;
          }
          .team-hero-title {
            font-size: 28px;
          }
          .team-hero-kpis {
            gap: 18px;
            width: 100%;
            justify-content: flex-start;
            margin-top: 16px;
          }
          .team-kpi {
            text-align: left;
          }
          .team-kpi-value {
            font-size: 28px;
          }
        }
      `}</style>
      <div className="team-hero">
        <div className="team-hero-grid">
          <div className="team-hero-left">
            <div className="team-hero-eyebrow">
              <span className="team-hero-eyebrow-dot" />
              YOUR AI WORKFORCE · LIVE
            </div>
            <h1 className="team-hero-title">
              {total === 0
                ? "Hire your first teammate."
                : total === 1
                  ? "Your first teammate is working."
                  : `${total} teammates, one team.`}
            </h1>
            <p className="team-hero-sub">
              {total === 0
                ? "No teammates yet. Pick a vertical and they'll be in your Slack in four minutes."
                : `${working} working right now · ${weeklyActions.toLocaleString()} actions this week · saving you ~${hoursSaved} hours.`}
            </p>
          </div>
          {total > 0 ? (
            <div className="team-hero-kpis">
              <div className="team-kpi">
                <div className="team-kpi-label">Working</div>
                <div className="team-kpi-value">{working}</div>
              </div>
              <div className="team-kpi">
                <div className="team-kpi-label">Actions/wk</div>
                <div className="team-kpi-value">{weeklyActions.toLocaleString()}</div>
              </div>
              <div className="team-kpi">
                <div className="team-kpi-label">Hours saved</div>
                <div className="team-kpi-value">{hoursSaved}</div>
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </>
  );
}
