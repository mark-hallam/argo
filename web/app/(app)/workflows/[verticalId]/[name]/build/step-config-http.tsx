"use client";
import { Plus, Trash2, Zap } from "lucide-react";
import { useCallback, useMemo, useState } from "react";

import {
  looksLikeJson,
  prettyPrintJson,
  tryParseJson,
} from "@/lib/builder/json-helpers";

import { defaultHttpRequestConfig } from "./builder-handlers";
import {
  HTTP_METHODS,
  type HttpAuthMode,
  type HttpHeader,
  type HttpMethod,
  type HttpRequestConfig,
} from "./types";

interface Props {
  config: HttpRequestConfig | undefined;
  onChange: (next: HttpRequestConfig) => void;
}

type TryResult =
  | { status: "idle" }
  | { status: "running" }
  | { status: "ok"; httpStatus: number; bodyPreview: string }
  | { status: "missing" };

const AUTH_OPTIONS: { value: HttpAuthMode; label: string }[] = [
  { value: "none", label: "None" },
  { value: "bearer", label: "Bearer" },
  { value: "basic", label: "Basic" },
  { value: "custom_header", label: "Custom header" },
];

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

const MONO_STYLE: React.CSSProperties = {
  ...INPUT_STYLE,
  fontFamily: "var(--font-mono)",
  fontSize: 12,
};

const FIELD_STYLE: React.CSSProperties = {
  marginBottom: 12,
};

/**
 * Inline editor for HTTP request node config. Lives inside the side
 * inspector while the user has an `http_request` step selected. The
 * "Try it" button posts the config to the gateway `/api/v1/tools/http/try`
 * test endpoint; on 404/501/network error it falls back to a friendly
 * "coming v1.x" notice (the endpoint is queued in a separate brief).
 */
export function HttpRequestConfigEditor({ config, onChange }: Props) {
  const cfg = config ?? defaultHttpRequestConfig();
  const [tryResult, setTryResult] = useState<TryResult>({ status: "idle" });

  const update = useCallback(
    (patch: Partial<HttpRequestConfig>) => onChange({ ...cfg, ...patch }),
    [cfg, onChange],
  );

  const setHeader = useCallback(
    (index: number, patch: Partial<HttpHeader>) => {
      const next = cfg.headers.map((h, i) => (i === index ? { ...h, ...patch } : h));
      update({ headers: next });
    },
    [cfg.headers, update],
  );

  const addHeader = useCallback(() => {
    update({ headers: [...cfg.headers, { key: "", value: "" }] });
  }, [cfg.headers, update]);

  const removeHeader = useCallback(
    (index: number) => {
      update({ headers: cfg.headers.filter((_, i) => i !== index) });
    },
    [cfg.headers, update],
  );

  const bodyJsonError = useMemo<string | null>(() => {
    if (!looksLikeJson(cfg.body)) return null;
    const parsed = tryParseJson(cfg.body);
    if (parsed.ok) return null;
    return `JSON parse error at line ${parsed.line}: ${parsed.message}`;
  }, [cfg.body]);

  const tryIt = useCallback(async () => {
    setTryResult({ status: "running" });
    try {
      const res = await fetch("/api/v1/tools/http/try", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          method: cfg.method,
          url: cfg.url,
          headers: cfg.headers,
          body: cfg.body,
          auth: cfg.auth,
          authToken: cfg.authToken,
          authUsername: cfg.authUsername,
          authPassword: cfg.authPassword,
          authHeaderName: cfg.authHeaderName,
          authHeaderValue: cfg.authHeaderValue,
          timeoutSeconds: cfg.timeoutSeconds,
        }),
      });
      if (res.status === 404 || res.status === 501) {
        setTryResult({ status: "missing" });
        return;
      }
      const text = await res.text();
      setTryResult({
        status: "ok",
        httpStatus: res.status,
        bodyPreview: text.length > 240 ? `${text.slice(0, 240)}…` : text,
      });
    } catch {
      // Most likely the dev server hasn't wired the test endpoint yet.
      // Treat as "coming v1.x" rather than a hard error.
      setTryResult({ status: "missing" });
    }
  }, [cfg]);

  return (
    <div data-testid="builder-http-config">
      <div style={FIELD_STYLE}>
        <label htmlFor="http-method" style={LABEL_STYLE}>Method</label>
        <select
          id="http-method"
          data-testid="builder-http-method"
          value={cfg.method}
          onChange={(e) => update({ method: e.target.value as HttpMethod })}
          style={INPUT_STYLE}
        >
          {HTTP_METHODS.map((m) => (
            <option key={m} value={m}>{m}</option>
          ))}
        </select>
      </div>
      <div style={FIELD_STYLE}>
        <label htmlFor="http-url" style={LABEL_STYLE}>URL</label>
        <input
          id="http-url"
          data-testid="builder-http-url"
          type="text"
          value={cfg.url}
          onChange={(e) => update({ url: e.target.value })}
          placeholder="https://api.example.com/v1/{{var}}"
          style={MONO_STYLE}
        />
        <div style={{ fontSize: 10.5, color: "#52525b", marginTop: 4 }}>
          Use <code>{"{{var}}"}</code> placeholders for upstream payload fields.
        </div>
      </div>
      <div style={FIELD_STYLE}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <span style={LABEL_STYLE}>Headers</span>
          <button
            type="button"
            data-testid="builder-http-add-header"
            onClick={addHeader}
            style={{
              padding: "3px 8px",
              borderRadius: 6,
              border: "1px dashed rgba(139,92,246,0.45)",
              background: "white",
              color: "#5b21b6",
              fontSize: 11,
              fontWeight: 600,
              cursor: "pointer",
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <Plus size={11} /> Add header
          </button>
        </div>
        {cfg.headers.length === 0 ? (
          <div style={{ fontSize: 11.5, color: "#a1a1aa" }}>No custom headers.</div>
        ) : (
          cfg.headers.map((h, idx) => (
            <div
              key={idx}
              style={{ display: "grid", gridTemplateColumns: "1fr 1fr auto", gap: 6, marginBottom: 6 }}
            >
              <input
                data-testid={`builder-http-header-key-${idx}`}
                placeholder="Header"
                value={h.key}
                onChange={(e) => setHeader(idx, { key: e.target.value })}
                style={MONO_STYLE}
              />
              <input
                data-testid={`builder-http-header-value-${idx}`}
                placeholder="Value"
                value={h.value}
                onChange={(e) => setHeader(idx, { value: e.target.value })}
                style={MONO_STYLE}
              />
              <button
                type="button"
                aria-label={`Remove header ${idx + 1}`}
                data-testid={`builder-http-header-remove-${idx}`}
                onClick={() => removeHeader(idx)}
                style={{
                  padding: "0 8px",
                  borderRadius: 6,
                  border: "1px solid rgba(244,63,94,0.35)",
                  background: "white",
                  color: "#9f1239",
                  cursor: "pointer",
                  display: "grid",
                  placeItems: "center",
                }}
              >
                <Trash2 size={12} />
              </button>
            </div>
          ))
        )}
      </div>
      <div style={FIELD_STYLE}>
        <label htmlFor="http-body" style={LABEL_STYLE}>Body</label>
        <textarea
          id="http-body"
          data-testid="builder-http-body"
          value={cfg.body}
          onChange={(e) => update({ body: e.target.value })}
          onBlur={() => {
            const pretty = prettyPrintJson(cfg.body);
            if (pretty !== null && pretty !== cfg.body) {
              update({ body: pretty });
            }
          }}
          rows={5}
          placeholder='{"key": "value"}'
          style={{ ...MONO_STYLE, resize: "vertical" }}
        />
        {bodyJsonError ? (
          <div
            data-testid="builder-http-body-json-error"
            style={{
              marginTop: 4,
              fontSize: 11,
              color: "#b91c1c",
              fontFamily: "var(--font-mono)",
            }}
          >
            {bodyJsonError}
          </div>
        ) : null}
      </div>
      <div style={FIELD_STYLE}>
        <label htmlFor="http-auth" style={LABEL_STYLE}>Auth</label>
        <select
          id="http-auth"
          data-testid="builder-http-auth"
          value={cfg.auth}
          onChange={(e) => update({ auth: e.target.value as HttpAuthMode })}
          style={INPUT_STYLE}
        >
          {AUTH_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>
        {cfg.auth === "bearer" ? (
          <input
            data-testid="builder-http-auth-token"
            type="text"
            value={cfg.authToken}
            onChange={(e) => update({ authToken: e.target.value })}
            placeholder="Bearer token (or {{secrets.token}})"
            style={{ ...MONO_STYLE, marginTop: 6 }}
          />
        ) : null}
        {cfg.auth === "basic" ? (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginTop: 6 }}>
            <input
              data-testid="builder-http-auth-username"
              placeholder="Username"
              value={cfg.authUsername}
              onChange={(e) => update({ authUsername: e.target.value })}
              style={MONO_STYLE}
            />
            <input
              data-testid="builder-http-auth-password"
              placeholder="Password"
              type="password"
              value={cfg.authPassword}
              onChange={(e) => update({ authPassword: e.target.value })}
              style={MONO_STYLE}
            />
          </div>
        ) : null}
        {cfg.auth === "custom_header" ? (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginTop: 6 }}>
            <input
              data-testid="builder-http-auth-header-name"
              placeholder="Header name"
              value={cfg.authHeaderName}
              onChange={(e) => update({ authHeaderName: e.target.value })}
              style={MONO_STYLE}
            />
            <input
              data-testid="builder-http-auth-header-value"
              placeholder="Header value"
              value={cfg.authHeaderValue}
              onChange={(e) => update({ authHeaderValue: e.target.value })}
              style={MONO_STYLE}
            />
          </div>
        ) : null}
      </div>
      <div style={FIELD_STYLE}>
        <label htmlFor="http-timeout" style={LABEL_STYLE}>Timeout (seconds)</label>
        <input
          id="http-timeout"
          data-testid="builder-http-timeout"
          type="number"
          min={1}
          max={300}
          value={cfg.timeoutSeconds}
          onChange={(e) =>
            update({ timeoutSeconds: Number.parseInt(e.target.value || "30", 10) || 30 })
          }
          style={INPUT_STYLE}
        />
      </div>
      <button
        type="button"
        data-testid="builder-http-try"
        onClick={() => { void tryIt(); }}
        disabled={tryResult.status === "running"}
        style={{
          width: "100%",
          padding: "8px 12px",
          borderRadius: 8,
          background: "white",
          color: "#5b21b6",
          border: "1px solid rgba(139,92,246,0.4)",
          fontWeight: 600,
          fontSize: 12.5,
          cursor: tryResult.status === "running" ? "wait" : "pointer",
          display: "inline-flex",
          gap: 6,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Zap size={12} />
        {tryResult.status === "running" ? "Running…" : "Try it"}
      </button>
      {tryResult.status === "missing" ? (
        <div
          data-testid="builder-http-try-missing"
          style={{
            marginTop: 8,
            padding: "8px 10px",
            borderRadius: 6,
            background: "rgba(245,158,11,0.08)",
            border: "1px solid rgba(245,158,11,0.3)",
            color: "#92400e",
            fontSize: 11.5,
          }}
        >
          Test-call endpoint coming in v1.x — save the workflow and run it end-to-end to test for now.
        </div>
      ) : null}
      {tryResult.status === "ok" ? (
        <div
          data-testid="builder-http-try-result"
          style={{
            marginTop: 8,
            padding: "8px 10px",
            borderRadius: 6,
            background: "rgba(16,185,129,0.08)",
            border: "1px solid rgba(16,185,129,0.3)",
            color: "#065f46",
            fontSize: 11.5,
          }}
        >
          <strong style={{ fontFamily: "var(--font-mono)" }}>{tryResult.httpStatus}</strong>
          <pre style={{ margin: "4px 0 0", fontFamily: "var(--font-mono)", fontSize: 11, whiteSpace: "pre-wrap" }}>
            {tryResult.bodyPreview}
          </pre>
        </div>
      ) : null}
    </div>
  );
}
