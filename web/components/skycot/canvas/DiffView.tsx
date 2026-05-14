"use client";

import { GitCompareArrows } from "lucide-react";
import { useMemo } from "react";
import {
  diffSkillGraphs,
  formatDiffSummary,
  type DiffState,
  type EdgeDiff,
  type NodeDiff,
} from "./diff-engine";
import { KIND_VISUALS } from "./NodeRenderer";
import { cn } from "@/lib/utils";
import type { Edge, Node, SkillGraph } from "@/lib/types/skill-ir";

interface DiffViewProps {
  leftGraph: SkillGraph;
  rightGraph: SkillGraph;
  leftVersion: string;
  rightVersion: string;
}

export function DiffView({
  leftGraph,
  rightGraph,
  leftVersion,
  rightVersion,
}: DiffViewProps) {
  const diff = useMemo(
    () => diffSkillGraphs(leftGraph, rightGraph),
    [leftGraph, rightGraph],
  );
  const nodeRows = useMemo(() => alignNodes(leftGraph, rightGraph, diff.nodeDiffs), [
    leftGraph,
    rightGraph,
    diff.nodeDiffs,
  ]);
  const edgeRows = useMemo(() => alignEdges(leftGraph, rightGraph, diff.edgeDiffs), [
    leftGraph,
    rightGraph,
    diff.edgeDiffs,
  ]);

  return (
    <section data-testid="skill-canvas-diff" className="space-y-4">
      <header className="flex flex-col gap-3 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium text-[var(--muted-foreground)]">
            <GitCompareArrows className="size-4" aria-hidden="true" />
            <span>Version diff</span>
          </div>
          <h1 className="mt-1 text-xl font-semibold text-[var(--foreground)]">
            {leftVersion} vs {rightVersion}
          </h1>
        </div>
        <p
          data-testid="diff-summary"
          className="max-w-3xl text-sm text-[var(--muted-foreground)]"
        >
          {formatDiffSummary(diff.summary)}
        </p>
      </header>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <DiffPane
          title={leftVersion}
          side="left"
          nodes={nodeRows.map((row) => row.left)}
          edges={edgeRows.map((row) => row.left)}
        />
        <DiffPane
          title={rightVersion}
          side="right"
          nodes={nodeRows.map((row) => row.right)}
          edges={edgeRows.map((row) => row.right)}
        />
      </div>
    </section>
  );
}

interface AlignedNode {
  left: NodeCell;
  right: NodeCell;
}

interface AlignedEdge {
  left: EdgeCell;
  right: EdgeCell;
}

interface NodeCell {
  id: string;
  node?: Node;
  state: DiffState;
  changedFields: string[];
}

interface EdgeCell {
  id: string;
  edge?: Edge;
  state: DiffState;
  changedFields: string[];
}

function alignNodes(left: SkillGraph, right: SkillGraph, diffs: NodeDiff[]): AlignedNode[] {
  const changed = new Map(diffs.map((d) => [d.id, d]));
  const leftById = new Map(left.nodes.map((node) => [node.id, node]));
  const rightById = new Map(right.nodes.map((node) => [node.id, node]));
  const ids = Array.from(new Set([...leftById.keys(), ...rightById.keys()])).sort();
  return ids.map((id) => {
    const diff = changed.get(id);
    return {
      left: {
        id,
        node: leftById.get(id),
        state: diff?.state === "removed" ? "removed" : "unchanged",
        changedFields: diff?.changedFields ?? [],
      },
      right: {
        id,
        node: rightById.get(id),
        state: diff?.state ?? "unchanged",
        changedFields: diff?.changedFields ?? [],
      },
    };
  });
}

function alignEdges(left: SkillGraph, right: SkillGraph, diffs: EdgeDiff[]): AlignedEdge[] {
  const changed = new Map(diffs.map((d) => [d.id, d]));
  const leftById = new Map(left.edges.map((edge) => [edge.id, edge]));
  const rightById = new Map(right.edges.map((edge) => [edge.id, edge]));
  const ids = Array.from(new Set([...leftById.keys(), ...rightById.keys()])).sort();
  return ids.map((id) => {
    const diff = changed.get(id);
    return {
      left: {
        id,
        edge: leftById.get(id),
        state: diff?.state === "removed" ? "removed" : "unchanged",
        changedFields: diff?.changedFields ?? [],
      },
      right: {
        id,
        edge: rightById.get(id),
        state: diff?.state ?? "unchanged",
        changedFields: diff?.changedFields ?? [],
      },
    };
  });
}

function DiffPane({
  title,
  side,
  nodes,
  edges,
}: {
  title: string;
  side: "left" | "right";
  nodes: NodeCell[];
  edges: EdgeCell[];
}) {
  return (
    <div
      data-testid={`diff-pane-${side}`}
      className="min-w-0 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)]"
    >
      <div className="border-b border-[var(--border)] px-4 py-3">
        <h2 className="text-sm font-semibold text-[var(--foreground)]">{title}</h2>
      </div>
      <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_minmax(260px,0.75fr)]">
        <div className="min-w-0 space-y-2">
          <h3 className="text-xs font-medium uppercase text-[var(--muted-foreground)]">
            Nodes
          </h3>
          {nodes.map((cell) => (
            <NodeDiffCard key={cell.id} cell={cell} side={side} />
          ))}
        </div>
        <div className="min-w-0 space-y-2">
          <h3 className="text-xs font-medium uppercase text-[var(--muted-foreground)]">
            Edges
          </h3>
          {edges.length > 0 ? (
            edges.map((cell) => <EdgeDiffCard key={cell.id} cell={cell} side={side} />)
          ) : (
            <p className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] p-3 text-sm text-[var(--muted-foreground)]">
              No edges
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function NodeDiffCard({ cell, side }: { cell: NodeCell; side: "left" | "right" }) {
  const visual = cell.node ? KIND_VISUALS[cell.node.kind.type] : null;
  const Icon = visual?.Icon;
  return (
    <div
      data-testid={`diff-node-${side}-${cell.id}`}
      data-state={cell.state}
      className={cn(
        "rounded-[var(--radius-md)] border px-3 py-2 text-sm",
        stateClass(cell.state, side),
      )}
    >
      <div className="flex items-start gap-2">
        {Icon ? (
          <Icon className={cn("mt-0.5 size-4 shrink-0", visual?.iconClass)} aria-hidden="true" />
        ) : (
          <span className="mt-1 size-2 shrink-0 rounded-full bg-[var(--muted-foreground)]" />
        )}
        <div className="min-w-0">
          <div className="font-medium text-[var(--foreground)]">
            {cell.node?.label ?? "Missing in this version"}
          </div>
          <div className="text-xs text-[var(--muted-foreground)]">
            {cell.node?.kind.type ?? cell.id}
            {cell.changedFields.length > 0 ? ` · ${cell.changedFields.join(", ")}` : ""}
          </div>
        </div>
      </div>
    </div>
  );
}

function EdgeDiffCard({ cell, side }: { cell: EdgeCell; side: "left" | "right" }) {
  return (
    <div
      data-testid={`diff-edge-${side}-${cell.id}`}
      data-state={cell.state}
      className={cn(
        "rounded-[var(--radius-md)] border px-3 py-2 text-sm",
        stateClass(cell.state, side),
      )}
    >
      <div className="font-medium text-[var(--foreground)]">
        {cell.edge ? `${cell.edge.from} → ${cell.edge.to}` : "Missing in this version"}
      </div>
      <div className="text-xs text-[var(--muted-foreground)]">
        {cell.edge ? edgeCondLabel(cell.edge) : cell.id}
        {cell.changedFields.length > 0 ? ` · ${cell.changedFields.join(", ")}` : ""}
      </div>
    </div>
  );
}

function edgeCondLabel(edge: Edge): string {
  if (edge.cond.kind === "on-pattern") return `pattern: ${edge.cond.pattern}`;
  if (edge.cond.kind === "always") return "always";
  return edge.cond.kind.replace("on-", "");
}

function stateClass(state: DiffState, side: "left" | "right"): string {
  if (side === "left") {
    return "border-[var(--border)] bg-[var(--muted)]/30 text-[var(--muted-foreground)]";
  }
  if (state === "added") return "border-emerald-300 bg-emerald-50 dark:border-emerald-700 dark:bg-emerald-950/30";
  if (state === "removed") return "border-rose-300 bg-rose-50 dark:border-rose-700 dark:bg-rose-950/30";
  if (state === "modified" || state === "rewired") {
    return "border-amber-300 bg-amber-50 dark:border-amber-700 dark:bg-amber-950/30";
  }
  return "border-[var(--border)] bg-[var(--card)]";
}
