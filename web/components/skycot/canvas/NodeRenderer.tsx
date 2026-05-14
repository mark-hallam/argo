"use client";

import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import {
  Activity,
  CheckCircle2,
  Cog,
  Download,
  GitBranch,
  HelpCircle,
  Repeat,
  Shield,
  Shuffle,
  Wand2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { Node as SkillNode, NodeKind } from "@/lib/types/skill-ir";

/**
 * Per-kind visual for the L4 read-only canvas. Each of the 9 IR
 * `NodeKind` variants gets its own icon + color via `KIND_VISUALS`.
 * The component is wrapped in `memo` because React Flow re-renders
 * nodes frequently during pan/zoom and the kind-to-visual mapping is
 * pure.
 *
 * The exported `KIND_VISUALS` map is what L4 promises downstream
 * consumers (L7 editor, L8 node primitives) — they reuse the icon +
 * color so the visual language stays coherent across modes.
 */

export type NodeKindTag = NodeKind["type"];

export interface NodeKindVisual {
  /** Short human label for the side panel + accessibility names. */
  readonly label: string;
  /** Lucide icon component. */
  readonly Icon: typeof Activity;
  /** Tailwind text-color class for the icon. */
  readonly iconClass: string;
  /** Tailwind background class for the node body. */
  readonly bodyClass: string;
  /** Tailwind border class for the node body. */
  readonly borderClass: string;
}

export const KIND_VISUALS: Record<NodeKindTag, NodeKindVisual> = {
  action: {
    label: "Action",
    Icon: Cog,
    iconClass: "text-sky-600 dark:text-sky-300",
    bodyClass: "bg-sky-50 dark:bg-sky-900/30",
    borderClass: "border-sky-300 dark:border-sky-700",
  },
  condition: {
    label: "Condition",
    Icon: HelpCircle,
    iconClass: "text-amber-600 dark:text-amber-300",
    bodyClass: "bg-amber-50 dark:bg-amber-900/30",
    borderClass: "border-amber-300 dark:border-amber-700",
  },
  branch: {
    label: "Branch",
    Icon: GitBranch,
    iconClass: "text-purple-600 dark:text-purple-300",
    bodyClass: "bg-purple-50 dark:bg-purple-900/30",
    borderClass: "border-purple-300 dark:border-purple-700",
  },
  "await-approval": {
    label: "Await approval",
    Icon: CheckCircle2,
    iconClass: "text-emerald-600 dark:text-emerald-300",
    bodyClass: "bg-emerald-50 dark:bg-emerald-900/30",
    borderClass: "border-emerald-300 dark:border-emerald-700",
  },
  fetch: {
    label: "Fetch",
    Icon: Download,
    iconClass: "text-blue-600 dark:text-blue-300",
    bodyClass: "bg-blue-50 dark:bg-blue-900/30",
    borderClass: "border-blue-300 dark:border-blue-700",
  },
  transform: {
    label: "Transform",
    Icon: Wand2,
    iconClass: "text-pink-600 dark:text-pink-300",
    bodyClass: "bg-pink-50 dark:bg-pink-900/30",
    borderClass: "border-pink-300 dark:border-pink-700",
  },
  "sub-skill-call": {
    label: "Sub-skill call",
    Icon: Shuffle,
    iconClass: "text-indigo-600 dark:text-indigo-300",
    bodyClass: "bg-indigo-50 dark:bg-indigo-900/30",
    borderClass: "border-indigo-300 dark:border-indigo-700",
  },
  loop: {
    label: "Loop",
    Icon: Repeat,
    iconClass: "text-orange-600 dark:text-orange-300",
    bodyClass: "bg-orange-50 dark:bg-orange-900/30",
    borderClass: "border-orange-300 dark:border-orange-700",
  },
  "try-catch": {
    label: "Try / catch",
    Icon: Shield,
    iconClass: "text-rose-600 dark:text-rose-300",
    bodyClass: "bg-rose-50 dark:bg-rose-900/30",
    borderClass: "border-rose-300 dark:border-rose-700",
  },
};

export const NODE_KIND_TAGS: readonly NodeKindTag[] = Object.keys(
  KIND_VISUALS,
) as NodeKindTag[];

/**
 * `data` payload carried by every React Flow node — wraps the canonical
 * IR `Node` plus the selection flag so the canvas can highlight the
 * active node without forcing the renderer to re-derive it.
 */
export interface SkillNodeData extends Record<string, unknown> {
  irNode: SkillNode;
  isSelected: boolean;
}

function kindSubtitle(kind: NodeKind): string | undefined {
  switch (kind.type) {
    case "action":
      return kind.instruction;
    case "condition":
      return kind.expression;
    case "fetch":
      return `${kind.method ?? "GET"} ${kind.target}`;
    case "transform":
      return kind.template;
    case "branch":
      return `${kind.arms.length} arm${kind.arms.length === 1 ? "" : "s"}`;
    case "await-approval":
      return `→ ${kind.approverRole}`;
    case "sub-skill-call":
      return `${kind.target.skillId} @ ${kind.target.version}`;
    case "loop":
      return `for ${kind.iterVar}`;
    case "try-catch":
      return `try (${kind.tryBody.length}) / catch (${kind.catchBody.length})`;
  }
}

function NodeRendererImpl(props: NodeProps) {
  const data = props.data as SkillNodeData;
  const { irNode, isSelected } = data;
  const visual = KIND_VISUALS[irNode.kind.type];
  const { Icon, iconClass, bodyClass, borderClass, label } = visual;
  const subtitle = kindSubtitle(irNode.kind);

  return (
    <div
      data-testid={`canvas-node-${irNode.id}`}
      data-kind={irNode.kind.type}
      data-selected={isSelected || undefined}
      aria-label={`${label}: ${irNode.label}`}
      className={cn(
        "min-w-[180px] max-w-[260px] rounded-[var(--radius-md)] border px-3 py-2 shadow-sm",
        "text-left text-[var(--card-foreground)]",
        bodyClass,
        borderClass,
        isSelected && "ring-2 ring-[var(--ring)] ring-offset-2 ring-offset-[var(--background)]",
      )}
    >
      <Handle type="target" position={Position.Top} className="!bg-[var(--border)]" />
      <div className="flex items-start gap-2">
        <Icon aria-hidden="true" className={cn("size-4 mt-0.5 shrink-0", iconClass)} />
        <div className="min-w-0">
          <div className="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
            {label}
          </div>
          <div className="text-sm font-medium leading-snug truncate">{irNode.label}</div>
          {subtitle ? (
            <div className="mt-0.5 text-xs text-[var(--muted-foreground)] line-clamp-2">
              {subtitle}
            </div>
          ) : null}
        </div>
      </div>
      <Handle type="source" position={Position.Bottom} className="!bg-[var(--border)]" />
    </div>
  );
}

export const NodeRenderer = memo(NodeRendererImpl);
NodeRenderer.displayName = "SkillCanvasNodeRenderer";
