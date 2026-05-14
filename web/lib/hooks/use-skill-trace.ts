"use client";

/**
 * Live-trace hook for the Skill Canvas overlay (Track L L5).
 *
 * Opens a WebSocket to the gateway's `/skill-trace/<skillId>/<runId>`
 * route and tracks three lifecycle moments per node: which node is
 * currently executing, which nodes have completed, and which have
 * failed. Returns the running state plus the underlying connection
 * status so the canvas can render a live indicator.
 *
 * Reuses {@link ManagedWebSocket} so the same auto-reconnect /
 * heartbeat / polling-fallback machinery driving the approvals inbox
 * applies here — disconnect-and-reconnect mid-run hand-off resumes
 * cleanly because the runtime keeps publishing to the per-topic
 * broadcast channel.
 *
 * Returns `{ status, activeNodeId, completedNodes, failedNodes,
 * lastEventAt }`. `completedNodes` is the set of node ids the worker
 * finished; `failedNodes` maps node id → human-readable error so the
 * overlay can show a tooltip without re-fetching.
 */

import { useEffect, useState } from "react";
import { ManagedWebSocket, type WebSocketStatus } from "@/lib/api/websocket";

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

/** UI-facing connection state surfaced by the overlay. */
export type SkillTraceStatus = "connecting" | "live" | "polling" | "offline";

/** Wire-format event vocabulary (matches `ws_skill_trace` envelope). */
export type SkillTraceWireEvent =
  | {
      type: "hello";
      protocol_version?: string;
      topic?: string;
      capabilities?: string[];
      seq?: number;
    }
  | { type: "node.entered"; node_id: string; seq?: number; ts?: string }
  | {
      type: "node.completed";
      node_id: string;
      duration_ms?: number;
      seq?: number;
      ts?: string;
    }
  | {
      type: "node.failed";
      node_id: string;
      error?: string;
      seq?: number;
      ts?: string;
    };

/** Trace state surfaced to the canvas overlay. */
export interface SkillTraceState {
  status: SkillTraceStatus;
  activeNodeId: string | null;
  completedNodes: ReadonlySet<string>;
  failedNodes: ReadonlyMap<string, string>;
  lastEventAt: Date | null;
}

const INITIAL_STATE: SkillTraceState = {
  status: "connecting",
  activeNodeId: null,
  completedNodes: new Set<string>(),
  failedNodes: new Map<string, string>(),
  lastEventAt: null,
};

function mapStatus(s: WebSocketStatus): SkillTraceStatus {
  switch (s) {
    case "open":
      return "live";
    case "polling":
      return "polling";
    case "connecting":
      return "connecting";
    case "closed":
      return "offline";
  }
}

function gatewayWsUrl(
  skillId: string,
  runId: string,
  token: string | undefined,
): string {
  const base = process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
  const wsBase = base.replace(/^http(s?):/, "ws$1:");
  const qs = token ? `?token=${encodeURIComponent(token)}` : "";
  return `${wsBase}/skill-trace/${encodeURIComponent(
    skillId,
  )}/${encodeURIComponent(runId)}${qs}`;
}

async function readSessionToken(): Promise<string | undefined> {
  if (typeof window === "undefined") return undefined;
  try {
    const res = await fetch("/api/auth/session", {
      credentials: "same-origin",
    });
    if (!res.ok) return undefined;
    const session = (await res.json()) as { gatewayToken?: string };
    return session?.gatewayToken;
  } catch {
    return undefined;
  }
}

/**
 * Apply one wire event to the prior trace state. Pure — exported so
 * unit tests can drive the state machine without standing up a real
 * WebSocket. The hello frame is a no-op (the brief reserves `seq: 0`
 * as the handshake sentinel; the SPA needs nothing else from it for
 * v1).
 */
export function applyTraceEvent(
  prev: SkillTraceState,
  evt: SkillTraceWireEvent,
): SkillTraceState {
  if (evt.type === "hello") return prev;
  const lastEventAt = new Date();
  if (evt.type === "node.entered") {
    return { ...prev, activeNodeId: evt.node_id, lastEventAt };
  }
  if (evt.type === "node.completed") {
    const next = new Set(prev.completedNodes);
    next.add(evt.node_id);
    return {
      ...prev,
      activeNodeId: prev.activeNodeId === evt.node_id ? null : prev.activeNodeId,
      completedNodes: next,
      lastEventAt,
    };
  }
  if (evt.type === "node.failed") {
    const next = new Map(prev.failedNodes);
    next.set(evt.node_id, evt.error ?? "node failed");
    return {
      ...prev,
      activeNodeId: prev.activeNodeId === evt.node_id ? null : prev.activeNodeId,
      failedNodes: next,
      lastEventAt,
    };
  }
  return prev;
}

/**
 * Subscribe to live trace events for one skill run. The hook owns
 * its own WebSocket lifecycle — open on mount, close on unmount.
 * Passing `null` for either argument disables the subscription
 * (useful when the operator isn't yet viewing a live run).
 *
 * @example
 * ```tsx
 * function CanvasLive({ skillId, runId }: Props) {
 *   const trace = useSkillTrace(skillId, runId);
 *   return <LiveTraceOverlay trace={trace} />;
 * }
 * ```
 */
export function useSkillTrace(
  skillId: string | null,
  runId: string | null,
): SkillTraceState {
  const [state, setState] = useState<SkillTraceState>(INITIAL_STATE);

  useEffect(() => {
    if (typeof window === "undefined") return;
    if (!skillId || !runId) {
      setState(INITIAL_STATE);
      return;
    }

    let cancelled = false;
    setState(INITIAL_STATE);

    const conn = new ManagedWebSocket<SkillTraceWireEvent>({
      url: async () => gatewayWsUrl(skillId, runId, await readSessionToken()),
      onMessage: (evt) => {
        if (cancelled) return;
        setState((prev) => applyTraceEvent(prev, evt));
      },
      onStatusChange: (s) => {
        if (cancelled) return;
        setState((prev) => ({ ...prev, status: mapStatus(s) }));
      },
    });

    conn.start();

    return () => {
      cancelled = true;
      conn.stop();
    };
  }, [skillId, runId]);

  return state;
}
