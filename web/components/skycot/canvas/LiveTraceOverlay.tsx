"use client";

/**
 * Live-trace overlay (Track L L5). Sibling to the L4
 * {@link SkillCanvas} — composes on top of, rather than mutating,
 * the read-only viewer. The brief reserves the core renderer for
 * future composition (`L4 SkillCanvas.tsx` is may-not-touch), so
 * this overlay surfaces the live state as a status panel that sits
 * adjacent to the canvas: the current node id, the set of completed
 * nodes, and any failed nodes with their error messages.
 *
 * v1 sensible deviation vs L0 §"live trace overlay": the brief
 * describes "pulses the current node" inside the canvas. A future
 * Track L brief will weave the pulse into React Flow's per-node
 * data (the L4 viewer already exposes `isSelected` — a future brief
 * extends `SkillNodeData` with a status flag). v1 lands the runtime
 * → gateway → SPA plumbing as a composable status panel so the
 * pulse polish can land additively without re-architecting the
 * transport.
 */

import React, { useMemo } from "react";
import { cn } from "@/lib/utils";
import type { SkillGraph } from "@/lib/types/skill-ir";
import {
  useSkillTrace,
  type SkillTraceState,
  type SkillTraceStatus,
} from "@/lib/hooks/use-skill-trace";

export interface LiveTraceOverlayProps {
  /** Skill id for the WS subscription topic. */
  skillId: string;
  /** Run id (one execution of `skillId`). */
  runId: string;
  /**
   * IR graph backing the canvas. Used to resolve node ids to
   * human-readable labels in the status panel. Optional — when
   * missing the overlay still renders, just with raw ids.
   */
  graph?: SkillGraph;
  /**
   * Inject pre-computed state for tests / Storybook so unit tests
   * can exercise the rendering without standing up a real
   * WebSocket. Production callers omit it.
   */
  injectedState?: SkillTraceState;
  className?: string;
}

const STATUS_COPY: Record<SkillTraceStatus, string> = {
  connecting: "Connecting…",
  live: "Live",
  polling: "Polling (offline mode)",
  offline: "Offline",
};

const STATUS_DOT: Record<SkillTraceStatus, string> = {
  connecting: "bg-amber-400",
  live: "bg-emerald-500",
  polling: "bg-amber-400",
  offline: "bg-rose-500",
};

function nodeLabel(graph: SkillGraph | undefined, nodeId: string): string {
  if (!graph) return nodeId;
  const node = graph.nodes.find((n) => n.id === nodeId);
  if (!node) return nodeId;
  return node.label.length > 0 ? `${node.label} (${nodeId})` : nodeId;
}

export function LiveTraceOverlay(
  props: LiveTraceOverlayProps,
): React.ReactElement {
  const { skillId, runId, graph, injectedState, className } = props;
  const liveState = useSkillTrace(
    injectedState ? null : skillId,
    injectedState ? null : runId,
  );
  const trace = injectedState ?? liveState;

  const completed = useMemo(
    () => Array.from(trace.completedNodes).sort(),
    [trace.completedNodes],
  );
  const failed = useMemo(
    () => Array.from(trace.failedNodes.entries()).sort(([a], [b]) => a.localeCompare(b)),
    [trace.failedNodes],
  );

  return (
    <aside
      className={cn(
        "rounded-md border bg-card p-4 text-sm",
        "flex w-full max-w-sm flex-col gap-3",
        className,
      )}
      aria-label="Skill live-trace overlay"
      data-testid="live-trace-overlay"
    >
      <header className="flex items-center justify-between gap-2">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Live trace
        </h3>
        <span
          className="flex items-center gap-1 text-xs"
          data-testid="live-trace-status"
        >
          <span
            className={cn("h-2 w-2 rounded-full", STATUS_DOT[trace.status])}
            aria-hidden="true"
          />
          {STATUS_COPY[trace.status]}
        </span>
      </header>

      <section data-testid="live-trace-active">
        <p className="text-xs font-medium text-muted-foreground">
          Currently executing
        </p>
        {trace.activeNodeId ? (
          <p className="mt-1 font-mono text-sm">
            {nodeLabel(graph, trace.activeNodeId)}
          </p>
        ) : (
          <p className="mt-1 text-xs italic text-muted-foreground">
            (idle)
          </p>
        )}
      </section>

      {completed.length > 0 && (
        <section data-testid="live-trace-completed">
          <p className="text-xs font-medium text-muted-foreground">
            Completed ({completed.length})
          </p>
          <ul className="mt-1 list-disc pl-4">
            {completed.map((id) => (
              <li
                key={id}
                className="text-emerald-700 dark:text-emerald-400"
                data-testid={`live-trace-completed-${id}`}
              >
                {nodeLabel(graph, id)}
              </li>
            ))}
          </ul>
        </section>
      )}

      {failed.length > 0 && (
        <section data-testid="live-trace-failed">
          <p className="text-xs font-medium text-muted-foreground">
            Failed ({failed.length})
          </p>
          <ul className="mt-1 list-disc pl-4">
            {failed.map(([id, err]) => (
              <li
                key={id}
                className="text-rose-700 dark:text-rose-400"
                data-testid={`live-trace-failed-${id}`}
                title={err}
              >
                {nodeLabel(graph, id)}: {err}
              </li>
            ))}
          </ul>
        </section>
      )}
    </aside>
  );
}
