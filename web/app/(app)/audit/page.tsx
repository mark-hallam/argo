"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Filter, Loader2 } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import { AuditDayGroup } from "@/components/skycot/audit/AuditDayGroup";
import { AuditFilterChipRail } from "@/components/skycot/audit/AuditFilterChipRail";
import { AuditFilterPopover } from "@/components/skycot/audit/AuditFilterPopover";
import { AuditPageHeader } from "@/components/skycot/audit/AuditPageHeader";
import { EvidenceBundleExporter } from "@/components/skycot/audit/EvidenceBundleExporter";
import { groupByDay } from "@/lib/api/audit";
import { useAuditFeed } from "@/lib/hooks/use-audit-feed";
import {
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  type AuditFilters,
  type AuditRangePreset,
} from "@/lib/types/audit";
import {
  AUDIT_FILTER_DIMENSIONS,
  EMPTY_AUDIT_FILTERS,
  applyAuditMultiFilters,
  auditApproverOptions,
  auditWorkerOptions,
  clearAuditDimension,
  isAuditFilterEmpty,
  parseAuditMultiFilters,
  serializeAuditMultiFilters,
  toggleAuditFilterValue,
  type AuditMultiFilters,
} from "@/lib/filters/audit";

const SEARCH_DEBOUNCE_MS = 200;
const FILTER_DEBOUNCE_MS = 250;

/**
 * `/audit` — cross-worker forensic timeline. URL is the source of
 * truth for filters (`?range=`, `?q=`, and the 7 multi-select
 * dimensions threaded through `web/lib/filters/audit.ts`). `useAuditFeed`
 * only re-queries on `range` + `q`; the multi-select dimensions are
 * applied client-side. Server-side push for scopes > 5000 rows is a
 * Track-H hardening brief (`mission-enterprise-audit-server-filter`).
 *
 * E5-c implements `/audit/action/<id>` (this brief routes to it).
 * E5-e implements the export dialog (placeholder toast for now).
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
  const range = useMemo(() => parseRange(searchParams), [searchParams]);
  const q = useMemo(() => searchParams?.get("q") ?? undefined, [searchParams]);
  const multiFilters = useMemo(
    () => parseAuditMultiFilters(searchParams ?? new URLSearchParams()),
    [searchParams],
  );
  const feedFilters: AuditFilters = useMemo(() => ({ range, q }), [range, q]);
  const [searchInput, setSearchInput] = useState(q ?? "");
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [pendingFilters, setPendingFilters] = useState<AuditMultiFilters | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [exporterOpen, setExporterOpen] = useState(false);
  const feed = useAuditFeed(feedFilters);

  // Sync local search input with URL on back/forward navigation.
  useEffect(() => {
    setSearchInput(q ?? "");
  }, [q]);

  // Debounce-write search input back to the URL.
  useEffect(() => {
    const handle = window.setTimeout(() => {
      if ((q ?? "") === searchInput) return;
      writeSingleParam(router, pathname, searchParams, "q", searchInput || null);
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [searchInput, q, router, pathname, searchParams]);

  // Debounce-write pending multi-filter changes to the URL.
  useEffect(() => {
    if (!pendingFilters) return;
    const handle = window.setTimeout(() => {
      writeMultiFilters(router, pathname, searchParams, pendingFilters);
      setPendingFilters(null);
    }, FILTER_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [pendingFilters, router, pathname, searchParams]);

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
      writeSingleParam(router, pathname, searchParams, "range", param);
    },
    [router, pathname, searchParams],
  );

  const onToggleFilter = useCallback(
    <K extends keyof AuditMultiFilters>(
      dimension: K,
      value: AuditMultiFilters[K][number],
    ) => {
      setPendingFilters((prev) =>
        toggleAuditFilterValue(prev ?? multiFilters, dimension, value),
      );
    },
    [multiFilters],
  );

  const onClearDimension = useCallback(
    (dimension: keyof AuditMultiFilters) => {
      const next = clearAuditDimension(pendingFilters ?? multiFilters, dimension);
      setPendingFilters(null);
      writeMultiFilters(router, pathname, searchParams, next);
    },
    [pendingFilters, multiFilters, router, pathname, searchParams],
  );

  const onClearAllFilters = useCallback(() => {
    setPendingFilters(null);
    writeMultiFilters(router, pathname, searchParams, EMPTY_AUDIT_FILTERS);
  }, [router, pathname, searchParams]);

  const liveFilters = pendingFilters ?? multiFilters;
  const remoteRows = useMemo(
    () => feed.data?.pages.flatMap((p) => p.rows) ?? [],
    [feed.data],
  );
  const rows = useMemo(
    () => applyAuditMultiFilters(remoteRows, multiFilters),
    [remoteRows, multiFilters],
  );
  const workers = useMemo(() => auditWorkerOptions(remoteRows), [remoteRows]);
  const approvers = useMemo(() => auditApproverOptions(remoteRows), [remoteRows]);
  const buckets = useMemo(() => groupByDay(rows), [rows]);
  const totalPages = feed.data?.pages[0]?.totalPages;
  const currentPageCount = feed.data?.pages.length ?? 0;
  const isPristine = isPristineState(range, q, multiFilters);
  const hasActiveFilters = !isAuditFilterEmpty(liveFilters);

  return (
    <>
      <AuditPageHeader
        range={range}
        query={searchInput}
        onRangeChange={onRangeChange}
        onQueryChange={setSearchInput}
        onExportClick={() => setExporterOpen(true)}
        filterTrigger={
          <AuditFilterPopover
            rows={remoteRows}
            filters={liveFilters}
            onToggle={onToggleFilter}
            onClearAll={onClearAllFilters}
            open={popoverOpen}
            onOpenChange={setPopoverOpen}
          >
            <FilterTriggerButton hasActive={hasActiveFilters} />
          </AuditFilterPopover>
        }
      />

      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-4">
        <AuditFilterChipRail
          filters={liveFilters}
          workers={workers}
          approvers={approvers}
          onClearDimension={onClearDimension}
          onClearAll={onClearAllFilters}
        />

        {feed.isPending ? (
          <LoadingState />
        ) : feed.isError ? (
          <ErrorState
            message={feed.error?.message ?? "Couldn't load audit"}
            onRetry={() => feed.refetch()}
          />
        ) : rows.length === 0 ? (
          <EmptyState
            isPristine={isPristine}
            filters={multiFilters}
            onClearAll={onClearAllFilters}
          />
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

      {exporterOpen ? (
        <EvidenceBundleExporter
          open
          onOpenChange={setExporterOpen}
          tenantId="tenant_default"
          tenantLabel="Default tenant"
          initialScope={{ kind: "filtered" }}
          onToast={setToast}
        />
      ) : null}
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

function FilterTriggerButton({ hasActive }: { hasActive: boolean }) {
  return (
    <button
      type="button"
      aria-label="Open filter popover"
      data-testid="audit-filter-trigger"
      className={cn(
        buttonVariants({ variant: "secondary", size: "sm" }),
        "relative px-2",
      )}
    >
      <Filter aria-hidden className="size-4" />
      <span className="sr-only">Filter</span>
      {hasActive ? (
        <span
          data-testid="audit-filter-trigger-dot"
          aria-hidden
          className="absolute -top-0.5 -right-0.5 inline-block size-2 rounded-full bg-[var(--primary)]"
        />
      ) : null}
    </button>
  );
}

function isPristineState(
  range: AuditRangePreset,
  q: string | undefined,
  multi: AuditMultiFilters,
): boolean {
  return range === DEFAULT_AUDIT_RANGE && !q && isAuditFilterEmpty(multi);
}

function parseRange(params: URLSearchParams | null): AuditRangePreset {
  const raw = params?.get("range") ?? "";
  return (AUDIT_RANGE_PRESETS as readonly string[]).includes(raw)
    ? (raw as AuditRangePreset)
    : DEFAULT_AUDIT_RANGE;
}

function writeSingleParam(
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

function writeMultiFilters(
  router: ReturnType<typeof useRouter>,
  pathname: string,
  searchParams: URLSearchParams | null,
  filters: AuditMultiFilters,
) {
  const params = new URLSearchParams(searchParams?.toString() ?? "");
  for (const dim of AUDIT_FILTER_DIMENSIONS) {
    params.delete(urlKey(dim));
  }
  const next = new URLSearchParams(serializeAuditMultiFilters(filters));
  next.forEach((value, key) => params.set(key, value));
  const qs = params.toString();
  router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
}

function urlKey(dim: keyof AuditMultiFilters): string {
  switch (dim) {
    case "actionType":
      return "action_type";
    case "costBand":
      return "cost_band";
    default:
      return dim;
  }
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

function EmptyState({
  isPristine,
  filters,
  onClearAll,
}: {
  isPristine: boolean;
  filters: AuditMultiFilters;
  onClearAll: () => void;
}) {
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
  const detail = filteredEmptyDetail(filters);
  return (
    <div
      data-testid="audit-empty-filtered"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">No actions match</p>
      <p className="text-sm text-[var(--muted-foreground)]">{detail}</p>
      <button
        type="button"
        onClick={onClearAll}
        data-testid="audit-empty-clear-filters"
        className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Clear filters
      </button>
    </div>
  );
}

function filteredEmptyDetail(filters: AuditMultiFilters): string {
  const active: string[] = [];
  if (filters.worker.length > 0) active.push("worker");
  if (filters.channel.length > 0) active.push("channel");
  if (filters.actionType.length > 0) active.push("action type");
  if (filters.status.length > 0) active.push("status");
  if (filters.approver.length > 0) active.push("approver");
  if (filters.costBand.length > 0) active.push("cost band");
  if (filters.classification.length > 0) active.push("classification");
  if (active.length === 0) {
    return "Try widening the time range or clearing search.";
  }
  if (active.length === 1) {
    return `Try removing the ${active[0]} filter or widening the time range.`;
  }
  return `Try removing one of the ${active.length} active filters or widening the time range.`;
}
