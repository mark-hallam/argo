"use client";
import { Settings as SettingsIcon, X } from "lucide-react";

import { STEP_KIND_META, type WorkflowStep } from "./types";

interface Props {
  step: WorkflowStep;
  isTrigger: boolean;
  onClose: () => void;
  onEdit: () => void;
}

export function BuilderInspector({ step, isTrigger, onClose, onEdit }: Props) {
  return (
    <aside className="builder-panel">
      <div className="builder-panel-head">
        <span className="builder-panel-eyebrow">{STEP_KIND_META[step.kind].label}</span>
        <button type="button" className="builder-panel-close" onClick={onClose}>
          <X size={16} />
        </button>
      </div>
      <h2 className="builder-panel-title">{step.name}</h2>
      <p style={{ fontSize: 13.5, color: "#3f3f46", lineHeight: 1.55, margin: "0 0 18px" }}>
        {step.description}
      </p>
      {step.inputs ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">INPUTS</div>
          <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "JetBrains Mono, monospace" }}>
            {step.inputs.join(", ")}
          </code>
        </div>
      ) : null}
      {step.outputs ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">OUTPUTS</div>
          <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "JetBrains Mono, monospace" }}>
            {step.outputs.join(", ")}
          </code>
        </div>
      ) : null}
      {step.approvalGate ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">APPROVAL GATE</div>
          <div
            style={{
              padding: "10px 12px",
              borderRadius: 8,
              background: "rgba(245,158,11,0.1)",
              border: "1px solid rgba(245,158,11,0.3)",
              color: "#92400e",
              fontSize: 12.5,
            }}
          >
            <div style={{ fontWeight: 700, marginBottom: 4 }}>Channel: {step.approvalGate.channel}</div>
            {step.approvalGate.reason}
          </div>
        </div>
      ) : null}
      <button
        type="button"
        data-testid="builder-edit-step"
        onClick={onEdit}
        style={{
          marginTop: 12,
          width: "100%",
          padding: "10px 16px",
          borderRadius: 9,
          background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
          color: "white",
          border: "none",
          fontWeight: 700,
          fontSize: 13,
          cursor: "pointer",
          display: "inline-flex",
          gap: 6,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <SettingsIcon size={13} />
        {isTrigger ? "Configure trigger" : "Edit step"}
      </button>
    </aside>
  );
}
