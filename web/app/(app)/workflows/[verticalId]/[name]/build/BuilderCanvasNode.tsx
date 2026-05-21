"use client";
import { Handle, Position, type NodeProps } from "@xyflow/react";

import { formatWaitSummary } from "./layout";
import { STEP_KIND_META, type SkyNodeData } from "./types";

export function SkycotStepNode({ data, selected }: NodeProps) {
  const step = (data as SkyNodeData).step;
  const meta = STEP_KIND_META[step.kind];
  const isLoop = step.kind === "loop";
  const isWait = step.kind === "wait";
  const isDecision = step.kind === "decision" && !!step.branch;
  return (
    <div
      style={{
        width: 280,
        background: "white",
        border: `2px solid ${selected ? "#8b5cf6" : "rgba(0,0,0,0.08)"}`,
        borderRadius: 14,
        boxShadow: selected
          ? "0 12px 30px -8px rgba(139, 92, 246, 0.45)"
          : "0 6px 16px -8px rgba(0, 0, 0, 0.15)",
        overflow: "visible",
        fontFamily: "Inter, sans-serif",
        cursor: "grab",
        position: "relative",
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: "#8b5cf6", border: "none", width: 8, height: 8 }} />
      <div
        style={{
          padding: "12px 16px",
          background: `linear-gradient(135deg, ${meta.color}1a, ${meta.color}0a)`,
          borderBottom: `1px solid ${meta.color}33`,
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        <div
          style={{
            width: 28,
            height: 28,
            borderRadius: 7,
            background: meta.color,
            color: "white",
            display: "grid",
            placeItems: "center",
          }}
        >
          {meta.icon}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 9.5,
              letterSpacing: "0.12em",
              textTransform: "uppercase",
              fontWeight: 700,
              color: meta.color,
            }}
          >
            {meta.label}
          </div>
          <div style={{ fontSize: 14, fontWeight: 600, color: "#1e1b4b", marginTop: 2 }}>
            {step.name}
          </div>
        </div>
      </div>
      <div style={{ padding: "12px 16px" }}>
        <div style={{ fontSize: 12.5, color: "#3f3f46", lineHeight: 1.45 }}>
          {isWait ? formatWaitSummary(step.waitConfig) : step.description}
        </div>
        {isLoop && step.loopConfig ? (
          <div
            style={{
              marginTop: 8,
              padding: "8px 10px",
              borderRadius: 6,
              background: "rgba(99,102,241,0.08)",
              border: "1px dashed rgba(99,102,241,0.4)",
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 10.5,
              color: "#3730a3",
              lineHeight: 1.5,
            }}
          >
            <div>for each <strong>{step.loopConfig.item_var_name}</strong> in {step.loopConfig.items_path}</div>
            <div style={{ opacity: 0.7, fontSize: 9.5, marginTop: 2 }}>max {step.loopConfig.max_iterations} iterations</div>
          </div>
        ) : null}
        {step.approvalGate ? (
          <div
            style={{
              marginTop: 8,
              padding: "6px 9px",
              borderRadius: 6,
              background: "rgba(245,158,11,0.1)",
              border: "1px solid rgba(245,158,11,0.3)",
              fontSize: 10.5,
              color: "#92400e",
            }}
          >
            <strong style={{ fontFamily: "JetBrains Mono, monospace", fontSize: 9.5, letterSpacing: "0.12em" }}>
              GATE · {step.approvalGate.channel}
            </strong>
          </div>
        ) : null}
        {step.inputs || step.outputs ? (
          <div
            style={{
              marginTop: 8,
              display: "flex",
              gap: 8,
              flexWrap: "wrap",
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 9,
              color: "#52525b",
              letterSpacing: "0.04em",
            }}
          >
            {step.inputs ? <span>IN: {step.inputs.join(", ").slice(0, 24)}…</span> : null}
            {step.outputs ? <span>OUT: {step.outputs.join(", ").slice(0, 24)}…</span> : null}
          </div>
        ) : null}
      </div>
      {isDecision ? (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            padding: "8px 14px 10px",
            borderTop: "1px dashed rgba(0,0,0,0.08)",
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 9.5,
            letterSpacing: "0.1em",
            textTransform: "uppercase",
            fontWeight: 700,
          }}
        >
          <span style={{ color: "#8b5cf6" }}>↓ {step.branch!.trueLabel}</span>
          <span style={{ color: "#f59e0b" }}>{step.branch!.falseLabel} →</span>
        </div>
      ) : null}
      {isDecision ? (
        <Handle
          type="source"
          position={Position.Bottom}
          id="true"
          style={{ background: "#8b5cf6", border: "none", width: 10, height: 10, left: "30%" }}
        />
      ) : (
        <Handle type="source" position={Position.Bottom} style={{ background: "#8b5cf6", border: "none", width: 8, height: 8 }} />
      )}
      {isDecision ? (
        <Handle
          type="source"
          position={Position.Right}
          id="false"
          style={{ background: "#f59e0b", border: "none", width: 10, height: 10, top: "50%" }}
        />
      ) : null}
      {isLoop ? (
        <div
          style={{
            position: "absolute",
            bottom: -22,
            left: "50%",
            transform: "translateX(-50%)",
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 9,
            letterSpacing: "0.1em",
            textTransform: "uppercase",
            fontWeight: 700,
            color: "#6366f1",
            background: "white",
            padding: "2px 8px",
            borderRadius: 999,
            border: "1px dashed rgba(99,102,241,0.4)",
            pointerEvents: "none",
            whiteSpace: "nowrap",
          }}
        >
          for each item ↻
        </div>
      ) : null}
    </div>
  );
}
