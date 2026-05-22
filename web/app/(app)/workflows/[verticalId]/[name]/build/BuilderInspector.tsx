"use client";
import { Pin, PinOff, Settings as SettingsIcon, X } from "lucide-react";

import { CodeConfigEditor } from "./step-config-code";
import { HttpRequestConfigEditor } from "./step-config-http";
import {
  STEP_KIND_META,
  type CodeConfig,
  type HttpRequestConfig,
  type WorkflowStep,
} from "./types";

interface Props {
  step: WorkflowStep;
  isTrigger: boolean;
  pinnedJson: string | null;
  onClose: () => void;
  onEdit: () => void;
  onOpenPin: () => void;
  onClearPin: () => void;
  onStepChange?: (next: WorkflowStep) => void;
}

export function BuilderInspector({
  step,
  isTrigger,
  pinnedJson,
  onClose,
  onEdit,
  onOpenPin,
  onClearPin,
  onStepChange,
}: Props) {
  const updateHttp = (next: HttpRequestConfig) =>
    onStepChange?.({ ...step, httpRequest: next });
  const updateCode = (next: CodeConfig) =>
    onStepChange?.({ ...step, code: next });
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
          <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "var(--font-mono)" }}>
            {step.inputs.join(", ")}
          </code>
        </div>
      ) : null}
      {step.outputs ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">OUTPUTS</div>
          <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "var(--font-mono)" }}>
            {step.outputs.join(", ")}
          </code>
        </div>
      ) : null}
      {step.kind === "http_request" && onStepChange ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">HTTP REQUEST</div>
          <HttpRequestConfigEditor config={step.httpRequest} onChange={updateHttp} />
        </div>
      ) : null}
      {step.kind === "code" && onStepChange ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">CODE</div>
          <CodeConfigEditor config={step.code} onChange={updateCode} />
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
      {!isTrigger ? (
        <div className="builder-panel-section">
          <div className="builder-panel-section-label">PIN INPUT DATA</div>
          {pinnedJson ? (
            <>
              <pre
                data-testid="builder-pinned-preview"
                style={{
                  margin: "0 0 8px",
                  padding: "8px 10px",
                  borderRadius: 6,
                  background: "rgba(139,92,246,0.06)",
                  border: "1px solid rgba(139,92,246,0.25)",
                  color: "#1e1b4b",
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  lineHeight: 1.4,
                  maxHeight: 120,
                  overflow: "auto",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {pinnedJson.length > 280 ? `${pinnedJson.slice(0, 280)}…` : pinnedJson}
              </pre>
              <div style={{ display: "flex", gap: 6 }}>
                <button
                  type="button"
                  data-testid="builder-pin-edit"
                  onClick={onOpenPin}
                  style={{
                    flex: 1,
                    padding: "7px 10px",
                    borderRadius: 7,
                    background: "white",
                    color: "#5b21b6",
                    border: "1px solid rgba(139,92,246,0.4)",
                    cursor: "pointer",
                    fontSize: 12,
                    fontWeight: 600,
                    display: "inline-flex",
                    gap: 6,
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <Pin size={12} /> Edit pin
                </button>
                <button
                  type="button"
                  data-testid="builder-pin-clear"
                  onClick={onClearPin}
                  style={{
                    padding: "7px 10px",
                    borderRadius: 7,
                    background: "white",
                    color: "#9f1239",
                    border: "1px solid rgba(244,63,94,0.35)",
                    cursor: "pointer",
                    fontSize: 12,
                    fontWeight: 600,
                    display: "inline-flex",
                    gap: 6,
                    alignItems: "center",
                  }}
                >
                  <PinOff size={12} /> Unpin
                </button>
              </div>
            </>
          ) : (
            <button
              type="button"
              data-testid="builder-pin-open"
              onClick={onOpenPin}
              style={{
                width: "100%",
                padding: "8px 12px",
                borderRadius: 7,
                background: "white",
                color: "#5b21b6",
                border: "1px dashed rgba(139,92,246,0.45)",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: 600,
                display: "inline-flex",
                gap: 6,
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Pin size={12} /> Pin data
            </button>
          )}
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
