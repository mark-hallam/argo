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
  color: "var(--muted-foreground)",
  marginBottom: 4,
};

const INPUT_STYLE: React.CSSProperties = {
  width: "100%",
  padding: "7px 9px",
  borderRadius: 7,
  border: "1px solid var(--border)",
  background: "var(--card)",
  color: "var(--foreground)",
  fontSize: 12.5,
  fontFamily: "var(--font-sans)",
};

const LANGUAGE_OPTIONS: { value: CodeLanguage; label: string }[] = [
  { value: "javascript", label: "JavaScript (Node 20)" },
  { value: "python", label: "Python 3.12" },
];

const EDITOR_FRAME_STYLE: React.CSSProperties = {
  borderRadius: 7,
  border: "1px solid var(--border)",
  background: "var(--card)",
  overflow: "hidden",
};

const EDITOR_FALLBACK_STYLE: React.CSSProperties = {
  height: 320,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  color: "var(--muted-foreground)",
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
          background: "color-mix(in oklab, var(--warning) 12%, var(--card))",
          border: "1px solid color-mix(in oklab, var(--warning) 36%, var(--border))",
          color: "var(--warning-foreground)",
          fontSize: 12,
          lineHeight: 1.45,
          display: "flex",
          gap: 8,
          marginBottom: 14,
        }}
      >
        <AlertTriangle size={14} aria-hidden="true" style={{ flexShrink: 0, marginTop: 2 }} />
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
              color: "var(--muted-foreground)",
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
              background: "color-mix(in oklab, var(--destructive) 10%, var(--card))",
              border: "1px solid color-mix(in oklab, var(--destructive) 32%, var(--border))",
              color: "var(--destructive)",
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
