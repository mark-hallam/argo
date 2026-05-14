import type { Edge, Node, SkillGraph } from "@/lib/types/skill-ir";

export type DiffState = "unchanged" | "added" | "removed" | "modified" | "rewired";

export interface NodeDiff {
  id: string;
  state: Exclude<DiffState, "rewired">;
  left?: Node;
  right?: Node;
  changedFields: string[];
}

export interface EdgeDiff {
  id: string;
  state: DiffState;
  left?: Edge;
  right?: Edge;
  changedFields: string[];
}

export interface AnnotationDiff {
  nodeId: string;
  left?: Node["annotations"];
  right?: Node["annotations"];
}

export interface SkillGraphDiffSummary {
  nodesAdded: number;
  nodesRemoved: number;
  nodesModified: number;
  edgesAdded: number;
  edgesRemoved: number;
  edgesModified: number;
  edgesRewired: number;
  annotationsChanged: number;
}

export interface SkillGraphDiff {
  summary: SkillGraphDiffSummary;
  nodeDiffs: NodeDiff[];
  edgeDiffs: EdgeDiff[];
  annotationDiffs: AnnotationDiff[];
}

export function diffSkillGraphs(left: SkillGraph, right: SkillGraph): SkillGraphDiff {
  const nodeDiffs = diffNodes(left.nodes, right.nodes);
  const edgeDiffs = diffEdges(left.edges, right.edges);
  const annotationDiffs = diffAnnotations(left.nodes, right.nodes);
  return {
    summary: {
      nodesAdded: countState(nodeDiffs, "added"),
      nodesRemoved: countState(nodeDiffs, "removed"),
      nodesModified: countState(nodeDiffs, "modified"),
      edgesAdded: countState(edgeDiffs, "added"),
      edgesRemoved: countState(edgeDiffs, "removed"),
      edgesModified: countState(edgeDiffs, "modified"),
      edgesRewired: countState(edgeDiffs, "rewired"),
      annotationsChanged: annotationDiffs.length,
    },
    nodeDiffs,
    edgeDiffs,
    annotationDiffs,
  };
}

export function formatDiffSummary(summary: SkillGraphDiffSummary): string {
  const parts = [
    plural(summary.nodesAdded, "node added"),
    plural(summary.nodesRemoved, "node removed"),
    plural(summary.nodesModified, "node modified"),
    plural(summary.edgesAdded, "edge added"),
    plural(summary.edgesRemoved, "edge removed"),
    plural(summary.edgesRewired, "edge rewired"),
    plural(summary.edgesModified, "edge condition changed"),
    plural(summary.annotationsChanged, "annotation shifted"),
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(", ") : "No structural changes";
}

function diffNodes(leftNodes: Node[], rightNodes: Node[]): NodeDiff[] {
  const leftById = mapById(leftNodes);
  const rightById = mapById(rightNodes);
  const ids = sortedUnion(leftById, rightById);
  return ids
    .map((id): NodeDiff => {
      const left = leftById.get(id);
      const right = rightById.get(id);
      if (!left) return { id, state: "added", right, changedFields: ["node"] };
      if (!right) return { id, state: "removed", left, changedFields: ["node"] };
      const changedFields = [
        left.label === right.label ? "" : "label",
        stableStringify(left.kind) === stableStringify(right.kind) ? "" : "kind",
      ].filter(Boolean);
      return {
        id,
        state: changedFields.length > 0 ? "modified" : "unchanged",
        left,
        right,
        changedFields,
      };
    })
    .filter((d) => d.state !== "unchanged");
}

function diffEdges(leftEdges: Edge[], rightEdges: Edge[]): EdgeDiff[] {
  const leftById = mapById(leftEdges);
  const rightById = mapById(rightEdges);
  const ids = sortedUnion(leftById, rightById);
  return ids
    .map((id): EdgeDiff => {
      const left = leftById.get(id);
      const right = rightById.get(id);
      if (!left) return { id, state: "added", right, changedFields: ["edge"] };
      if (!right) return { id, state: "removed", left, changedFields: ["edge"] };
      const rewired = left.from !== right.from || left.to !== right.to;
      const condChanged = stableStringify(left.cond) !== stableStringify(right.cond);
      const changedFields = [
        rewired ? "endpoints" : "",
        condChanged ? "condition" : "",
      ].filter(Boolean);
      return {
        id,
        state: rewired ? "rewired" : condChanged ? "modified" : "unchanged",
        left,
        right,
        changedFields,
      };
    })
    .filter((d) => d.state !== "unchanged");
}

function diffAnnotations(leftNodes: Node[], rightNodes: Node[]): AnnotationDiff[] {
  const leftById = mapById(leftNodes);
  const rightById = mapById(rightNodes);
  return sortedUnion(leftById, rightById)
    .map((nodeId): AnnotationDiff | null => {
      const left = leftById.get(nodeId)?.annotations;
      const right = rightById.get(nodeId)?.annotations;
      if (!left || !right) return null;
      if (stableStringify(left) === stableStringify(right)) return null;
      return { nodeId, left, right };
    })
    .filter((d): d is AnnotationDiff => Boolean(d));
}

function mapById<T extends Node | Edge>(items: T[]): Map<string, T> {
  return new Map(items.map((item) => [item.id, item]));
}

function sortedUnion<T>(left: Map<string, T>, right: Map<string, T>): string[] {
  return Array.from(new Set([...left.keys(), ...right.keys()])).sort();
}

function countState<T extends { state: DiffState }>(items: T[], state: DiffState): number {
  return items.filter((item) => item.state === state).length;
}

function plural(count: number, label: string): string | null {
  if (count === 0) return null;
  return `${count} ${label}${count === 1 ? "" : "s"}`;
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(",")}}`;
}
