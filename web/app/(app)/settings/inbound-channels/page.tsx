"use client";

import "./page.css";

import { useCallback, useEffect, useState } from "react";
import { Copy, ExternalLink, Inbox } from "lucide-react";

import {
  INBOUND_CHANNELS,
  type InboundChannelEntry,
  type InboundKind,
} from "@/lib/notify-channels/inbound-registry";
import {
  fetchInboundChannelRecent,
  type InboundChannelEvent,
} from "@/lib/api/inbound-channels";

/**
 * `/settings/inbound-channels` — Scene-6 surface that lets the
 * operator see the webhook URLs to paste into Twilio / Discord +
 * confirm inbound activity is actually reaching the gateway.
 *
 * Read-mostly (no mutation surfaces). The "Send test" button opens a
 * modal explaining the upstream-console procedure; we don't simulate
 * inbound traffic from the SPA because every inbound route validates
 * an upstream HMAC the SPA can't forge.
 */

interface RecentMap {
  [kind: string]: InboundChannelEvent[];
}

function resolveBaseUrl(): string {
  if (typeof window !== "undefined" && window.location?.origin) {
    return window.location.origin;
  }
  return "https://<your-skycot-host>";
}

export default function InboundChannelsPage() {
  const [recent, setRecent] = useState<RecentMap>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [testModal, setTestModal] = useState<InboundChannelEntry | null>(null);
  const [baseUrl, setBaseUrl] = useState<string>("https://<your-skycot-host>");

  useEffect(() => {
    setBaseUrl(resolveBaseUrl());
  }, []);

  const loadRecent = useCallback(async () => {
    const entries: Array<[string, InboundChannelEvent[]]> = await Promise.all(
      INBOUND_CHANNELS.map(async (c): Promise<[string, InboundChannelEvent[]]> => {
        try {
          const items = await fetchInboundChannelRecent(c.id, 20);
          return [c.id, items];
        } catch {
          return [c.id, []];
        }
      }),
    );
    const next: RecentMap = {};
    for (const [id, items] of entries) {
      next[id] = items;
    }
    setRecent(next);
  }, []);

  useEffect(() => {
    void loadRecent();
  }, [loadRecent]);

  function copyUrl(channelId: InboundKind, url: string) {
    void navigator.clipboard?.writeText(url).catch(() => undefined);
    setCopiedId(channelId);
    window.setTimeout(() => setCopiedId((cur) => (cur === channelId ? null : cur)), 1400);
  }

  return (
    <div className="inbound-stage" data-testid="inbound-channels-page">
      <span className="inbound-eyebrow">
        <Inbox size={12} aria-hidden />
        INBOUND CHANNELS
      </span>
      <h1 className="inbound-hero">
        Approval replies come back through these URLs.
      </h1>
      <p className="inbound-sub">
        Paste each webhook URL into the matching upstream console (Twilio for
        SMS &amp; Voice, the Discord Developer Portal for Discord) so operator
        replies reach the gateway. The “recent activity” feed below shows the
        last replies that landed for each channel.
      </p>

      {INBOUND_CHANNELS.map((c) => (
        <InboundCard
          key={c.id}
          entry={c}
          baseUrl={baseUrl}
          copied={copiedId === c.id}
          events={recent[c.id] ?? []}
          onCopy={(url) => copyUrl(c.id, url)}
          onOpenTest={() => setTestModal(c)}
        />
      ))}

      {testModal ? (
        <TestModal entry={testModal} onClose={() => setTestModal(null)} />
      ) : null}
    </div>
  );
}

interface CardProps {
  entry: InboundChannelEntry;
  baseUrl: string;
  copied: boolean;
  events: InboundChannelEvent[];
  onCopy: (url: string) => void;
  onOpenTest: () => void;
}

function InboundCard({ entry, baseUrl, copied, events, onCopy, onOpenTest }: CardProps) {
  const Icon = entry.Icon;
  const url = `${baseUrl}${entry.webhookPath}`;
  return (
    <article
      className="inbound-card"
      data-testid={`inbound-card-${entry.id}`}
      data-channel-id={entry.id}
    >
      <header className="inbound-card-head">
        <div className={`inbound-card-icon ${entry.tintClass}`}>
          <Icon size={22} aria-hidden />
        </div>
        <div>
          <h2 className="inbound-card-name">{entry.name}</h2>
          <p className="inbound-card-blurb">{entry.description}</p>
        </div>
        <span
          className="inbound-status-pill"
          data-status={events.length > 0 ? "active" : "not-configured"}
          data-testid={`inbound-status-${entry.id}`}
        >
          {events.length > 0 ? "Receiving" : "No activity yet"}
        </span>
      </header>

      <p className="inbound-upstream">
        <ExternalLink size={12} aria-hidden /> Paste into{" "}
        <strong>{entry.upstreamConsole}</strong>.
      </p>

      <div className="inbound-url-row" data-testid={`inbound-url-${entry.id}`}>
        <span className="inbound-url-label">Webhook URL</span>
        <span className="inbound-url-value">{url}</span>
        <button
          type="button"
          className="inbound-copy-btn"
          onClick={() => onCopy(url)}
          aria-label={`Copy ${entry.name} webhook URL`}
          data-testid={`inbound-copy-${entry.id}`}
          data-copied={copied ? "true" : undefined}
        >
          <Copy size={12} aria-hidden /> {copied ? "Copied" : "Copy"}
        </button>
      </div>

      <button
        type="button"
        className="inbound-test-btn"
        onClick={onOpenTest}
        data-testid={`inbound-test-${entry.id}`}
      >
        Send test
      </button>

      <div className="inbound-section-label">RECENT ACTIVITY</div>
      {events.length === 0 ? (
        <div
          className="inbound-events-empty"
          data-testid={`inbound-events-empty-${entry.id}`}
        >
          No inbound replies yet. Once the upstream console is wired and an
          approval is sent, the last 20 will show here.
        </div>
      ) : (
        <ul className="inbound-events" data-testid={`inbound-events-${entry.id}`}>
          {events.map((ev) => (
            <li key={ev.id}>
              <span className="inbound-events-when">{formatWhen(ev)}</span>
              <span className="inbound-events-from">
                {ev.replier ?? "—"} · {ev.action_id ?? "—"}
              </span>
              <span
                className="inbound-events-decision"
                data-decision={ev.decision ?? "unknown"}
              >
                {ev.decision ?? "—"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </article>
  );
}

function TestModal({
  entry,
  onClose,
}: {
  entry: InboundChannelEntry;
  onClose: () => void;
}) {
  return (
    <div
      className="inbound-modal-backdrop"
      role="presentation"
      onClick={onClose}
      data-testid="inbound-test-modal"
    >
      <div
        className="inbound-modal"
        role="dialog"
        aria-labelledby="inbound-test-modal-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="inbound-test-modal-title">Send a test to {entry.name}</h2>
        <p>{entry.testHowTo}</p>
        <p>
          Required env vars: <code>{entry.envVars.join(", ")}</code>
        </p>
        <button type="button" className="inbound-modal-close" onClick={onClose}>
          Close
        </button>
      </div>
    </div>
  );
}

function formatWhen(ev: InboundChannelEvent): string {
  if (ev.replied_at) return ev.replied_at.replace("T", " ").slice(0, 19);
  if (ev.created_at_micros) {
    return new Date(ev.created_at_micros / 1000).toISOString().replace("T", " ").slice(0, 19);
  }
  return "—";
}
