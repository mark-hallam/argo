"use client";
import { Layers, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { SEED_WORKFLOWS } from "./seed-workflows";
import type { SubWorkflowRef, WorkflowSpec } from "./types";

interface SubWorkflowPickerProps {
  // Workflow that the picker is rendered inside — excluded from results
  // so a workflow can't recursively pick itself.
  currentVerticalId: string;
  currentName: string;
  onPick: (ref: SubWorkflowRef) => void;
  onClose: () => void;
}

interface PickerCandidate {
  key: string;
  ref: SubWorkflowRef;
  spec: WorkflowSpec;
}

function buildCandidates(currentVerticalId: string, currentName: string): PickerCandidate[] {
  return Object.entries(SEED_WORKFLOWS)
    .filter(([, spec]) => !(spec.vertical === currentVerticalId && spec.name === currentName))
    .map(([key, spec]) => ({
      key,
      spec,
      ref: {
        verticalId: spec.vertical,
        name: spec.name,
        displayName: spec.name.replace(/_/g, " "),
        stepCount: spec.steps.length,
      },
    }));
}

export function SubWorkflowPicker({
  currentVerticalId,
  currentName,
  onPick,
  onClose,
}: SubWorkflowPickerProps) {
  const [query, setQuery] = useState("");

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const candidates = useMemo(
    () => buildCandidates(currentVerticalId, currentName),
    [currentVerticalId, currentName],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return candidates;
    return candidates.filter((c) => {
      return (
        c.ref.displayName.toLowerCase().includes(q)
        || c.ref.name.toLowerCase().includes(q)
        || c.ref.verticalId.toLowerCase().includes(q)
        || c.spec.description.toLowerCase().includes(q)
      );
    });
  }, [candidates, query]);

  return (
    <div
      role="dialog"
      aria-label="Pick a sub-workflow"
      data-testid="sub-workflow-picker"
      style={{
        position: "absolute",
        inset: 0,
        background: "rgba(15, 23, 42, 0.35)",
        display: "grid",
        placeItems: "center",
        zIndex: 20,
        fontFamily: "Inter, sans-serif",
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: "min(540px, 90%)",
          maxHeight: "70vh",
          background: "white",
          border: "1px solid #ece9f5",
          borderRadius: 14,
          boxShadow: "0 24px 60px -16px rgba(30, 27, 75, 0.4)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "14px 18px",
            borderBottom: "1px solid #ece9f5",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div
              style={{
                width: 28,
                height: 28,
                borderRadius: 7,
                background: "linear-gradient(135deg, #0891b2, #8b5cf6)",
                color: "white",
                display: "grid",
                placeItems: "center",
              }}
            >
              <Layers size={14} />
            </div>
            <div>
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 9.5,
                  letterSpacing: "0.14em",
                  textTransform: "uppercase",
                  fontWeight: 700,
                  color: "#52525b",
                }}
              >
                Pick a workflow
              </div>
              <div style={{ fontSize: 13, color: "#1e1b4b", marginTop: 2 }}>
                It will be called as a single step in this workflow.
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close picker"
            style={{
              background: "transparent",
              border: "none",
              cursor: "pointer",
              color: "#52525b",
              padding: 4,
              borderRadius: 4,
            }}
          >
            <X size={16} />
          </button>
        </div>
        <div style={{ padding: "12px 18px", borderBottom: "1px solid #ece9f5" }}>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by name, vertical, or description…"
            autoFocus
            data-testid="sub-workflow-picker-search"
            style={{
              width: "100%",
              padding: "8px 12px",
              borderRadius: 8,
              border: "1px solid #d4d4d8",
              fontSize: 13,
              fontFamily: "Inter, sans-serif",
              outline: "none",
            }}
          />
        </div>
        <div
          style={{
            overflowY: "auto",
            padding: 8,
            display: "flex",
            flexDirection: "column",
            gap: 6,
          }}
        >
          {filtered.length === 0 ? (
            <div
              data-testid="sub-workflow-picker-empty"
              style={{
                padding: 24,
                textAlign: "center",
                color: "#52525b",
                fontSize: 13,
              }}
            >
              No workflows match. Try a different search.
            </div>
          ) : (
            filtered.map((c) => (
              <button
                key={c.key}
                type="button"
                data-testid={`sub-workflow-picker-option-${c.ref.verticalId}-${c.ref.name}`}
                onClick={() => onPick(c.ref)}
                style={{
                  display: "flex",
                  alignItems: "flex-start",
                  gap: 10,
                  width: "100%",
                  padding: "10px 12px",
                  borderRadius: 8,
                  background: "transparent",
                  border: "1px solid transparent",
                  cursor: "pointer",
                  textAlign: "left",
                  color: "#1e1b4b",
                  fontFamily: "Inter, sans-serif",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(8,145,178,0.08)";
                  e.currentTarget.style.borderColor = "rgba(8,145,178,0.25)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.borderColor = "transparent";
                }}
              >
                <span
                  style={{
                    padding: "2px 8px",
                    borderRadius: 999,
                    background: "rgba(8,145,178,0.10)",
                    border: "1px solid rgba(8,145,178,0.25)",
                    fontFamily: "var(--font-mono)",
                    fontSize: 9.5,
                    letterSpacing: "0.08em",
                    textTransform: "uppercase",
                    fontWeight: 700,
                    color: "#0e7490",
                    flexShrink: 0,
                    marginTop: 2,
                  }}
                >
                  {c.ref.verticalId}
                </span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 14, fontWeight: 600 }}>{c.ref.displayName}</div>
                  <div style={{ fontSize: 12, color: "#52525b", marginTop: 2, lineHeight: 1.4 }}>
                    {c.spec.description}
                  </div>
                  <div
                    style={{
                      marginTop: 4,
                      fontFamily: "var(--font-mono)",
                      fontSize: 9.5,
                      color: "#52525b",
                    }}
                  >
                    {c.ref.stepCount} step{c.ref.stepCount === 1 ? "" : "s"} · {c.spec.risk}
                  </div>
                </div>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
