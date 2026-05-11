"use client";

import { useMemo, useState } from "react";
import {
  Activity,
  Bell,
  ChevronDown,
  Filter,
  Inbox,
  Plus,
  Search,
  Settings,
  Sparkles,
  Users,
  Workflow,
} from "lucide-react";
import { Avatar } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { NavDrawer, NavItem } from "@/components/ui/NavDrawer";
import { ApprovalCard, type ApprovalDecision } from "@/components/skycot/ApprovalCard";
import { ThemePicker } from "@/components/skycot/ThemePicker";
import { groupApprovals, SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

export default function ApprovalsInboxPage() {
  // Decisions are mock state; in Track E this becomes a TanStack Query mutation against the gateway.
  const [decisions, setDecisions] = useState<Record<string, ApprovalDecision | null>>({});
  const [query, setQuery] = useState("");

  const filtered = useMemo<ApprovalItem[]>(() => {
    if (!query.trim()) return SAMPLE_APPROVALS;
    const q = query.trim().toLowerCase();
    return SAMPLE_APPROVALS.filter((a) =>
      [a.worker.name, a.verb, a.detail].some((s) => s.toLowerCase().includes(q)),
    );
  }, [query]);

  const groups = useMemo(() => groupApprovals(filtered), [filtered]);
  const pendingCount = filtered.filter((a) => decisions[a.id] == null).length;

  return (
    <div className="min-h-screen bg-[var(--background)]">
      <NavDrawer brand={<Brand />}>
        <NavItem href="#" active>
          <Inbox /> Approvals
          {pendingCount > 0 ? (
            <StatusPill tone="info" size="sm" dot={false} className="ml-auto">
              {pendingCount}
            </StatusPill>
          ) : null}
        </NavItem>
        <NavItem href="#">
          <Users /> Team
        </NavItem>
        <NavItem href="#">
          <Workflow /> Workers
        </NavItem>
        <NavItem href="#">
          <Activity /> Audit
        </NavItem>
        <NavItem href="#">
          <Sparkles /> Skills
        </NavItem>
        <div className="mt-auto flex flex-col gap-1 pt-3 border-t border-[var(--border)]">
          <NavItem href="#">
            <Settings /> Settings
          </NavItem>
          <WorkspaceFooter />
        </div>
      </NavDrawer>

      <main className="md:pl-64">
        <PageHeader query={query} setQuery={setQuery} pendingCount={pendingCount} />

        <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-8 space-y-10">
          {filtered.length === 0 ? (
            <EmptyState query={query} />
          ) : (
            <>
              <ApprovalGroup
                label="Today"
                items={groups.today}
                decisions={decisions}
                setDecisions={setDecisions}
              />
              <ApprovalGroup
                label="Yesterday"
                items={groups.yesterday}
                decisions={decisions}
                setDecisions={setDecisions}
              />
              <ApprovalGroup
                label="Earlier"
                items={groups.earlier}
                decisions={decisions}
                setDecisions={setDecisions}
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
      </main>
    </div>
  );
}

function Brand() {
  return (
    <a href="#" className="flex items-center gap-2.5 group">
      <span
        aria-hidden
        className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-semibold text-sm shadow-sm"
      >
        S
      </span>
      <span className="flex flex-col leading-tight">
        <span className="text-sm font-semibold text-[var(--foreground)]">SKYCOT</span>
        <span className="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
          AI teammates
        </span>
      </span>
    </a>
  );
}

function WorkspaceFooter() {
  return (
    <button
      type="button"
      className="mt-1 flex items-center gap-2.5 rounded-[var(--radius-md)] px-3 py-2 hover:bg-[var(--accent)] transition-colors duration-[var(--duration-fast)]"
    >
      <Avatar alt="Sam Harlow" size="sm" />
      <span className="flex-1 flex flex-col items-start leading-tight">
        <span className="text-sm font-medium text-[var(--foreground)]">Sam Harlow</span>
        <span className="text-[11px] text-[var(--muted-foreground)]">Acmeco · Owner</span>
      </span>
      <ChevronDown className="size-4 text-[var(--muted-foreground)]" />
    </button>
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

          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" aria-label="Filter">
              <Filter />
            </Button>
            <Button variant="ghost" size="icon" aria-label="Notifications">
              <Bell />
            </Button>
            <Button variant="primary" size="md" className="shadow-sm">
              <Plus />
              Hire teammate
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
  decisions,
  setDecisions,
}: {
  label: string;
  items: ApprovalItem[];
  decisions: Record<string, ApprovalDecision | null>;
  setDecisions: React.Dispatch<React.SetStateAction<Record<string, ApprovalDecision | null>>>;
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
            decision={decisions[item.id] ?? null}
            onDecide={(d) =>
              setDecisions((m) => ({ ...m, [item.id]: d }))
            }
            onUndo={() =>
              setDecisions((m) => ({ ...m, [item.id]: null }))
            }
          />
        ))}
      </div>
    </section>
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
