"use client";
import { X } from "lucide-react";
import { useEffect } from "react";

import { PALETTE_GROUPS, STEP_KIND_META, type StepKind } from "./types";

interface AddStepPaletteProps {
  onSelect: (kind: StepKind) => void;
  onClose: () => void;
}

export function AddStepPalette({ onSelect, onClose }: AddStepPaletteProps) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);
  return (
    <div
      role="dialog"
      aria-label="Add step palette"
      style={{
        position: "absolute",
        right: 24,
        bottom: 80,
        zIndex: 11,
        width: 280,
        background: "white",
        border: "1px solid #ece9f5",
        borderRadius: 12,
        boxShadow: "0 18px 40px -10px rgba(30, 27, 75, 0.25)",
        padding: 12,
        fontFamily: "Inter, sans-serif",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
        <span
          style={{
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 10,
            letterSpacing: "0.14em",
            textTransform: "uppercase",
            fontWeight: 700,
            color: "#52525b",
          }}
        >
          Add step
        </span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close palette"
          style={{ background: "transparent", border: "none", cursor: "pointer", color: "#52525b", padding: 2, borderRadius: 4 }}
        >
          <X size={14} />
        </button>
      </div>
      {PALETTE_GROUPS.map((group) => (
        <div key={group.label} style={{ marginBottom: 10 }}>
          <div
            style={{
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 9.5,
              letterSpacing: "0.16em",
              textTransform: "uppercase",
              fontWeight: 700,
              color: "#a1a1aa",
              padding: "4px 4px",
            }}
          >
            {group.label}
          </div>
          {group.kinds.map((kind) => {
            const meta = STEP_KIND_META[kind];
            return (
              <button
                key={kind}
                type="button"
                data-testid={`palette-kind-${kind}`}
                onClick={() => {
                  onSelect(kind);
                  onClose();
                }}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  width: "100%",
                  padding: "8px 8px",
                  borderRadius: 8,
                  background: "transparent",
                  border: "none",
                  cursor: "pointer",
                  textAlign: "left",
                  color: "#1e1b4b",
                  fontSize: 13.5,
                  fontWeight: 500,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = `${meta.color}14`;
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                }}
              >
                <span
                  style={{
                    width: 26,
                    height: 26,
                    borderRadius: 7,
                    background: meta.color,
                    color: "white",
                    display: "grid",
                    placeItems: "center",
                    flexShrink: 0,
                  }}
                >
                  {meta.icon}
                </span>
                <span>{meta.label}</span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}
