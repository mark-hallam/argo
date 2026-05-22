"use client";
import { AlertTriangle } from "lucide-react";
import dynamic from "next/dynamic";
import { useCallback } from "react";

import { defaultCodeConfig } from "./builder-handlers";
import type { CodeConfig, CodeLanguage } from "./types";

interface Props {
  config: CodeConfig | undefined;
  onChange: (next: CodeConfig) => void;
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

const LANGUAGE_OPTIONS: { value: CodeLanguage; label: string }[] = [
  { value: "javascript", label: "JavaScript (Node 20)" },
  { value: "python", label: "Python 3.12" },
];

const EDITOR_FRAME_STYLE: React.CSSProperties = {
  borderRadius: 7,
  border: "1px solid rgba(0,0,0,0.12)",
  background: "white",
  overflow: "hidden",
};

const EDITOR_FALLBACK_STYLE: React.CSSProperties = {
  height: 320,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  color: "#a1a1aa",
};

// Monaco runtime weighs ~3MB; ship it from a code-split chunk that only
// loads when a code-step config is opened. `ssr: false` avoids a
// Next.js SSR boundary that the editor's window-globals would trip.
const MonacoCodeEditor = dynamic(
  () => import("@/components/builder/MonacoCodeEditor"),
  {
    ssr: false,
    loading: () => (
      <div data-testid="builder-code-monaco-loading" style={EDITOR_FALLBACK_STYLE}>
        Loading editor…
      </div>
    ),
  },
);

/**
 * Inline editor for the Code node config. The code body is a
 * code-split Monaco editor (JavaScript / Python syntax highlighting,
 * line numbers, 2-space tabs). The sandbox warning banner is the
 * brief's "no surprises" promise: code execution is isolated,
 * time-budgeted, and network-allowlisted.
 */
export function CodeConfigEditor({ config, onChange }: Props) {
  const cfg = config ?? defaultCodeConfig();
  const update = useCallback(
    (patch: Partial<CodeConfig>) => onChange({ ...cfg, ...patch }),
    [cfg, onChange],
  );
  const isEmpty = cfg.code.trim().length === 0;

  return (
    <div data-testid="builder-code-config">
      <div
        data-testid="builder-code-sandbox-warning"
        style={{
          padding: "10px 12px",
          borderRadius: 8,
          background: "rgba(245,158,11,0.1)",
          border: "1px solid rgba(245,158,11,0.35)",
          color: "#92400e",
          fontSize: 12,
          lineHeight: 1.45,
          display: "flex",
          gap: 8,
          marginBottom: 14,
        }}
      >
        <AlertTriangle size={14} style={{ flexShrink: 0, marginTop: 2 }} />
        <span>
          Code runs in an isolated sandbox. No filesystem access. 5s CPU budget.
          Network requires explicit allowlist.
        </span>
      </div>
      <div style={{ marginBottom: 12 }}>
        <label htmlFor="code-language" style={LABEL_STYLE}>Language</label>
        <select
          id="code-language"
          data-testid="builder-code-language"
          value={cfg.language}
          onChange={(e) => update({ language: e.target.value as CodeLanguage })}
          style={INPUT_STYLE}
        >
          {LANGUAGE_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>
      </div>
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 4 }}>
          <span style={LABEL_STYLE}>Source</span>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: "#a1a1aa",
            }}
          >
            Available: input, env, secrets
          </span>
        </div>
        <div data-testid="builder-code-body" style={EDITOR_FRAME_STYLE}>
          <MonacoCodeEditor
            language={cfg.language}
            value={cfg.code}
            onChange={(next) => update({ code: next })}
          />
        </div>
        {isEmpty ? (
          <div
            role="alert"
            data-testid="builder-code-empty-error"
            style={{
              marginTop: 6,
              padding: "6px 9px",
              borderRadius: 6,
              background: "rgba(244,63,94,0.08)",
              border: "1px solid rgba(244,63,94,0.3)",
              color: "#9f1239",
              fontSize: 11.5,
            }}
          >
            Code can&apos;t be empty — write a snippet that returns a value.
          </div>
        ) : null}
      </div>
    </div>
  );
}
