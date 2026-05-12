"use client";

import dynamic from "next/dynamic";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useMemo, useState, type ReactNode } from "react";
import { AlertCircle, Filter, Inbox, Loader2, Plus, Search } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import {
  ApprovalDaySection,
  IDLE_BATCH_UI,
  type BatchUiState,
} from "@/components/skycot/BatchGroupBlock";
import { DataSourceChip } from "@/components/skycot/DataSourceChip";
import { FilterChipRail } from "@/components/skycot/FilterChipRail";
import { FilterPopover } from "@/components/skycot/FilterPopover";
import { LiveIndicator } from "@/components/skycot/LiveIndicator";
import { NotificationsBell } from "@/components/skycot/NotificationsBell";
import { RequestElevatedView } from "@/components/skycot/RequestElevatedView";
import { ThemePicker } from "@/components/skycot/ThemePicker";
import { useUnreadNotificationCount } from "@/lib/hooks/use-notifications";
import { useRedactionPolicy } from "@/lib/hooks/use-redaction-policy";
import { groupApprovals as groupByDay, type ApprovalItem } from "@/lib/sample-approvals";
import type { ApprovalGroup } from "@/lib/similarity/group-approvals";
import {
  useApprovals,
  useBatchSubmitDecisions,
  useSubmitDecision,
  useUndoDecision,
} from "@/lib/hooks/use-approvals";
import {
  maskedCategories,
  redactPII,
  resultHasMasks,
} from "@/lib/redaction/redact";
import type { RedactionCategory } from "@/lib/redaction/patterns";
import {
  activeFilterCount, applyFilters, clearDimension, EMPTY_FILTERS, isFilterEmpty,
  parseFilters, serializeFilters, toggleFilterValue, workerOptions as deriveWorkerOptions,
  type ApprovalFilters,
} from "@/lib/filters/approvals";

// Lazy-load the popover (Radix popover + grouping logic adds ~20KB to
// First Load JS). The bell glyph + unread dot still render
// synchronously; the popover content fetches on first open.
const NotificationsPopover = dynamic(
  () =>
    import("@/components/skycot/NotificationsPopover").then((m) => ({
      default: m.NotificationsPopover,
    })),
  { ssr: false },
);

export default function ApprovalsInboxPage() {
  // useSearchParams() requires a Suspense boundary during static
  // generation (Next 15 prerender contract). The inner shell carries
  // all filter / popover state.
  return (
    <Suspense fallback={<LoadingState />}>
      <ApprovalsInboxShell />
    </Suspense>
  );
}

function ApprovalsInboxShell() {
  const approvalsQuery = useApprovals();
  const submitDecision = useSubmitDecision();
  const undoDecision = useUndoDecision();
  const batchSubmit = useBatchSubmitDecisions();
  const redactionPolicy = useRedactionPolicy();
  const [query, setQuery] = useState("");
  const [batchUi, setBatchUi] = useState<Record<string, BatchUiState>>({});
  const [approvedByGroup, setApprovedByGroup] = useState<Record<string, string[]>>({});

  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const filters = useMemo<ApprovalFilters>(() => parseFilters(searchParams), [searchParams]);

  const items = approvalsQuery.data ?? [];
  const workerOptionList = useMemo(() => deriveWorkerOptions(items), [items]);

  const setFilters = useCallback((next: ApprovalFilters) => {
    const filterQs = serializeFilters(next);
    const preserved = new URLSearchParams();
    searchParams.forEach((value, key) => {
      if (key !== "worker" && key !== "channel" && key !== "status") preserved.set(key, value);
    });
    const merged = [filterQs, preserved.toString()].filter(Boolean).join("&");
    router.replace(merged ? `${pathname}?${merged}` : pathname, { scroll: false });
  }, [pathname, router, searchParams]);

  const handleToggle = useCallback(
    <K extends keyof ApprovalFilters>(d: K, v: ApprovalFilters[K][number]) =>
      setFilters(toggleFilterValue(filters, d, v)),
    [filters, setFilters],
  );
  const handleClearDimension = useCallback(
    (d: keyof ApprovalFilters) => setFilters(clearDimension(filters, d)),
    [filters, setFilters],
  );
  const handleClearAll = useCallback(() => setFilters(EMPTY_FILTERS), [setFilters]);

  const filteredByFilters = useMemo(() => applyFilters(items, filters), [items, filters]);

  const filteredByAll = useMemo<ApprovalItem[]>(() => {
    if (!query.trim()) return filteredByFilters;
    const q = query.trim().toLowerCase();
    return filteredByFilters.filter((a) =>
      [a.worker.name, a.verb, a.detail].some((s) => s.toLowerCase().includes(q)),
    );
  }, [filteredByFilters, query]);

  const dayGroups = useMemo(() => groupByDay(filteredByAll), [filteredByAll]);
  const decisionsById = useMemo(() => {
    const map: Record<string, ApprovalItem["decision"]> = {};
    for (const it of items) map[it.id] = it.decision ?? null;
    return map;
  }, [items]);

  const pendingCount = filteredByAll.filter((a) => a.decision == null).length;

  // Pre-scan visible cards to know whether to surface the elevated-view
  // CTA, and what categories to record on the audit POST. Cheap; the
  // regex passes run once per filter change.
  const maskedOnPage = useMemo<RedactionCategory[]>(() => {
    const seen = new Set<RedactionCategory>();
    for (const a of filteredByAll) {
      for (const field of [a.detail, a.consequenceIfYes, a.consequenceIfNo, a.reason ?? ""]) {
        if (!field) continue;
        const result = redactPII(field, redactionPolicy);
        if (resultHasMasks(result)) {
          for (const c of maskedCategories(result)) seen.add(c);
        }
      }
    }
    return Array.from(seen);
  }, [filteredByAll, redactionPolicy]);

  const onDecide = useCallback(
    (id: string, decision: "approve" | "refuse") =>
      submitDecision.mutate({ approvalId: id, decision }),
    [submitDecision],
  );
  const onUndo = useCallback(
    (id: string) => undoDecision.mutate(id),
    [undoDecision],
  );

  const handleApproveAll = useCallback(
    async (group: ApprovalGroup, idsOverride?: string[]) => {
      const ids = idsOverride ?? group.items.filter((it) => it.decision == null).map((it) => it.id);
      if (ids.length === 0) return;
      setBatchUi((s) => ({
        ...s,
        [group.key]: { ...IDLE_BATCH_UI, phase: "loading" },
      }));
      const outcomes = await batchSubmit.submit(
        ids.map((approvalId) => ({ approvalId, decision: "approve" as const })),
      );
      const failed = outcomes.filter((o) => !o.ok);
      const completed = outcomes.length - failed.length;
      const approvedIds = outcomes.filter((o) => o.ok).map((o) => o.approvalId);
      setBatchUi((s) => ({
        ...s,
        [group.key]: {
          ...IDLE_BATCH_UI,
          phase: "settled",
          completed,
          failed: failed.length,
          failedIds: failed.map((o) => o.approvalId),
        },
      }));
      setApprovedByGroup((m) => ({ ...m, [group.key]: approvedIds }));
    },
    [batchSubmit],
  );

  const handleReviewEach = useCallback(
    (group: ApprovalGroup) =>
      setBatchUi((s) => ({
        ...s,
        [group.key]: { ...(s[group.key] ?? IDLE_BATCH_UI), carouselOpen: true },
      })),
    [],
  );

  const handleCloseCarousel = useCallback(
    (groupKey: string) =>
      setBatchUi((s) => ({
        ...s,
        [groupKey]: { ...(s[groupKey] ?? IDLE_BATCH_UI), carouselOpen: false },
      })),
    [],
  );

  const handleUndoBatch = useCallback(
    (groupKey: string) => {
      const ids = approvedByGroup[groupKey] ?? [];
      for (const id of ids) undoDecision.mutate(id);
      setBatchUi((s) => ({ ...s, [groupKey]: IDLE_BATCH_UI }));
      setApprovedByGroup((m) => {
        const next = { ...m };
        delete next[groupKey];
        return next;
      });
    },
    [undoDecision, approvedByGroup],
  );

  return (
    <>
      <PageHeader
        query={query}
        setQuery={setQuery}
        pendingCount={pendingCount}
        dataSource={approvalsQuery.dataSource}
        liveStatus={approvalsQuery.liveStatus}
        filters={filters}
        workerOptions={workerOptionList}
        onToggle={handleToggle}
        onClearAll={handleClearAll}
      />

      <FilterChipRail
        filters={filters}
        onClearDimension={handleClearDimension}
        onClearAll={handleClearAll}
      />

      {maskedOnPage.length > 0 ? (
        <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 pt-3">
          <RequestElevatedView categories={maskedOnPage} />
        </div>
      ) : null}

      <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-8 space-y-10">
        {approvalsQuery.isPending ? (
          <LoadingState />
        ) : approvalsQuery.isError ? (
          <ErrorState
            message={(approvalsQuery.error as Error)?.message ?? "Couldn't load approvals"}
            onRetry={() => approvalsQuery.refetch()}
          />
        ) : filteredByAll.length === 0 ? (
          <EmptyState query={query} hasFilters={!isFilterEmpty(filters)} onClearAll={handleClearAll} />
        ) : (
          <>
            <ApprovalDaySection
              label="Today"
              items={dayGroups.today}
              decisionsById={decisionsById}
              batchUi={batchUi}
              redactionPolicy={redactionPolicy}
              onDecide={onDecide}
              onUndo={onUndo}
              onApproveAll={handleApproveAll}
              onReviewEach={handleReviewEach}
              onCloseCarousel={handleCloseCarousel}
              onUndoBatch={handleUndoBatch}
            />
            <ApprovalDaySection
              label="Yesterday"
              items={dayGroups.yesterday}
              decisionsById={decisionsById}
              batchUi={batchUi}
              redactionPolicy={redactionPolicy}
              onDecide={onDecide}
              onUndo={onUndo}
              onApproveAll={handleApproveAll}
              onReviewEach={handleReviewEach}
              onCloseCarousel={handleCloseCarousel}
              onUndoBatch={handleUndoBatch}
            />
            <ApprovalDaySection
              label="Earlier"
              items={dayGroups.earlier}
              decisionsById={decisionsById}
              batchUi={batchUi}
              redactionPolicy={redactionPolicy}
              onDecide={onDecide}
              onUndo={onUndo}
              onApproveAll={handleApproveAll}
              onReviewEach={handleReviewEach}
              onCloseCarousel={handleCloseCarousel}
              onUndoBatch={handleUndoBatch}
            />
          </>
        )}

        <footer className="pt-8 mt-12 border-t border-[var(--border)] text-xs text-[var(--muted-foreground)]">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <span>SKYCOT preview — design system Track C. Switch theme to test white-label:</span>
            <ThemePicker />
          </div>
        </footer>
      </div>

    </>
  );
}

function NotificationsBellTrigger() {
  const unreadCount = useUnreadNotificationCount();
  const [armed, setArmed] = useState(false);

  const bellButton: ReactNode = (
    <Button
      variant="ghost"
      size="icon"
      aria-label="Notifications"
      onClick={armed ? undefined : () => setArmed(true)}
    >
      <NotificationsBell unreadCount={unreadCount} />
    </Button>
  );

  if (!armed) return bellButton;

  return (
    <NotificationsPopover defaultOpen>{bellButton}</NotificationsPopover>
  );
}

interface PageHeaderProps {
  query: string;
  setQuery: (s: string) => void;
  pendingCount: number;
  dataSource: ReturnType<typeof useApprovals>["dataSource"];
  liveStatus: ReturnType<typeof useApprovals>["liveStatus"];
  filters: ApprovalFilters;
  workerOptions: string[];
  onToggle: <K extends keyof ApprovalFilters>(d: K, v: ApprovalFilters[K][number]) => void;
  onClearAll: () => void;
}

function PageHeader({ query, setQuery, pendingCount, dataSource, liveStatus, filters, workerOptions, onToggle, onClearAll }: PageHeaderProps) {
  const activeCount = activeFilterCount(filters);
  return (
    <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
      <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-5">
        <div className="flex items-start justify-between gap-4 flex-wrap">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
                Approvals
              </h1>
              <StatusPill tone="info" size="sm">
                {pendingCount} waiting
              </StatusPill>
            </div>
            <div className="flex items-center gap-2">
              <p className="text-sm text-[var(--muted-foreground)]">
                Your AI teammates need a yes or no.
              </p>
              <LiveIndicator status={liveStatus} />
              <DataSourceChip source={dataSource} />
            </div>
          </div>

          <div className="flex items-center gap-1.5 sm:gap-2">
            <FilterPopover
              filters={filters}
              workerOptions={workerOptions}
              onToggle={onToggle}
              onClearAll={onClearAll}
            >
              <Button
                variant="ghost"
                size="icon"
                aria-label="Filter"
                aria-pressed={activeCount > 0}
                className="relative"
                data-testid="filter-trigger"
              >
                <Filter />
                {activeCount > 0 ? (
                  <span
                    className="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-[var(--primary)]"
                    aria-hidden
                    data-testid="filter-trigger-dot"
                  />
                ) : null}
              </Button>
            </FilterPopover>
            <NotificationsBellTrigger />
            <Button variant="primary" size="md" className="shadow-sm" aria-label="Hire teammate">
              <Plus />
              <span className="hidden sm:inline">Hire teammate</span>
            </Button>
          </div>
        </div>

        <div className="mt-4 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by worker, action, or detail..."
            className="w-full h-10 pl-9 pr-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
          />
        </div>
      </div>
    </div>
  );
}

function LoadingState() {
  return (
    <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-3">
      <Loader2 className="size-6 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="text-sm text-[var(--muted-foreground)]">Loading approvals…</p>
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
        <h2 className="text-base font-semibold text-[var(--foreground)]">Couldn&apos;t reach the gateway</h2>
        <p className="text-sm text-[var(--muted-foreground)]">{message}</p>
      </div>
      <Button variant="secondary" size="sm" onClick={onRetry}>
        Try again
      </Button>
    </div>
  );
}

function EmptyState({ query, hasFilters, onClearAll }: { query: string; hasFilters: boolean; onClearAll: () => void }) {
  const idle = !hasFilters && !query;
  const title = idle ? "All clear" : "No matching approvals";
  return (
    <div
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-3"
      data-testid={hasFilters ? "approvals-empty-filtered" : "approvals-empty"}
    >
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]">
        <Inbox />
      </div>
      <div>
        <h2 className="text-base font-semibold text-[var(--foreground)]">{title}</h2>
        <p className="text-sm text-[var(--muted-foreground)] mt-1">
          {idle
            ? "Your teammates have nothing waiting on you. Get back to work — they've got the rest."
            : hasFilters
              ? <>Try different filters or <button type="button" onClick={onClearAll} className="text-[var(--primary)] underline-offset-4 hover:underline" data-testid="approvals-empty-clear-all">Clear all</button>.</>
              : `Nothing matches "${query}". Try different keywords or clear the filter.`}
        </p>
      </div>
    </div>
  );
}
