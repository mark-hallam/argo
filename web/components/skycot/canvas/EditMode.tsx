"use client";

import { useMemo, useState } from "react";
import { Trash2 } from "lucide-react";
import type {
  Edge,
  EdgeCond,
  Node,
  NodeKind,
  NodePolicy,
  SkillGraph,
} from "@/lib/types/skill-ir";
import { saveSkillIr, type SaveSkillIrResult } from "@/lib/api/skill-ir-save";
import { cn } from "@/lib/utils";
import { NodePalette } from "./NodePalette";
import { SaveBar } from "./SaveBar";
import { KIND_VISUALS, type NodeKindTag } from "./NodeRenderer";

export interface EditModeProps {
  skillId: string;
  graph: SkillGraph;
  initialVersion: string;
  saveGraph?: (skillId: string, graph: SkillGraph) => Promise<SaveSkillIrResult>;
}

export function validateSkillGraphForSave(graph: SkillGraph): string | null {
  const ids = new Set(graph.nodes.map((node) => node.id));
  if (!ids.has(graph.entry)) return "Graph must have an entry node.";
  const out = new Map(graph.nodes.map((node) => [node.id, 0]));
  const adjacency = new Map(graph.nodes.map((node) => [node.id, [] as string[]]));
  for (const edge of graph.edges) {
    if (!ids.has(edge.from) || !ids.has(edge.to)) {
      return "Every edge must connect existing nodes.";
    }
    out.set(edge.from, (out.get(edge.from) ?? 0) + 1);
    adjacency.get(edge.from)?.push(edge.to);
  }
  for (const node of graph.nodes) {
    for (const target of structuralTargets(node.kind)) {
      if (!ids.has(target)) return "Every structural body must reference existing nodes.";
      out.set(node.id, (out.get(node.id) ?? 0) + 1);
      adjacency.get(node.id)?.push(target);
    }
  }
  if (![...out.values()].some((count) => count === 0)) {
    return "Graph must have at least one terminal node.";
  }
  const seen = new Set<string>();
  const queue = [graph.entry];
  while (queue.length) {
    const id = queue.shift() as string;
    if (seen.has(id)) continue;
    seen.add(id);
    queue.push(...(adjacency.get(id) ?? []));
  }
  if (seen.size !== graph.nodes.length) {
    return "Graph has orphaned nodes.";
  }
  return null;
}

export function EditMode({
  skillId,
  graph,
  initialVersion,
  saveGraph = saveSkillIr,
}: EditModeProps) {
  const [draft, setDraft] = useState<SkillGraph>(() => cloneGraph(graph));
  const [source, setSource] = useState(graph.entry);
  const [target, setTarget] = useState(graph.nodes[1]?.id ?? graph.entry);
  const [versions, setVersions] = useState([initialVersion]);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [abScheduled, setAbScheduled] = useState(false);

  const validation = useMemo(() => validateSkillGraphForSave(draft), [draft]);

  function mutate(next: SkillGraph) {
    setDraft(next);
    setDirty(true);
    setError(null);
  }

  async function onSave() {
    const err = validateSkillGraphForSave(draft);
    if (err) {
      setError(err);
      return;
    }
    setSaving(true);
    try {
      const saved = await saveGraph(skillId, draft);
      setVersions((prev) => [saved.version, ...prev].slice(0, 5));
      setAbScheduled(saved.abTestScheduled);
      setDirty(false);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save failed");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div
      data-testid="skill-canvas-edit-mode"
      className="overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]"
    >
      <NodePalette onAdd={(kind) => mutate(addNode(draft, kind))} />
      <div className="grid gap-0 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="min-h-[460px] p-3">
          <GraphList
            graph={draft}
            onDelete={(id) => mutate(deleteNode(draft, id))}
            onPolicy={(id, policy) => mutate(updatePolicy(draft, id, policy))}
            onTool={(id, raw) => mutate(updateToolBinding(draft, id, raw))}
          />
        </div>
        <aside className="border-t border-[var(--border)] p-3 lg:border-l lg:border-t-0">
          <EdgeControls
            graph={draft}
            source={source}
            target={target}
            onSource={setSource}
            onTarget={setTarget}
            onConnect={() => mutate(connectNodes(draft, source, target))}
          />
          {validation ? (
            <p data-testid="topology-warning" className="mt-3 text-xs text-[var(--destructive)]">
              {validation}
            </p>
          ) : null}
        </aside>
      </div>
      <SaveBar
        dirty={dirty}
        saving={saving}
        error={error}
        versions={versions}
        abScheduled={abScheduled}
        onSave={onSave}
      />
    </div>
  );
}

function GraphList({
  graph,
  onDelete,
  onPolicy,
  onTool,
}: {
  graph: SkillGraph;
  onDelete: (id: string) => void;
  onPolicy: (id: string, policy: NodePolicy) => void;
  onTool: (id: string, raw: string) => void;
}) {
  return (
    <ul data-testid="edit-node-list" className="grid gap-2">
      {graph.nodes.map((node) => {
        const visual = KIND_VISUALS[node.kind.type];
        return (
          <li
            key={node.id}
            data-testid={`edit-node-${node.id}`}
            className={cn(
              "rounded-[var(--radius-md)] border p-3",
              visual.bodyClass,
              visual.borderClass,
            )}
          >
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[10px] uppercase text-[var(--muted-foreground)]">
                  {visual.label}
                </div>
                <div className="truncate text-sm font-medium">{node.label}</div>
              </div>
              {node.id === graph.entry ? null : (
                <button
                  type="button"
                  aria-label={`Delete ${node.label}`}
                  data-testid={`delete-node-${node.id}`}
                  onClick={() => onDelete(node.id)}
                  className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--background)]"
                >
                  <Trash2 className="size-4" aria-hidden="true" />
                </button>
              )}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              <label className="text-xs text-[var(--muted-foreground)]">
                Policy{" "}
                <select
                  data-testid={`policy-${node.id}`}
                  value={node.annotations.policy ?? "auto"}
                  onChange={(e) => onPolicy(node.id, e.target.value as NodePolicy)}
                  className="rounded border border-[var(--border)] bg-[var(--background)] px-1 py-0.5"
                >
                  <option value="auto">Auto</option>
                  <option value="ask">Ask</option>
                </select>
              </label>
              {node.kind.type === "action" || node.kind.type === "fetch" ? (
                <label className="text-xs text-[var(--muted-foreground)]">
                  Tool{" "}
                  <input
                    data-testid={`tool-${node.id}`}
                    defaultValue={formatTool(node)}
                    onBlur={(e) => onTool(node.id, e.target.value)}
                    placeholder="server.tool"
                    className="w-36 rounded border border-[var(--border)] bg-[var(--background)] px-1 py-0.5"
                  />
                </label>
              ) : null}
            </div>
          </li>
        );
      })}
    </ul>
  );
}

function EdgeControls({
  graph,
  source,
  target,
  onSource,
  onTarget,
  onConnect,
}: {
  graph: SkillGraph;
  source: string;
  target: string;
  onSource: (id: string) => void;
  onTarget: (id: string) => void;
  onConnect: () => void;
}) {
  return (
    <div className="space-y-2">
      <div className="text-xs font-medium text-[var(--foreground)]">Edges</div>
      <select
        data-testid="edge-source"
        value={source}
        onChange={(e) => onSource(e.target.value)}
        className="w-full rounded border border-[var(--border)] bg-[var(--background)] px-2 py-1 text-sm"
      >
        {graph.nodes.map((node) => <option key={node.id}>{node.id}</option>)}
      </select>
      <select
        data-testid="edge-target"
        value={target}
        onChange={(e) => onTarget(e.target.value)}
        className="w-full rounded border border-[var(--border)] bg-[var(--background)] px-2 py-1 text-sm"
      >
        {graph.nodes.map((node) => <option key={node.id}>{node.id}</option>)}
      </select>
      <button
        type="button"
        data-testid="connect-nodes"
        onClick={onConnect}
        className="w-full rounded-[var(--radius-md)] border border-[var(--border)] px-2 py-2 text-sm hover:bg-[var(--accent)]/40"
      >
        Connect
      </button>
      <ul data-testid="edge-list" className="pt-2 text-xs text-[var(--muted-foreground)]">
        {graph.edges.map((edge) => (
          <li key={edge.id}>
            {edge.from} {"->"} {edge.to}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function addNode(graph: SkillGraph, kind: NodeKindTag): SkillGraph {
  const id = `n_${kind.replaceAll("-", "_")}_${graph.nodes.length + 1}`;
  const node: Node = {
    id,
    label: KIND_VISUALS[kind].label,
    kind: defaultKind(kind),
    annotations: { policy: "ask" },
  };
  return { ...graph, nodes: [...graph.nodes, node] };
}

export function deleteNode(graph: SkillGraph, id: string): SkillGraph {
  return {
    ...graph,
    nodes: graph.nodes
      .filter((node) => node.id !== id)
      .map((node) => ({ ...node, kind: deleteStructuralTarget(node.kind, id) })),
    edges: graph.edges.filter((edge) => edge.from !== id && edge.to !== id),
  };
}

export function connectNodes(graph: SkillGraph, from: string, to: string): SkillGraph {
  if (from === to || graph.edges.some((edge) => edge.from === from && edge.to === to)) {
    return graph;
  }
  const cond: EdgeCond = { kind: "always" };
  return {
    ...graph,
    edges: [...graph.edges, { id: `e_${from}_${to}`, from, to, cond }],
  };
}

function updatePolicy(graph: SkillGraph, id: string, policy: NodePolicy): SkillGraph {
  return {
    ...graph,
    nodes: graph.nodes.map((node) =>
      node.id === id ? { ...node, annotations: { ...node.annotations, policy } } : node,
    ),
  };
}

function updateToolBinding(graph: SkillGraph, id: string, raw: string): SkillGraph {
  const [mcpServer, toolName] = raw.split(".");
  return {
    ...graph,
    nodes: graph.nodes.map((node) =>
      node.id === id
        ? {
            ...node,
            annotations: {
              ...node.annotations,
              toolBinding: mcpServer && toolName ? { mcpServer, toolName } : undefined,
            },
          }
        : node,
    ),
  };
}

function defaultKind(kind: NodeKindTag): NodeKind {
  switch (kind) {
    case "action": return { type: "action", instruction: "Run action" };
    case "condition": return { type: "condition", expression: "ok" };
    case "branch": return { type: "branch", arms: [{ label: "default" }] };
    case "await-approval": return { type: "await-approval", approverRole: "operator" };
    case "fetch": return { type: "fetch", target: "mcp://tool", method: "GET" };
    case "transform": return { type: "transform", template: "{{input}}" };
    case "sub-skill-call": return { type: "sub-skill-call", target: { skillId: "skill", version: "latest" } };
    case "loop": return { type: "loop", iterVar: "item", body: [] };
    case "try-catch": return { type: "try-catch", tryBody: [], catchBody: [] };
  }
}

function structuralTargets(kind: NodeKind): string[] {
  if (kind.type === "loop") return kind.body;
  if (kind.type === "try-catch") return [...kind.tryBody, ...kind.catchBody];
  return [];
}

function deleteStructuralTarget(kind: NodeKind, id: string): NodeKind {
  if (kind.type === "loop") {
    return { ...kind, body: kind.body.filter((nodeId) => nodeId !== id) };
  }
  if (kind.type === "try-catch") {
    return {
      ...kind,
      tryBody: kind.tryBody.filter((nodeId) => nodeId !== id),
      catchBody: kind.catchBody.filter((nodeId) => nodeId !== id),
    };
  }
  return kind;
}

function formatTool(node: Node): string {
  const binding = node.annotations.toolBinding;
  return binding ? `${binding.mcpServer}.${binding.toolName}` : "";
}

function cloneGraph(graph: SkillGraph): SkillGraph {
  return JSON.parse(JSON.stringify(graph)) as SkillGraph;
}
