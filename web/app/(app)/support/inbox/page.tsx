"use client";

import { useMemo } from "react";
import { TicketCard } from "@/components/support/TicketCard";
import { useTickets, type SupportTicket, type TicketStatus } from "@/lib/support/use-tickets";

const COLUMNS: { status: TicketStatus; label: string }[] = [
  { status: "open", label: "Open" },
  { status: "pending", label: "Pending" },
  { status: "resolved", label: "Resolved" },
];

export default function SupportInboxPage() {
  const all = useTickets();
  const tickets = all.data ?? [];

  const byStatus = useMemo(() => {
    const map = new Map<TicketStatus, SupportTicket[]>();
    for (const col of COLUMNS) map.set(col.status, []);
    for (const t of tickets) {
      if (map.has(t.status)) map.get(t.status)!.push(t);
    }
    return map;
  }, [tickets]);

  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <header className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
          Support inbox
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Inbound tickets from the help widget and marketing-site contact form.
        </p>
      </header>

      {all.isError ? (
        <div
          role="alert"
          data-testid="support-inbox-error"
          className="rounded-[var(--radius-md)] border border-[var(--destructive)]/40 bg-[var(--destructive)]/10 p-4 text-sm text-[var(--destructive)]"
        >
          Failed to load tickets. Admin access required.
        </div>
      ) : null}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4" data-testid="support-inbox-board">
        {COLUMNS.map((col) => {
          const items = byStatus.get(col.status) ?? [];
          return (
            <section
              key={col.status}
              data-testid={`column-${col.status}`}
              className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--muted)]/30 p-3 space-y-3 min-h-[200px]"
            >
              <header className="flex items-center justify-between">
                <h2 className="text-sm font-semibold text-[var(--foreground)]">
                  {col.label}
                </h2>
                <span className="text-xs text-[var(--muted-foreground)]">
                  {items.length}
                </span>
              </header>
              {all.isPending ? (
                <p className="text-xs text-[var(--muted-foreground)]">Loading…</p>
              ) : items.length === 0 ? (
                <p className="text-xs text-[var(--muted-foreground)]">No tickets</p>
              ) : (
                <div className="space-y-2">
                  {items.map((t) => (
                    <TicketCard key={t.id} ticket={t} />
                  ))}
                </div>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
