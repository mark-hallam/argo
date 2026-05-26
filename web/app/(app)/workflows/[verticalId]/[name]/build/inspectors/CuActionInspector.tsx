"use client";
import { useCallback } from "react";

import { defaultCuActionConfig } from "../builder-handlers";
import type { CuActionConfig } from "../types";

interface Props {
  config: CuActionConfig | undefined;
  onChange: (next: CuActionConfig) => void;
}

const LABEL_STYLE: React.CSSProperties = {
  display: "block",
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  letterSpacing: "0.12em",
  textTransform: "uppercase",
  fontWeight: 700,
  color: "#52525b",
  marginBottom: 4,
};

const INPUT_STYLE: React.CSSProperties = {
  width: "100%",
  padding: "7px 9px",
  borderRadius: 7,
  border: "1px solid rgba(0,0,0,0.12)",
  background: "white",
  color: "#1e1b4b",
  fontSize: 12.5,
  fontFamily: "Inter, sans-serif",
};

const SUBSTRATE_BADGE_STYLE: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
  padding: "4px 10px",
  borderRadius: 999,
  background: "rgba(139,92,246,0.08)",
  border: "1px solid rgba(139,92,246,0.3)",
  color: "#5b21b6",
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 700,
};

/**
 * Inspector form for `cu_action` (computer-use) steps. v1.0 ships
 * macOS-only per the 2026-05-14 operator decision; substrate renders
 * as a non-editable badge so the screen is honest about what the
 * platform supports today. `app`/`action`/`target` map 1:1 to the
 * `cu.run_macro` argument tuple used in the demo seed workflow
 * (`crates/argo-demo-seed/src/workflow.rs`).
 */
export function CuActionInspector({ config, onChange }: Props) {
  const cfg = config ?? defaultCuActionConfig();
  const update = useCallback(
    (patch: Partial<CuActionConfig>) => onChange({ ...cfg, ...patch }),
    [cfg, onChange],
  );

  return (
    <div data-testid="builder-cu-config">
      <div style={{ marginBottom: 12 }}>
        <span style={LABEL_STYLE}>Substrate</span>
        <span data-testid="builder-cu-substrate" style={SUBSTRATE_BADGE_STYLE}>
          {cfg.substrate}
        </span>
      </div>
      <div style={{ marginBottom: 12 }}>
        <label htmlFor="cu-app" style={LABEL_STYLE}>App</label>
        <input
          id="cu-app"
          data-testid="builder-cu-app"
          value={cfg.app}
          onChange={(e) => update({ app: e.target.value })}
          placeholder="Notes.app"
          style={INPUT_STYLE}
        />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label htmlFor="cu-action" style={LABEL_STYLE}>Action</label>
        <input
          id="cu-action"
          data-testid="builder-cu-action"
          value={cfg.action}
          onChange={(e) => update({ action: e.target.value })}
          placeholder="run_macro"
          style={INPUT_STYLE}
        />
      </div>
      <div style={{ marginBottom: 4 }}>
        <label htmlFor="cu-target" style={LABEL_STYLE}>Target</label>
        <input
          id="cu-target"
          data-testid="builder-cu-target"
          value={cfg.target}
          onChange={(e) => update({ target: e.target.value })}
          placeholder="e.g. note titled 'Follow-up'"
          style={INPUT_STYLE}
        />
      </div>
    </div>
  );
}
