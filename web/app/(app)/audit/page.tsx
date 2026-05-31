"use client";

import { Suspense, useCallback, useEffect, useMemo, useState, type ButtonHTMLAttributes } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Filter, Loader2 } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import { AuditEmpty } from "./AuditEmptyStates";
import { cn } from "@/lib/utils";
import { useToast } from "@/lib/ui/use-toast";
import { AuditDayGroup } from "@/components/skycot/audit/AuditDayGroup";
import { AuditFilterChipRail } from "@/components/skycot/audit/AuditFilterChipRail";
import { AuditFilterPopover } from "@/components/skycot/audit/AuditFilterPopover";
import { AuditChainExplorerHero } from "@/components/skycot/audit/AuditChainExplorerHero";
import { AuditPageHeader } from "@/components/skycot/audit/AuditPageHeader";
import { EvidenceBundleExporter } from "@/components/skycot/audit/EvidenceBundleExporter";
import { RedactionBanner } from "@/components/skycot/audit/RedactionBanner";
import { groupByDay } from "@/lib/api/audit";
import {
  useAuditFeed,
  useAuditWorkspaceFreshness,
  type AuditWorkspaceState,
} from "@/lib/hooks/use-audit-feed";
import { useAuditFeedFilters } from "./use-audit-url-state";
import {
  isPristineState,
  parseRange,
  resolveWorkerSlugs,
  writeMultiFilters,
  writeSingleParam,
} from "./audit-url-writers";
import { useViewerPolicy } from "@/lib/hooks/use-viewer-policy";
import { redactAuditRow } from "@/lib/redaction/redact";
import { categoriesLackedBy } from "@/lib/redaction/audit-rules";
import type { RbacPermission } from "@/lib/redaction/types";
import {
  DEFAULT_AUDIT_RANGE,
  type AuditFilters,
  type AuditRangePreset,
} from "@/lib/types/audit";
import {
  EMPTY_AUDIT_FILTERS,
  applyAuditMultiFilters,
  auditApproverOptions,
  auditWorkerOptions,
  clearAuditDimension,
  isAuditFilterEmpty,
  parseAuditMultiFilters,
  toggleAuditFilterValue,
  type AuditMultiFilters,
} from "@/lib/filters/audit";
import type { UseInfiniteQueryResult, InfiniteData } from "@tanstack/react-query";
import type { AuditFeedPage } from "@/lib/types/audit";

const SEARCH_DEBOUNCE_MS = 200;
const FILTER_DEBOUNCE_MS = 250;

type AuditFeedQuery = UseInfiniteQueryResult<InfiniteData<AuditFeedPage>, Error>;

/**
 * `/audit` — B3 chain-explorer surface over real audit feed rows. URL
 * filters remain source-of-truth; multi-select dimensions are applied
 * client-side until server-side enterprise filtering ships.
 *
 * The feed fetch (`useAuditFeed` + the freshness probe) is deliberately
 * called HERE, in the OUTER component, ABOVE the `useSearchParams()`
 * Suspense boundary below — the same hoist the `/team` Scene-1 fix used
 * (`web/app/(app)/team/page.tsx`). `useSearchParams()` triggers React's
 * client-side-rendering bailout (a suspend) on this route; that boundary
 * sits inside `RouteTransition`'s `<AnimatePresence>`
 * (`lib/motion/primitives.tsx`), and when it suspends during hydration the
 * subtree never commits — so a fetch mounted *inside* it never has its
 * React Query observer subscribe and `queryFn` never fires. `/audit` then
 * showed "Loading audit timeline…" FOREVER with NO `/api/v1/audit` request
 * (verified live: 14s, zero audit requests). Mounting the fetch here — in a
 * component that reads the URL via the non-suspending `useAuditFeedFilters`
 * instead of `useSearchParams` — guarantees it fires on mount regardless of
 * the inner boundary. The search-param-driven filter UI stays inside the
 * boundary (it needs the live URL) and only filters already-fetched rows.
 * Pinned by `web/app/(app)/audit/audit-fetch-fires.test.tsx`.
 */
export default function AuditPage() {
  // Feed filters read WITHOUT `useSearchParams` (see module-level
  // `useAuditFeedFilters` doc) so this component never suspends.
  const feedFilters = useAuditFeedFilters();
  const feed = useAuditFeed(feedFilters);

  // Workspace-freshness probe lives here too so the empty-state copy is
  // driven by an observer that is never gated behind the boundary. We can't
  // cheaply know `rows.length === 0 && isPristine` before the inner view
  // computes it, so probe whenever the primary feed has settled to zero
  // rows on a default (no-`q`) range — the same trigger the inner view used,
  // minus the multi-filter dimensions (which only ever shrink the set).
  const probeFreshness =
    !feed.isPending &&
    !feed.isError &&
    (feed.data?.pages.flatMap((p) => p.rows).length ?? 0) === 0 &&
    feedFilters.range === DEFAULT_AUDIT_RANGE &&
    !feedFilters.q;
  const workspaceState = useAuditWorkspaceFreshness({ enabled: probeFreshness });

  return (
    <Suspense
      fallback={<AuditPageFallback feed={feed} workspaceState={workspaceState} />}
    >
      <AuditPageContent feed={feed} workspaceState={workspaceState} />
    </Suspense>
  );
}

// Suspense fallback — shown only for the brief window while
// `useSearchParams()` resolves. It renders the SAME timeline body the
// committed view would, driven by the already-mounted `feed` query, so the
// audit rows are NEVER gated behind the boundary resolving. Filter chrome is
// the static, non-interactive variant (no URL to read yet).
function AuditPageFallback({
  feed,
  workspaceState,
}: {
  feed: AuditFeedQuery;
  workspaceState: AuditWorkspaceState;
}) {
  const rawRemoteRows = feed.data?.pages.flatMap((p) => p.rows) ?? [];
  const buckets = groupByDay(rawRemoteRows);
  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-8 space-y-4">
      {feed.isPending ? (
        <LoadingState />
      ) : feed.isError ? (
        <AuditUnreachable
          message={feed.error?.message ?? "Couldn't load audit"}
          onRetry={() => feed.refetch()}
        />
      ) : rawRemoteRows.length === 0 ? (
        <AuditEmpty
          isPristine
          filters={EMPTY_AUDIT_FILTERS}
          workspaceState={workspaceState}
          onClearAll={() => {}}
        />
      ) : (
        <div data-testid="audit-feed" className="space-y-6">
          {buckets.map((bucket) => (
            <AuditDayGroup key={bucket.key} bucket={bucket} />
          ))}
        </div>
      )}
    </div>
  );
}

function AuditPageContent({
  feed,
  workspaceState,
}: {
  feed: AuditFeedQuery;
  workspaceState: AuditWorkspaceState;
}) {
  const router = useRouter();
  const pathname = usePathname() ?? "/audit";
  const searchParams = useSearchParams();
  const range = useMemo(() => parseRange(searchParams), [searchParams]);
  const q = useMemo(() => searchParams?.get("q") ?? undefined, [searchParams]);
  const multiFilters = useMemo(
    () => resolveWorkerSlugs(parseAuditMultiFilters(searchParams ?? new URLSearchParams())),
    [searchParams],
  );
  const [searchInput, setSearchInput] = useState(q ?? "");
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [pendingFilters, setPendingFilters] = useState<AuditMultiFilters | null>(null);
  const { info: toastInfo } = useToast();
  const [exporterOpen, setExporterOpen] = useState(false);

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

  const onRangeChange = useCallback(
    (next: AuditRangePreset | "custom") => {
      if (next === "custom") {
        toastInfo("Custom range picker ships in E5-b");
        return;
      }
      const param = next === DEFAULT_AUDIT_RANGE ? null : next;
      writeSingleParam(router, pathname, searchParams, "range", param);
    },
    [router, pathname, searchParams, toastInfo],
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
  const policy = useViewerPolicy();
  const rawRemoteRows = useMemo(
    () => feed.data?.pages.flatMap((p) => p.rows) ?? [],
    [feed.data],
  );
  const remoteRows = useMemo(
    () => rawRemoteRows.map((row) => redactAuditRow(row, policy)),
    [rawRemoteRows, policy],
  );
  const rows = useMemo(
    () => applyAuditMultiFilters(remoteRows, multiFilters),
    [remoteRows, multiFilters],
  );
  const presentCategories = useMemo<RbacPermission[]>(
    () => categoriesLackedBy(policy.permissions),
    [policy.permissions],
  );
  const workers = useMemo(() => auditWorkerOptions(remoteRows), [remoteRows]);
  const approvers = useMemo(() => auditApproverOptions(remoteRows), [remoteRows]);
  const buckets = useMemo(() => groupByDay(rows), [rows]);
  const totalPages = feed.data?.pages[0]?.totalPages;
  const currentPageCount = feed.data?.pages.length ?? 0;
  const isPristine = isPristineState(range, q, multiFilters);
  const hasActiveFilters = !isAuditFilterEmpty(liveFilters);
  const showRedactionBanner = rows.length > 0 && presentCategories.length > 0;
  const exportTenantId = policy.tenantId ?? testExportTenantId();
  const exportTenantLabel = policy.tenantLabel ?? "Current workspace";
  // `feed` + `workspaceState` are mounted in the OUTER `AuditPage`
  // component (above the `useSearchParams()` Suspense boundary) and passed
  // in — see the `AuditPage` doc-comment for why the fetch must live there.

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

      <AuditChainExplorerHero
        rows={rows}
        range={range}
        isFiltered={!isPristine}
        onExportClick={() => setExporterOpen(true)}
      />

      <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-6 space-y-4">
        {showRedactionBanner ? (
          <RedactionBanner
            policy={policy}
            presentCategories={presentCategories}
            surface="audit-feed"
          />
        ) : null}

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
          <AuditUnreachable
            message={feed.error?.message ?? "Couldn't load audit"}
            onRetry={() => feed.refetch()}
          />
        ) : rows.length === 0 ? (
          <AuditEmpty
            isPristine={isPristine}
            filters={multiFilters}
            workspaceState={workspaceState}
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
          tenantId={exportTenantId}
          tenantLabel={exportTenantLabel}
          initialScope={{ kind: "filtered" }}
          onToast={toastInfo}
        />
      ) : null}
    </>
  );
}

function testExportTenantId(): string {
  return process.env.NODE_ENV === "test" ? "tenant_test" : "";
}

// Radix `Popover.Trigger asChild` clones the immediate child element and
// injects onClick/ref/aria-expanded onto it. A plain function component
// that doesn't accept + spread the rest-props swallows those injections
// silently — the click reaches the button but never reaches Radix. Spread
// the rest-props onto the underlying `<button>` so the popover opens.
function FilterTriggerButton({
  hasActive,
  ...slotProps
}: { hasActive: boolean } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      aria-label="Open filter popover"
      data-testid="audit-filter-trigger"
      className={cn(
        buttonVariants({ variant: "secondary", size: "sm" }),
        "relative px-2",
      )}
      {...slotProps}
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

function AuditUnreachable({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div
      data-testid="audit-error"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--destructive)]/40 bg-[var(--destructive)]/5 py-8 px-6 text-center space-y-3"
    >
      <p className="text-sm text-[var(--foreground)]">
        Couldn&apos;t reach the audit feed
      </p>
      <p className="text-xs text-[var(--muted-foreground)]">
        Check your connection or contact support if this keeps happening.
      </p>
      <p className="text-[10px] font-mono text-[var(--muted-foreground)]/80">
        {message}
      </p>
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

