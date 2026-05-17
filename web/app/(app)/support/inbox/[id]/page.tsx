"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { TicketThread } from "@/components/support/TicketThread";
import { useTicket } from "@/lib/support/use-tickets";

export default function SupportTicketDetailPage() {
  const params = useParams<{ id: string }>();
  const id = params?.id ?? "";
  const detail = useTicket(id);

  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <Link
        href="/support/inbox"
        className="inline-flex items-center gap-1 text-sm text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        data-testid="ticket-back"
      >
        <ArrowLeft className="size-4" />
        Back to inbox
      </Link>

      {detail.isError ? (
        <div
          role="alert"
          data-testid="ticket-detail-error"
          className="rounded-[var(--radius-md)] border border-[var(--destructive)]/40 bg-[var(--destructive)]/10 p-4 text-sm text-[var(--destructive)]"
        >
          Failed to load ticket. The ticket may have been deleted or you may
          lack admin access.
        </div>
      ) : null}

      {detail.isPending ? (
        <p className="text-sm text-[var(--muted-foreground)]">Loading ticket…</p>
      ) : detail.data ? (
        <TicketThread ticket={detail.data.ticket} replies={detail.data.replies} />
      ) : null}
    </div>
  );
}
