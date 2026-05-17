"use client";

import { useState, type FormEvent } from "react";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import type { SupportReply, SupportTicket, TicketStatus } from "@/lib/support/use-tickets";
import { usePostReply, useUpdateTicket } from "@/lib/support/use-tickets";

const AUTHOR_LABEL: Record<SupportReply["authorKind"], string> = {
  user: "Customer",
  operator: "Operator",
  system: "System",
};

const STATUS_OPTIONS: TicketStatus[] = ["open", "pending", "resolved", "closed"];

function formatTimestamp(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleString();
}

export interface TicketThreadProps {
  ticket: SupportTicket;
  replies: SupportReply[];
}

export function TicketThread({ ticket, replies }: TicketThreadProps) {
  const [draft, setDraft] = useState("");
  const postReply = usePostReply();
  const updateTicket = useUpdateTicket();

  const onSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const body = draft.trim();
    if (body.length === 0) return;
    postReply.mutate(
      { ticketId: ticket.id, body },
      {
        onSuccess: () => {
          setDraft("");
        },
      },
    );
  };

  return (
    <div className="space-y-6">
      <header className="space-y-2">
        <div className="flex items-center justify-between gap-3 flex-wrap">
          <h1 className="text-xl font-semibold text-[var(--foreground)]">
            {ticket.subject}
          </h1>
          <div className="flex items-center gap-2">
            <StatusPill tone="muted" size="sm" dot={false}>
              {ticket.status}
            </StatusPill>
            <StatusPill tone="info" size="sm" dot={false}>
              {ticket.priority}
            </StatusPill>
          </div>
        </div>
        <p className="text-sm text-[var(--muted-foreground)]">{ticket.userEmail}</p>
      </header>

      <section
        data-testid="ticket-status-row"
        className="flex items-center gap-2 flex-wrap"
      >
        <span className="text-xs text-[var(--muted-foreground)]">Status:</span>
        {STATUS_OPTIONS.map((s) => (
          <button
            key={s}
            type="button"
            disabled={updateTicket.isPending || ticket.status === s}
            onClick={() =>
              updateTicket.mutate({ ticketId: ticket.id, status: s })
            }
            className="text-xs px-2 py-1 rounded-[var(--radius-sm)] border border-[var(--border)] hover:bg-[var(--muted)] disabled:opacity-50 disabled:cursor-not-allowed"
            data-testid={`set-status-${s}`}
          >
            {s}
          </button>
        ))}
      </section>

      <article className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4 space-y-2">
        <header className="flex items-center justify-between text-xs text-[var(--muted-foreground)]">
          <span>Original message</span>
          <span>{formatTimestamp(ticket.createdAt)}</span>
        </header>
        <p className="text-sm text-[var(--foreground)] whitespace-pre-wrap">{ticket.body}</p>
      </article>

      <section className="space-y-3" data-testid="ticket-thread">
        {replies.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">No replies yet.</p>
        ) : (
          replies.map((r) => (
            <article
              key={r.id}
              data-testid={`reply-${r.id}`}
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4 space-y-2"
            >
              <header className="flex items-center justify-between text-xs text-[var(--muted-foreground)]">
                <span>{AUTHOR_LABEL[r.authorKind]}</span>
                <span>{formatTimestamp(r.createdAt)}</span>
              </header>
              <p className="text-sm text-[var(--foreground)] whitespace-pre-wrap">{r.body}</p>
            </article>
          ))
        )}
      </section>

      <form onSubmit={onSubmit} className="space-y-2" data-testid="reply-form">
        <label htmlFor="reply-body" className="text-xs font-medium text-[var(--muted-foreground)]">
          Reply
        </label>
        <textarea
          id="reply-body"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          rows={4}
          className="w-full rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)] p-3 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
          placeholder="Write a reply to the customer..."
          data-testid="reply-textarea"
        />
        <div className="flex items-center justify-between">
          <span className="text-xs text-[var(--muted-foreground)]">
            {postReply.isError ? "Failed to send reply" : ""}
          </span>
          <Button
            type="submit"
            disabled={postReply.isPending || draft.trim().length === 0}
            data-testid="reply-submit"
          >
            {postReply.isPending ? "Sending…" : "Send reply"}
          </Button>
        </div>
      </form>
    </div>
  );
}
