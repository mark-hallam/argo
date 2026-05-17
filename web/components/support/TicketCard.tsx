"use client";

import Link from "next/link";
import { Card } from "@/components/ui/Card";
import { StatusPill } from "@/components/ui/StatusPill";
import type { SupportTicket } from "@/lib/support/use-tickets";

const PRIORITY_TONE: Record<SupportTicket["priority"], "success" | "warning" | "error" | "info" | "muted"> = {
  low: "muted",
  normal: "info",
  high: "warning",
  urgent: "error",
};

function relativeTime(epochSeconds: number): string {
  const deltaMs = Date.now() - epochSeconds * 1000;
  if (deltaMs < 60_000) return "just now";
  const minutes = Math.round(deltaMs / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

export interface TicketCardProps {
  ticket: SupportTicket;
}

export function TicketCard({ ticket }: TicketCardProps) {
  return (
    <Link href={`/support/inbox/${ticket.id}`} className="block" data-testid={`ticket-card-${ticket.id}`}>
      <Card hoverable className="p-4 space-y-2">
        <div className="flex items-start justify-between gap-3">
          <h3 className="text-sm font-semibold text-[var(--foreground)] line-clamp-2">
            {ticket.subject}
          </h3>
          <StatusPill tone={PRIORITY_TONE[ticket.priority]} size="sm" dot={false}>
            {ticket.priority}
          </StatusPill>
        </div>
        <p className="text-xs text-[var(--muted-foreground)] truncate">{ticket.userEmail}</p>
        <p className="text-xs text-[var(--muted-foreground)] line-clamp-2">{ticket.body}</p>
        <div className="flex items-center justify-between text-[11px] text-[var(--muted-foreground)] pt-1">
          <span>{relativeTime(ticket.createdAt)}</span>
          {ticket.updatedAt > ticket.createdAt ? (
            <span>updated {relativeTime(ticket.updatedAt)}</span>
          ) : null}
        </div>
      </Card>
    </Link>
  );
}
