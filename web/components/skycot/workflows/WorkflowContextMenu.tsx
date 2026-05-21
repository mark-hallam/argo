"use client";

import { type LucideIcon } from "lucide-react";
import { useEffect, useRef } from "react";

/**
 * Right-click menu shown on workflow-builder nodes and edges.
 *
 * Items are pure-data — the caller wires `onClick` against the
 * useWorkflowBuilderState surface (addNode / removeNode / removeEdge).
 * The menu is `position: fixed` (not absolute) because the React Flow
 * canvas pans and zooms — absolute positioning would scroll away.
 *
 * Closes on click-outside or Escape via window listeners; the parent
 * also closes it from `onPaneClick`.
 */

export interface ContextMenuItem {
  label: string;
  Icon: LucideIcon;
  onClick: () => void;
  destructive?: boolean;
  disabled?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export function WorkflowContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      data-testid="workflow-context-menu"
      style={{
        position: "fixed",
        left: x,
        top: y,
        zIndex: 9999,
        minWidth: 188,
        background: "white",
        border: "1px solid #ece9f5",
        borderRadius: 10,
        boxShadow: "0 18px 40px -10px rgba(30, 27, 75, 0.28)",
        padding: 6,
        fontFamily: "Inter, sans-serif",
      }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          data-destructive={item.destructive ? "true" : "false"}
          onClick={() => {
            if (item.disabled) return;
            item.onClick();
            onClose();
          }}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            width: "100%",
            padding: "8px 10px",
            borderRadius: 7,
            background: "transparent",
            border: "none",
            cursor: item.disabled ? "not-allowed" : "pointer",
            textAlign: "left",
            color: item.destructive ? "#dc2626" : "#1e1b4b",
            opacity: item.disabled ? 0.45 : 1,
            fontSize: 13.5,
            fontWeight: 500,
          }}
          onMouseEnter={(e) => {
            if (item.disabled) return;
            e.currentTarget.style.background = item.destructive ? "#fee2e2" : "#f5f3ff";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
          }}
        >
          <item.Icon size={14} aria-hidden /> {item.label}
        </button>
      ))}
    </div>
  );
}
