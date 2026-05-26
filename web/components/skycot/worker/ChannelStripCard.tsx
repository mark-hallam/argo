"use client";

import { useMemo } from "react";
import Link from "next/link";
import {
  ChevronRight,
  Globe,
  Mail,
  MessageCircle,
  MessageSquare,
  Mic,
  Smartphone,
  type LucideIcon,
} from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { aggregateByChannel, type ChannelAggregate } from "@/lib/api/audit";
import { useAuditFeed } from "@/lib/hooks/use-audit-feed";
import type { ChannelKind } from "@/lib/sample-approvals";

/**
 * Claim 6 (multi-channel + multi-tenant) proof surface for the worker
 * overview tab. Aggregates the worker's last-7-days audit rows by
 * `channel` so the v1.0 demo MP4 can land "Mariah pinged your Slack
 * 4 times and your Email 2 times in the last 7 days — here's the
 * audit trail for one of those Slack messages." (Scene 1/2 finale.)
 *
 * Each row links to `/audit?worker=<id>&channel=<channel>` so the
 * operator can drill into the per-channel slice in one click. The
 * audit page's `AuditFilterChipRail` already supports the `channel`
 * multi-select dimension; we just round-trip the URL.
 *
 * Empty / single-channel-only state: when the worker has acted only
 * on `web` (the SPA's own inbox) the card switches to a connect-CTA
 * that points at `/settings/connectors` so the operator can wire
 * Slack / Gmail / SMS without leaving the worker drill-in.
 */
export interface ChannelStripCardProps {
  workerId: string;
  workerName?: string;
  /** Test seam — bypasses `useAuditFeed` so unit tests can assert
   *  rendered output against a hand-rolled aggregate list. */
  aggregatesOverride?: ChannelAggregate[];
  className?: string;
}

const CHANNEL_LABEL: Record<ChannelKind, string> = {
  slack: "Slack",
  imessage: "iMessage",
  sms: "SMS",
  push: "Push",
  web: "Web inbox",
  voice: "Voice",
  whatsapp: "WhatsApp",
};

const CHANNEL_ICON: Record<ChannelKind, LucideIcon> = {
  slack: MessageSquare,
  imessage: MessageCircle,
  sms: Smartphone,
  push: Smartphone,
  web: Globe,
  voice: Mic,
  whatsapp: MessageCircle,
};

const NON_WEB_CHANNELS: readonly ChannelKind[] = [
  "slack",
  "imessage",
  "sms",
  "voice",
  "whatsapp",
  "push",
];

export function ChannelStripCard({
  workerId,
  workerName,
  aggregatesOverride,
  className,
}: ChannelStripCardProps) {
  const feed = useAuditFeed(
    { worker: workerId, range: "7d" },
    { enabled: aggregatesOverride === undefined },
  );

  const aggregates = useMemo<ChannelAggregate[]>(() => {
    if (aggregatesOverride !== undefined) return aggregatesOverride;
    const rows = feed.data?.pages.flatMap((p) => p.rows) ?? [];
    return aggregateByChannel(rows);
  }, [aggregatesOverride, feed.data]);

  if (aggregatesOverride === undefined && feed.isPending) {
    return <ChannelStripCardSkeleton className={className} />;
  }

  const hasNonWebChannel = aggregates.some((a) =>
    (NON_WEB_CHANNELS as readonly string[]).includes(a.channel),
  );

  if (!hasNonWebChannel) {
    return (
      <Card data-testid="card-channel-strip" className={className}>
        <CardHeader>
          <CardTitle>Channels</CardTitle>
        </CardHeader>
        <CardBody className="pt-1">
          <EmptyState workerName={workerName} />
        </CardBody>
      </Card>
    );
  }

  return (
    <Card data-testid="card-channel-strip" className={className}>
      <CardHeader>
        <CardTitle>Channels</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        <ul role="list" className="divide-y divide-[var(--border)]/60">
          {aggregates.map((agg) => (
            <ChannelRow
              key={agg.channel}
              aggregate={agg}
              workerId={workerId}
              workerName={workerName}
            />
          ))}
        </ul>
      </CardBody>
    </Card>
  );
}

function ChannelRow({
  aggregate,
  workerId,
  workerName,
}: {
  aggregate: ChannelAggregate;
  workerId: string;
  workerName?: string;
}) {
  const Icon = CHANNEL_ICON[aggregate.channel];
  const label = CHANNEL_LABEL[aggregate.channel];
  const subject = workerName ?? "this worker";
  const href = `/audit?worker=${encodeURIComponent(workerId)}&channel=${aggregate.channel}`;
  return (
    <li data-testid={`channel-row-${aggregate.channel}`} className="py-2">
      <Link
        href={href}
        aria-label={`Show ${subject}'s ${label} activity`}
        data-testid={`channel-row-link-${aggregate.channel}`}
        className="block rounded-[var(--radius-sm)] -mx-1 px-1 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] hover:bg-[var(--accent)]/40"
      >
        <span className="grid grid-cols-[auto_1fr_auto_auto] items-center gap-x-3 text-sm">
          <span
            aria-hidden
            className="grid size-7 place-items-center rounded-[var(--radius-sm)] bg-[var(--muted)] text-[var(--muted-foreground)]"
          >
            <Icon className="size-4" />
          </span>
          <span className="truncate text-[var(--foreground)]">{label}</span>
          <span
            data-testid={`channel-count-${aggregate.channel}`}
            className="shrink-0 tabular-nums text-xs text-[var(--muted-foreground)]"
          >
            {aggregate.count} {aggregate.count === 1 ? "action" : "actions"}
            <span className="px-1.5 text-[var(--border)]" aria-hidden>
              ·
            </span>
            <time
              dateTime={aggregate.lastSeen}
              className="tabular-nums text-[var(--muted-foreground)]"
            >
              {formatLastSeen(aggregate.lastSeen)}
            </time>
          </span>
          <ChevronRight
            aria-hidden
            className="size-4 text-[var(--muted-foreground)]"
          />
        </span>
      </Link>
    </li>
  );
}

function EmptyState({ workerName }: { workerName?: string }) {
  const subject = workerName ?? "This worker";
  return (
    <div
      data-testid="channel-strip-empty"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] bg-[var(--muted)]/30 px-4 py-5 text-sm"
    >
      <p className="text-[var(--foreground)]">
        {subject} hasn&apos;t picked up Slack or email yet.
      </p>
      <p className="mt-1 text-[var(--muted-foreground)]">
        Connect a channel so {subject.toLowerCase() === "this worker" ? "she" : subject} can
        respond where your team already works.
      </p>
      <Link
        href="/settings/connectors"
        data-testid="channel-strip-connect-cta"
        className="mt-3 inline-flex items-center gap-1 text-sm font-medium text-[var(--primary)] hover:underline underline-offset-4"
      >
        Connect a channel
        <ChevronRight className="size-3.5" aria-hidden />
      </Link>
    </div>
  );
}

function formatLastSeen(iso: string, now: number = Date.now()): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const diff = Math.max(0, Math.round((now - t) / 1000));
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.round(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)}h ago`;
  return `${Math.round(diff / 86400)}d ago`;
}

export function ChannelStripCardSkeleton({ className }: { className?: string }) {
  return (
    <Card data-testid="card-channel-strip-skeleton" className={className}>
      <CardHeader>
        <div className="h-4 w-24 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {[0, 1, 2].map((i) => (
          <div key={i} className="flex items-center gap-3 py-1.5">
            <div className="size-7 rounded bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-3 w-24 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
