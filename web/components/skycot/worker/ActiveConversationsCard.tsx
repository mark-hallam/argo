"use client";

import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { RedactedText } from "@/components/skycot/RedactedText";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import type { ActiveConversation } from "@/lib/api/worker-memory";

/**
 * Active conversations the worker is mid-thread on. Each row links
 * to the audit drill-in for the latest action on that conversation
 * (E5-a is the canonical surface). Counterparty handle + topic snippet
 * route through `<RedactedText>` so PII sentinels resolve per the
 * viewer's redaction policy.
 */

export interface ActiveConversationsCardProps {
  conversations: ActiveConversation[];
  error?: string | null;
  onRetry?: () => void;
  /** Resolved redaction policy. Omit to fall back to `useRedactionPolicy()`. */
  redactionPolicy?: RedactionPolicy;
  className?: string;
}

export function ActiveConversationsCard({
  conversations,
  error,
  onRetry,
  redactionPolicy,
  className,
}: ActiveConversationsCardProps) {
  return (
    <Card data-testid="card-active-conversations" className={className}>
      <CardHeader>
        <CardTitle>Active conversations</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <ConversationsErrorState message={error} onRetry={onRetry} />
        ) : conversations.length === 0 ? (
          <p
            data-testid="conversations-empty"
            className="text-sm text-[var(--muted-foreground)]"
          >
            No open conversations right now.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {conversations.map((c) => (
              <ConversationRow
                key={c.conversationId}
                conversation={c}
                redactionPolicy={redactionPolicy}
              />
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

function ConversationRow({
  conversation,
  redactionPolicy,
}: {
  conversation: ActiveConversation;
  redactionPolicy?: RedactionPolicy;
}) {
  return (
    <li data-testid={`conversation-row-${conversation.conversationId}`} className="py-2">
      <Link
        href={conversation.href}
        className="block rounded-[var(--radius-sm)] -mx-1 px-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] hover:bg-[var(--accent)]/40"
      >
        <div className="grid grid-cols-[1fr_auto] items-baseline gap-x-3 text-sm">
          <div className="min-w-0">
            <p className="truncate font-medium text-[var(--foreground)]">
              <RedactedText policy={redactionPolicy}>
                {conversation.counterpartyHandle}
              </RedactedText>
            </p>
            <p className="truncate text-xs text-[var(--muted-foreground)]">
              <RedactedText policy={redactionPolicy}>{conversation.topic}</RedactedText>
            </p>
          </div>
          <div className="flex items-center gap-1 shrink-0">
            <time
              dateTime={conversation.lastMessageAt}
              className="text-xs tabular-nums text-[var(--muted-foreground)]"
            >
              {formatRelative(conversation.lastMessageAt)}
            </time>
            <ChevronRight className="size-3.5 text-[var(--muted-foreground)]" aria-hidden />
          </div>
        </div>
      </Link>
    </li>
  );
}

function formatRelative(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  const deltaMs = Math.max(0, Date.now() - t);
  const minutes = Math.round(deltaMs / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

function ConversationsErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}

export function ActiveConversationsCardSkeleton() {
  return (
    <Card data-testid="card-active-conversations-skeleton">
      <CardHeader>
        <div className="h-4 w-44 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {[0, 1].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="flex-1 space-y-1">
              <div className="h-3 w-28 rounded bg-[var(--muted)]" />
              <div className="h-3 w-48 rounded bg-[var(--muted)]" />
            </div>
            <div className="h-3 w-10 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
