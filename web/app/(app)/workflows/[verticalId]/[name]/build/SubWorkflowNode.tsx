"use client";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { ExternalLink, Layers } from "lucide-react";

import type { SkyNodeData } from "./types";

const COMPOSE_ACCENT = "var(--brand-teal)";
const COMPOSE_ACCENT_TINT = "color-mix(in oklch, var(--brand-teal) 12%, transparent)";
const COMPOSE_ACCENT_BORDER = "color-mix(in oklch, var(--brand-teal) 28%, transparent)";
const COMPOSE_HEADER_GRADIENT =
  "linear-gradient(135deg, color-mix(in oklch, var(--brand-teal) 14%, transparent), color-mix(in oklch, var(--brand-pink) 10%, transparent))";
const COMPOSE_ICON_GRADIENT = "var(--brand-gradient)";
const COMPOSE_EXPAND_GRADIENT =
  "linear-gradient(135deg, color-mix(in oklch, var(--brand-teal) 10%, transparent), color-mix(in oklch, var(--brand-pink) 10%, transparent))";

// Stacked-card group node — visually distinct from the standard step node
// so the eye reads "this is a whole workflow, not a single action."
// Two offset shadow layers behind the main card create the "card-stack"
// illusion. Click "Open" to drill into the sub-workflow's own builder
// in a new tab; the parent builder stays in place.
export function SubWorkflowNode({ data, selected }: NodeProps) {
  const step = (data as SkyNodeData).step;
  const ref = step.subWorkflow;
  const borderColor = selected
    ? "var(--brand-primary)"
    : "color-mix(in oklch, var(--brand-teal) 35%, transparent)";
  const buildHref = ref
    ? `/workflows/${ref.verticalId}/${ref.name}/build`
    : null;

  return (
    <div
      data-testid="sub-workflow-node"
      style={{
        width: 300,
        position: "relative",
        fontFamily: "Inter, sans-serif",
        cursor: "grab",
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: COMPOSE_ACCENT, border: "none", width: 8, height: 8 }}
      />
      <div
        aria-hidden
        style={{
          position: "absolute",
          inset: 0,
          background: "white",
          border: "2px solid color-mix(in oklch, var(--brand-teal) 18%, transparent)",
          borderRadius: 14,
          transform: "translate(10px, 10px)",
          boxShadow: "0 4px 10px -4px color-mix(in oklch, var(--brand-teal) 12%, transparent)",
        }}
      />
      <div
        aria-hidden
        style={{
          position: "absolute",
          inset: 0,
          background: "white",
          border: "2px solid color-mix(in oklch, var(--brand-teal) 25%, transparent)",
          borderRadius: 14,
          transform: "translate(5px, 5px)",
          boxShadow: "0 4px 10px -4px color-mix(in oklch, var(--brand-teal) 18%, transparent)",
        }}
      />
      <div
        style={{
          position: "relative",
          background: "white",
          border: `2px solid ${borderColor}`,
          borderRadius: 14,
          boxShadow: selected
            ? "0 14px 32px -8px color-mix(in oklch, var(--brand-primary) 50%, transparent)"
            : "0 10px 24px -10px color-mix(in oklch, var(--brand-teal) 35%, transparent)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "12px 16px",
            background: COMPOSE_HEADER_GRADIENT,
            borderBottom: "1px solid color-mix(in oklch, var(--brand-teal) 22%, transparent)",
            display: "flex",
            alignItems: "center",
            gap: 10,
          }}
        >
          <div
            style={{
              width: 30,
              height: 30,
              borderRadius: 8,
              background: COMPOSE_ICON_GRADIENT,
              color: "white",
              display: "grid",
              placeItems: "center",
              flexShrink: 0,
            }}
          >
            <Layers size={16} />
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 9.5,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                fontWeight: 700,
                color: COMPOSE_ACCENT,
              }}
            >
              Sub-workflow
            </div>
            <div style={{ fontSize: 14, fontWeight: 600, color: "var(--brand-ink)", marginTop: 2 }}>
              {ref ? ref.displayName : step.name}
            </div>
          </div>
        </div>
        <div style={{ padding: "12px 16px" }}>
          {ref ? (
            <>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  flexWrap: "wrap",
                  marginBottom: 8,
                }}
              >
                <span
                  data-testid="sub-workflow-vertical"
                  style={{
                    padding: "2px 8px",
                    borderRadius: 999,
                    background: COMPOSE_ACCENT_TINT,
                    border: `1px solid ${COMPOSE_ACCENT_BORDER}`,
                    fontFamily: "var(--font-mono)",
                    fontSize: 9.5,
                    letterSpacing: "0.08em",
                    textTransform: "uppercase",
                    fontWeight: 700,
                    color: COMPOSE_ACCENT,
                  }}
                >
                  {ref.verticalId}
                </span>
                <span
                  data-testid="sub-workflow-step-count"
                  style={{
                    padding: "2px 8px",
                    borderRadius: 999,
                    background: "color-mix(in oklch, var(--brand-primary) 10%, transparent)",
                    border: "1px solid color-mix(in oklch, var(--brand-primary) 25%, transparent)",
                    fontFamily: "var(--font-mono)",
                    fontSize: 9.5,
                    letterSpacing: "0.08em",
                    fontWeight: 700,
                    color: "var(--brand-primary)",
                  }}
                >
                  {ref.stepCount} step{ref.stepCount === 1 ? "" : "s"}
                </span>
              </div>
              <div style={{ fontSize: 12.5, color: "var(--muted-foreground)", lineHeight: 1.45 }}>
                {step.description}
              </div>
              {buildHref ? (
                <a
                  data-testid="sub-workflow-expand"
                  href={buildHref}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={(e) => e.stopPropagation()}
                  style={{
                    marginTop: 10,
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    padding: "6px 10px",
                    borderRadius: 8,
                    background: COMPOSE_EXPAND_GRADIENT,
                    border: `1px solid ${COMPOSE_ACCENT_BORDER}`,
                    fontSize: 11.5,
                    fontWeight: 600,
                    color: COMPOSE_ACCENT,
                    textDecoration: "none",
                  }}
                >
                  Expand <ExternalLink size={11} />
                </a>
              ) : null}
            </>
          ) : (
            <div
              data-testid="sub-workflow-empty"
              style={{
                fontSize: 12.5,
                color: "#a16207",
                lineHeight: 1.45,
                padding: "8px 10px",
                borderRadius: 6,
                background: "rgba(245,158,11,0.10)",
                border: "1px dashed rgba(245,158,11,0.4)",
              }}
            >
              Pick a sub-workflow from the side panel.
            </div>
          )}
        </div>
      </div>
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: COMPOSE_ACCENT, border: "none", width: 8, height: 8 }}
      />
    </div>
  );
}
