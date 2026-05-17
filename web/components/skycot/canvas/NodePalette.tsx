"use client";

import { NODE_KIND_TAGS, KIND_VISUALS, type NodeKindTag } from "./NodeRenderer";

export interface NodePaletteProps {
  onAdd: (kind: NodeKindTag) => void;
}

export function NodePalette({ onAdd }: NodePaletteProps) {
  return (
    <div
      data-testid="node-palette"
      className="flex flex-wrap gap-2 border-b border-[var(--border)] bg-[var(--muted)]/20 p-3"
    >
      {NODE_KIND_TAGS.map((kind) => {
        const { Icon, label, iconClass } = KIND_VISUALS[kind];
        return (
          <button
            key={kind}
            type="button"
            draggable
            data-testid={`palette-${kind}`}
            onClick={() => onAdd(kind)}
            className="inline-flex items-center gap-1 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2.5 py-1 text-xs font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
          >
            <Icon className={`size-3.5 ${iconClass}`} aria-hidden="true" />
            {label}
          </button>
        );
      })}
    </div>
  );
}
