"use client";

/**
 * Live-update hook for the approvals inbox. Opens a WebSocket to the
 * gateway's `/inbox/ws` route (J3 structured-event protocol) and maps
 * the four `approval.*` lifecycle events to TanStack Query cache
 * mutations on the `["approvals"]` query.
 *
 * Returns `{ status, lastEventAt }` so the page can render a small
 * LiveIndicator. The hook owns its own WebSocket lifecycle — open on
 * mount, close on unmount — and connects regardless of auth-bypass
 * mode; if the gateway rejects the connection or is unreachable, the
 * underlying [`ManagedWebSocket`] trips into the polling-fallback
 * state and the indicator surfaces it.
 *
 * `approval.created` semantics:
 *
 * - When the wire payload carries the full `ApprovalRowFull` shape
 *   (the post-`mission-gateway-ws-emit-full-approval-row` default),
 *   we `setQueryData` and splice the new item into the cache without
 *   re-fetching, preserving the operator's scroll position.
 * - When the wire payload is the partial-row fallback
 *   (`partial: true`, e.g. worker-deleted race), we invalidate the
 *   query and re-fetch via the J1 JSON endpoint. The LiveIndicator
 *   stays on `live` — partial fallback is brief degradation, not a
 *   transport failure.
 */

import { useEffect, useState } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { ManagedWebSocket, type WebSocketStatus } from "@/lib/api/websocket";
import type { ApprovalsResult } from "@/lib/api/approvals";
import type { ApprovalItem, ChannelKind } from "@/lib/sample-approvals";
import type { WireApprovalCreatedPayload } from "@/lib/api/types";

const APPROVALS_KEY = ["approvals"] as const;
const DEFAULT_GATEWAY = "http://127.0.0.1:8787";
const KNOWN_CHANNELS: ReadonlySet<string> = new Set<ChannelKind>([
  "slack",
  "imessage",
  "sms",
  "push",
  "web",
  "voice",
  "whatsapp",
]);

/** UI-facing connection state surfaced to LiveIndicator. */
export type LiveStatus = "connecting" | "live" | "polling" | "offline";

/** Subset of the J3 wire vocabulary we surface to the cache. */
type WireEvent =
  | { type: "hello"; protocol_version?: string; capabilities?: string[]; seq?: number }
  | { type: "approval.created"; payload: WireApprovalCreatedPayload; seq?: number; ts?: string }
  | { type: "approval.decided"; payload: { approval_id: string }; decision: "approve" | "refuse"; seq?: number; ts?: string }
  | { type: "approval.undone"; payload: { approval_id: string }; seq?: number; ts?: string }
  | { type: "approval.expired"; payload: { approval_id: string }; seq?: number; ts?: string };

function mapStatus(s: WebSocketStatus): LiveStatus {
  switch (s) {
    case "open": return "live";
    case "polling": return "polling";
    case "connecting": return "connecting";
    case "closed": return "offline";
  }
}

function gatewayWsUrl(token: string | undefined): string {
  const base = process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
  const wsBase = base.replace(/^http(s?):/, "ws$1:");
  const qs = token ? `?token=${encodeURIComponent(token)}` : "";
  return `${wsBase}/inbox/ws${qs}`;
}

async function readSessionToken(): Promise<string | undefined> {
  if (typeof window === "undefined") return undefined;
  try {
    const res = await fetch("/api/auth/session", { credentials: "same-origin" });
    if (!res.ok) return undefined;
    const session = (await res.json()) as { gatewayToken?: string };
    return session?.gatewayToken;
  } catch {
    return undefined;
  }
}

/**
 * Convert a full wire payload into an `ApprovalItem`. Returns `null`
 * when required fields are missing — caller invalidates instead.
 */
export function wireToApprovalItem(
  payload: WireApprovalCreatedPayload,
): ApprovalItem | null {
  if (payload.partial) return null;
  if (
    typeof payload.id !== "string" ||
    !payload.worker ||
    typeof payload.worker.name !== "string" ||
    typeof payload.verb !== "string" ||
    typeof payload.detail !== "string" ||
    typeof payload.channel !== "string" ||
    typeof payload.received_at !== "string"
  ) {
    return null;
  }
  const channel: ChannelKind = KNOWN_CHANNELS.has(payload.channel)
    ? (payload.channel as ChannelKind)
    : "web";
  return {
    id: payload.id,
    worker: {
      name: payload.worker.name,
      avatarUrl: payload.worker.avatar_url ?? undefined,
    },
    verb: payload.verb,
    detail: payload.detail,
    consequenceIfYes: payload.consequence_if_yes ?? "",
    consequenceIfNo: payload.consequence_if_no ?? "",
    reason: payload.reason ?? undefined,
    channel,
    receivedAt: payload.received_at,
    decision: null,
  };
}

export function applyEvent(qc: QueryClient, evt: WireEvent): void {
  if (evt.type === "hello") return;
  if (evt.type === "approval.created") {
    const item = wireToApprovalItem(evt.payload);
    if (!item) {
      void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
      return;
    }
    qc.setQueryData<ApprovalsResult>(APPROVALS_KEY, (prev) => {
      if (!prev) {
        // Cache hasn't loaded yet — invalidate so the first fetch
        // grabs the latest list including this row.
        void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
        return prev;
      }
      if (prev.items.some((i) => i.id === item.id)) return prev; // dedupe
      return { ...prev, items: [item, ...prev.items] };
    });
    return;
  }
  const id = evt.payload?.approval_id;
  if (!id) return;
  qc.setQueryData<ApprovalsResult>(APPROVALS_KEY, (prev) => {
    if (!prev) return prev;
    if (evt.type === "approval.expired") {
      return { ...prev, items: prev.items.filter((i) => i.id !== id) };
    }
    const decision = evt.type === "approval.decided" ? evt.decision : null;
    return {
      ...prev,
      items: prev.items.map((i) =>
        i.id === id ? { ...i, decision } : i,
      ),
    };
  });
}

export interface UseApprovalsLiveResult {
  status: LiveStatus;
  lastEventAt: Date | null;
}

/**
 * Subscribe to `/inbox/ws` and stream approval events into the
 * TanStack Query cache. Returns the connection status + the timestamp
 * of the last event for diagnostics.
 *
 * @example
 * ```tsx
 * function Inbox() {
 *   const { status } = useApprovalsLive();
 *   return <LiveIndicator status={status} />;
 * }
 * ```
 */
export function useApprovalsLive(): UseApprovalsLiveResult {
  const qc = useQueryClient();
  const [status, setStatus] = useState<LiveStatus>("connecting");
  const [lastEventAt, setLastEventAt] = useState<Date | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;

    let cancelled = false;

    const conn = new ManagedWebSocket<WireEvent>({
      url: async () => gatewayWsUrl(await readSessionToken()),
      onMessage: (evt) => {
        if (cancelled) return;
        setLastEventAt(new Date());
        applyEvent(qc, evt);
      },
      onStatusChange: (s) => {
        if (cancelled) return;
        const mapped = mapStatus(s);
        setStatus(mapped);
        // On polling-fallback, force a refetch so the list isn't
        // frozen on whatever the last live snapshot was. Subsequent
        // refetches happen on tab-focus + TanStack's stale-time, so
        // we don't need a long-running polling interval here.
        if (s === "polling") {
          void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
        }
      },
    });

    // Tab-focus catches up after a possible throttled-tab period.
    const handleFocus = () => {
      void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
    };
    window.addEventListener("focus", handleFocus);

    conn.start();

    return () => {
      cancelled = true;
      window.removeEventListener("focus", handleFocus);
      conn.stop();
    };
  }, [qc]);

  return { status, lastEventAt };
}
