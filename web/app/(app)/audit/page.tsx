"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Loader2, X } from "lucide-react";
import { AuditDayGroup } from "@/components/skycot/audit/AuditDayGroup";
import { AuditPageHeader } from "@/components/skycot/audit/AuditPageHeader";
import { groupByDay } from "@/lib/api/audit";
import { useAuditFeed } from "@/lib/hooks/use-audit-feed";
import {
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  type AuditFilters,
  type AuditRangePreset,
} from "@/lib/types/audit";

const SEARCH_DEBOUNCE_MS = 200;

/**
 * `/audit` — cross-worker forensic timeline. CONTRACT PR for the E5
 * slice. URL is the source of truth for filters (`?range=`, `?q=`,
 * `?worker=`, `?action_type=`, `?status=`); changing any of them
 * re-derives the filters object and re-keys the infinite-query.
 *
 * E5-b adds the filter popover (this brief renders only the icon
 * trigger). E5-c implements `/audit/action/<id>` (this brief routes
 * to it; until E5-c ships the link returns 404). E5-e implements
 * the export dialog (placeholder toast for now).
 */
export default function AuditPage() {
  return (
    <Suspense fallback={<AuditPageFallback />}>
      <AuditPageContent />
    </Suspense>
  );
}

function AuditPageFallback() {
  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8" data-testid="audit-loading">
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
    </div>
  );
}

function AuditPageContent() {
  const router = useRouter();
  const pathname = usePathname() ?? "/audit";
  const searchParams = useSearchParams();
  const filters = useMemo(() => parseFilters(searchParams), [searchParams]);
  const [searchInput, setSearchInput] = useState(filters.q ?? "");
  const [toast, setToast] = useState<string | null>(null);
  const feed = useAuditFeed(filters);

  // Sync local search input with URL on back/forward navigation.
  useEffect(() => {
    setSearchInput(filters.q ?? "");
  }, [filters.q]);

  // Debounce-write search input back to the URL.
  useEffect(() => {
    const handle = window.setTimeout(() => {
      if ((filters.q ?? "") === searchInput) return;
      writeParam(router, pathname, searchParams, "q", searchInput || null);
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [searchInput, filters.q, router, pathname, searchParams]);

  // Auto-clear placeholder toast after 2.5s.
  useEffect(() => {
    if (!toast) return;
    const handle = window.setTimeout(() => setToast(null), 2500);
    return () => window.clearTimeout(handle);
  }, [toast]);

  const onRangeChange = useCallback(
    (next: AuditRangePreset | "custom") => {
      if (next === "custom") {
        setToast("Custom range picker ships in E5-b");
        return;
      }
      const param = next === DEFAULT_AUDIT_RANGE ? null : next;
      writeParam(router, pathname, searchParams, "range", param);
    },
    [router, pathname, searchParams],
  );

  const clearChip = useCallback(
    (key: "worker" | "action_type" | "status") => {
      writeParam(router, pathname, searchParams, key, null);
    },
    [router, pathname, searchParams],
  );

  const rows = useMemo(
    () => feed.data?.pages.flatMap((p) => p.rows) ?? [],
    [feed.data],
  );
  const buckets = useMemo(() => groupByDay(rows), [rows]);
  const totalPages = feed.data?.pages[0]?.totalPages;
  const currentPageCount = feed.data?.pages.length ?? 0;
  const isPristine = isPristineFilters(filters);
  const chips = activeChips(filters);

  return (
    <>
      <AuditPageHeader
        range={filters.range ?? DEFAULT_AUDIT_RANGE}
        query={searchInput}
        onRangeChange={onRangeChange}
        onQueryChange={setSearchInput}
        onFilterClick={() => setToast("Filter popover ships in E5-b")}
        onExportClick={() => setToast("Export dialog ships in E5-e")}
      />

      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-4">
        {chips.length > 0 ? (
          <div data-testid="audit-chip-rail" className="flex flex-wrap gap-2">
            {chips.map((chip) => (
              <button
                key={chip.key}
                type="button"
                data-testid={`audit-chip-${chip.key}`}
                onClick={() => clearChip(chip.key)}
                className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--card)] px-2.5 py-1 text-xs font-medium text-[var(--foreground)] hover:border-[var(--primary)]/40"
              >
                <span className="text-[var(--muted-foreground)]">{chip.prefix}</span>
                {chip.value}
                <X aria-hidden className="size-3" />
                <span className="sr-only">Remove filter</span>
              </button>
            ))}
          </div>
        ) : null}

        {feed.isPending ? (
          <LoadingState />
        ) : feed.isError ? (
          <ErrorState
            message={feed.error?.message ?? "Couldn't load audit"}
            onRetry={() => feed.refetch()}
          />
        ) : rows.length === 0 ? (
          <EmptyState isPristine={isPristine} />
        ) : (
          <div data-testid="audit-feed" className="space-y-6">
            {buckets.map((bucket) => (
              <AuditDayGroup key={bucket.key} bucket={bucket} />
            ))}
          </div>
        )}

        {feed.hasNextPage ? (
          <div className="flex justify-center pt-2">
            <button
              type="button"
              data-testid="audit-load-more"
              disabled={feed.isFetchingNextPage}
              onClick={() => feed.fetchNextPage()}
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] disabled:cursor-not-allowed disabled:opacity-60"
            >
              {feed.isFetchingNextPage
                ? "Loading…"
                : `Load more — page ${currentPageCount + 1}${totalPages ? ` of ${totalPages}` : ""}`}
            </button>
          </div>
        ) : null}
      </div>

      {toast ? (
        <div
          role="status"
          data-testid="audit-toast"
          className="fixed bottom-6 left-1/2 -translate-x-1/2 z-30 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--foreground)] shadow-md"
        >
          {toast}
        </div>
      ) : null}
    </>
  );
}

interface Chip {
  key: "worker" | "action_type" | "status";
  prefix: string;
  value: string;
}

function activeChips(filters: AuditFilters): Chip[] {
  const out: Chip[] = [];
  if (filters.worker) out.push({ key: "worker", prefix: "Worker:", value: filters.worker });
  if (filters.actionType) out.push({ key: "action_type", prefix: "Type:", value: filters.actionType });
  if (filters.status) out.push({ key: "status", prefix: "Status:", value: filters.status });
  return out;
}

function isPristineFilters(filters: AuditFilters): boolean {
  return (
    (!filters.range || filters.range === DEFAULT_AUDIT_RANGE) &&
    !filters.q &&
    !filters.worker &&
    !filters.actionType &&
    !filters.status
  );
}

function parseFilters(params: URLSearchParams | null): AuditFilters {
  const rawRange = params?.get("range") ?? "";
  const range: AuditRangePreset = (AUDIT_RANGE_PRESETS as readonly string[]).includes(rawRange)
    ? (rawRange as AuditRangePreset)
    : DEFAULT_AUDIT_RANGE;
  return {
    range,
    q: params?.get("q") ?? undefined,
    worker: params?.get("worker") ?? undefined,
    actionType: params?.get("action_type") ?? undefined,
    status: params?.get("status") ?? undefined,
  };
}

function writeParam(
  router: ReturnType<typeof useRouter>,
  pathname: string,
  searchParams: URLSearchParams | null,
  key: string,
  value: string | null,
) {
  const params = new URLSearchParams(searchParams?.toString() ?? "");
  if (value && value.length > 0) params.set(key, value);
  else params.delete(key);
  const qs = params.toString();
  router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
}

function LoadingState() {
  return (
    <div
      data-testid="audit-loading"
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/60 py-10 text-center"
    >
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading audit timeline…</p>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      data-testid="audit-error"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--destructive)]/40 bg-[var(--destructive)]/5 py-8 px-6 text-center space-y-3"
    >
      <p className="text-sm text-[var(--foreground)]">Couldn&apos;t load audit</p>
      <p className="text-xs text-[var(--muted-foreground)]">{message}</p>
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

function EmptyState({ isPristine }: { isPristine: boolean }) {
  if (isPristine) {
    return (
      <div
        data-testid="audit-empty-pristine"
        className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-2"
      >
        <p className="text-sm font-medium text-[var(--foreground)]">Nothing&apos;s happened yet.</p>
        <p className="text-sm text-[var(--muted-foreground)]">
          Once your workers start acting, you&apos;ll see their activity here.
        </p>
      </div>
    );
  }
  return (
    <div
      data-testid="audit-empty-filtered"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-2"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">No actions match</p>
      <p className="text-sm text-[var(--muted-foreground)]">
        Try widening the time range, clearing search, or removing filters.
      </p>
    </div>
  );
}
