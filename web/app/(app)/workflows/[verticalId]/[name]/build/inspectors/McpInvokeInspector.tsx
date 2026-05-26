"use client";
import { useCallback, useMemo } from "react";

import { findConnectorById } from "@/app/(app)/settings/connectors/connector-catalog";

import { defaultMcpInvokeConfig } from "../builder-handlers";
import type { McpInvokeConfig } from "../types";

interface Props {
  config: McpInvokeConfig | undefined;
  onChange: (next: McpInvokeConfig) => void;
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

const SERVER_BADGE_STYLE: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 6,
  padding: "4px 10px",
  borderRadius: 999,
  background: "rgba(8,145,178,0.08)",
  border: "1px solid rgba(8,145,178,0.3)",
  color: "#0e7490",
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 700,
};

const ERROR_STYLE: React.CSSProperties = {
  marginTop: 6,
  padding: "6px 9px",
  borderRadius: 6,
  background: "rgba(244,63,94,0.08)",
  border: "1px solid rgba(244,63,94,0.3)",
  color: "#9f1239",
  fontSize: 11.5,
};

function validateJson(raw: string): string | null {
  if (!raw.trim()) return null;
  try {
    JSON.parse(raw);
    return null;
  } catch (e) {
    return (e as Error).message;
  }
}

/**
 * Inspector form for `mcp_invoke` steps. `server` is locked to the
 * connector slug chosen from the palette — the connector identity is
 * the load-bearing piece of the step, not a free-text field. `tool`
 * picks an operation on that server (free text since per-server tool
 * catalogs land in a follow-up). `args` is a JSON editor that surfaces
 * parse errors inline so the operator catches typos before Save.
 */
export function McpInvokeInspector({ config, onChange }: Props) {
  const cfg = config ?? defaultMcpInvokeConfig("");
  const connector = useMemo(
    () => (cfg.server ? findConnectorById(cfg.server) : undefined),
    [cfg.server],
  );
  const update = useCallback(
    (patch: Partial<McpInvokeConfig>) => onChange({ ...cfg, ...patch }),
    [cfg, onChange],
  );
  const argsError = validateJson(cfg.args);

  return (
    <div data-testid="builder-mcp-config">
      <div style={{ marginBottom: 12 }}>
        <span style={LABEL_STYLE}>Server</span>
        <span data-testid="builder-mcp-server" style={SERVER_BADGE_STYLE}>
          {connector?.name ?? (cfg.server || "—")}
        </span>
      </div>
      <div style={{ marginBottom: 12 }}>
        <label htmlFor="mcp-tool" style={LABEL_STYLE}>Tool</label>
        <input
          id="mcp-tool"
          data-testid="builder-mcp-tool"
          value={cfg.tool}
          onChange={(e) => update({ tool: e.target.value })}
          placeholder="e.g. create_record"
          style={INPUT_STYLE}
        />
      </div>
      <div style={{ marginBottom: 4 }}>
        <label htmlFor="mcp-args" style={LABEL_STYLE}>Args (JSON)</label>
        <textarea
          id="mcp-args"
          data-testid="builder-mcp-args"
          value={cfg.args}
          onChange={(e) => update({ args: e.target.value })}
          rows={6}
          spellCheck={false}
          style={{ ...INPUT_STYLE, fontFamily: "var(--font-mono)", fontSize: 11.5, resize: "vertical" }}
        />
        {argsError ? (
          <div role="alert" data-testid="builder-mcp-args-error" style={ERROR_STYLE}>
            {argsError}
          </div>
        ) : null}
      </div>
    </div>
  );
}
