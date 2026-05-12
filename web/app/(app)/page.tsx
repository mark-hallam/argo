"use client";

import dynamic from "next/dynamic";
import { useMemo, useState, type ReactNode } from "react";
import {
  AlertCircle,
  Filter,
  Inbox,
  Loader2,
  Plus,
  Search,
} from "lucide-react";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { ApprovalCard } from "@/components/skycot/ApprovalCard";
import { NotificationsBell } from "@/components/skycot/NotificationsBell";
import { ThemePicker } from "@/components/skycot/ThemePicker";
import { useUnreadNotificationCount } from "@/lib/hooks/use-notifications";
import { groupApprovals, type ApprovalItem } from "@/lib/sample-approvals";
import { useApprovals, useSubmitDecision, useUndoDecision } from "@/lib/hooks/use-approvals";

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
  const approvalsQuery = useApprovals();
  const submitDecision = useSubmitDecision();
  const undoDecision = useUndoDecision();
  const [query, setQuery] = useState("");

  const items = approvalsQuery.data ?? [];

  const filtered = useMemo<ApprovalItem[]>(() => {
    if (!query.trim()) return items;
    const q = query.trim().toLowerCase();
    return items.filter((a) =>
      [a.worker.name, a.verb, a.detail].some((s) => s.toLowerCase().includes(q)),
    );
  }, [items, query]);

  const groups = useMemo(() => groupApprovals(filtered), [filtered]);
  const pendingCount = filtered.filter((a) => a.decision == null).length;

  return (
    <>
      <PageHeader query={query} setQuery={setQuery} pendingCount={pendingCount} />

      <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-8 space-y-10">
        {approvalsQuery.isPending ? (
          <LoadingState />
        ) : approvalsQuery.isError ? (
          <ErrorState
            message={(approvalsQuery.error as Error)?.message ?? "Couldn't load approvals"}
            onRetry={() => approvalsQuery.refetch()}
          />
        ) : filtered.length === 0 ? (
          <EmptyState query={query} />
        ) : (
          <>
            <ApprovalGroup
              label="Today"
              items={groups.today}
              onDecide={(id, d) => submitDecision.mutate({ approvalId: id, decision: d })}
              onUndo={(id) => undoDecision.mutate(id)}
            />
            <ApprovalGroup
              label="Yesterday"
              items={groups.yesterday}
              onDecide={(id, d) => submitDecision.mutate({ approvalId: id, decision: d })}
              onUndo={(id) => undoDecision.mutate(id)}
            />
            <ApprovalGroup
              label="Earlier"
              items={groups.earlier}
              onDecide={(id, d) => submitDecision.mutate({ approvalId: id, decision: d })}
              onUndo={(id) => undoDecision.mutate(id)}
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
  // Defer mounting the Radix popover until first interaction. Once
  // mounted, leave it in the tree so subsequent opens are instant.
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

function PageHeader({
  query,
  setQuery,
  pendingCount,
}: {
  query: string;
  setQuery: (s: string) => void;
  pendingCount: number;
}) {
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
            <p className="text-sm text-[var(--muted-foreground)]">
              Your AI teammates need a yes or no.
            </p>
          </div>

          <div className="flex items-center gap-1.5 sm:gap-2">
            <Button variant="ghost" size="icon" aria-label="Filter">
              <Filter />
            </Button>
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

function ApprovalGroup({
  label,
  items,
  onDecide,
  onUndo,
}: {
  label: string;
  items: ApprovalItem[];
  onDecide: (approvalId: string, decision: "approve" | "refuse") => void;
  onUndo: (approvalId: string) => void;
}) {
  if (items.length === 0) return null;

  return (
    <section className="space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          {label}
        </h2>
        <div className="flex-1 h-px bg-[var(--border)]" />
        <span className="text-xs text-[var(--muted-foreground)]">
          {items.length} {items.length === 1 ? "request" : "requests"}
        </span>
      </div>

      <div className="space-y-3">
        {items.map((item) => (
          <ApprovalCard
            key={item.id}
            className="max-w-full"
            worker={item.worker}
            verb={item.verb}
            detail={item.detail}
            consequenceIfYes={item.consequenceIfYes}
            consequenceIfNo={item.consequenceIfNo}
            reason={item.reason}
            channel={item.channel}
            receivedAt={item.receivedAt}
            decision={item.decision ?? null}
            onDecide={(d) => onDecide(item.id, d)}
            onUndo={() => onUndo(item.id)}
          />
        ))}
      </div>
    </section>
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

function EmptyState({ query }: { query: string }) {
  return (
    <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-3">
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]">
        <Inbox />
      </div>
      <div>
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          {query ? "No matching approvals" : "All clear"}
        </h2>
        <p className="text-sm text-[var(--muted-foreground)] mt-1">
          {query
            ? `Nothing matches "${query}". Try different keywords or clear the filter.`
            : "Your teammates have nothing waiting on you. Get back to work — they've got the rest."}
        </p>
      </div>
    </div>
  );
}
