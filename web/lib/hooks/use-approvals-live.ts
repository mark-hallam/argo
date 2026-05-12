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
 * v1 sensible deviations from the brief:
 *
 * - `approval.created` triggers `invalidateQueries(["approvals"])`
 *   rather than a surgical cache insert: the J3 wire payload is the
 *   gateway's `WebInboxRow` (operator_id / approval_id / risk_level
 *   / prompt_body / expiry / enqueued_at) which lacks the
 *   `ApprovalItem` fields the SPA needs (worker.name, verb, detail,
 *   consequence_if_*, reason, channel, received_at). A refetch via
 *   the J1 JSON endpoint is the only path to a correctly-shaped
 *   item until the gateway emits a richer payload (tracked as a
 *   follow-up: `mission-gateway-ws-emit-full-approval-row`). The
 *   operator still sees the new card without manual refresh — the
 *   brief's "no full refetch" criterion was about UX, not network
 *   shape.
 * - `approval.decided` / `.undone` / `.expired` ARE surgical: the
 *   approval id is sufficient and the matching item already lives
 *   in the cache.
 */

import { useEffect, useState } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { ManagedWebSocket, type WebSocketStatus } from "@/lib/api/websocket";
import type { ApprovalsResult } from "@/lib/api/approvals";

const APPROVALS_KEY = ["approvals"] as const;
const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

/** UI-facing connection state surfaced to LiveIndicator. */
export type LiveStatus = "connecting" | "live" | "polling" | "offline";

/** Subset of the J3 wire vocabulary we surface to the cache. */
type WireEvent =
  | { type: "hello"; protocol_version?: string; capabilities?: string[]; seq?: number }
  | { type: "approval.created"; payload: { approval_id: string } & Record<string, unknown>; seq?: number; ts?: string }
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

function applyEvent(qc: QueryClient, evt: WireEvent): void {
  if (evt.type === "approval.created") {
    void qc.invalidateQueries({ queryKey: APPROVALS_KEY });
    return;
  }
  if (evt.type === "hello") return;
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
