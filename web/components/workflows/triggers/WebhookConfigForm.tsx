"use client";

import { useMemo, useState } from "react";
import { Copy, RefreshCw, Send, Check } from "lucide-react";

import {
  webhookConfigSchema,
  webhookUrl,
  generateSecretSuffix,
  WEBHOOK_METHODS,
  type WebhookConfig,
  type WebhookMethod,
} from "./types";
import {
  TextField,
  FieldShell,
  PillButton,
  PrimaryButton,
  SecondaryButton,
  triggerStyles,
} from "./shared";

interface Props {
  config: WebhookConfig;
  onChange: (next: WebhookConfig) => void;
  onTestPing?: (url: string) => Promise<{ status: number; body: string }>;
}

export default function WebhookConfigForm({ config, onChange, onTestPing }: Props) {
  const [copied, setCopied] = useState(false);
  const [confirmRotate, setConfirmRotate] = useState(false);
  const [pingState, setPingState] = useState<"idle" | "pending" | "ok" | "fail">("idle");
  const [pingResponse, setPingResponse] = useState<{ status: number; body: string } | null>(null);

  const url = useMemo(() => webhookUrl(config), [config]);
  const parsed = webhookConfigSchema.safeParse(config);
  const errors = parsed.success ? {} : flattenErrors(parsed.error.issues);

  function patch(p: Partial<WebhookConfig>) {
    onChange({ ...config, ...p });
  }

  function toggleMethod(m: WebhookMethod) {
    const next = config.allowedMethods.includes(m)
      ? config.allowedMethods.filter((x) => x !== m)
      : [...config.allowedMethods, m];
    patch({ allowedMethods: next });
  }

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore; clipboard API not available in test envs
    }
  }

  function handleRotate() {
    if (!confirmRotate) {
      setConfirmRotate(true);
      return;
    }
    patch({ secretSuffix: generateSecretSuffix() });
    setConfirmRotate(false);
    setPingState("idle");
    setPingResponse(null);
  }

  async function handleTestPing() {
    setPingState("pending");
    try {
      const fn = onTestPing ?? defaultTestPing;
      const res = await fn(url);
      setPingResponse(res);
      setPingState(res.status >= 200 && res.status < 300 ? "ok" : "fail");
    } catch (err) {
      setPingResponse({ status: 0, body: String(err) });
      setPingState("fail");
    }
  }

  const curlSnippet = `curl -X POST ${url} \\\n  -H "Content-Type: application/json" \\\n  -d '{"event": "test"}'`;

  return (
    <div data-testid="webhook-config-form">
      <FieldShell label="Webhook URL" hint="POST to this URL to fire the workflow.">
        <div style={{ display: "flex", gap: 8 }}>
          <input
            readOnly
            value={url}
            data-testid="webhook-url"
            style={{ ...triggerStyles.input, background: "var(--background)", fontFamily: "var(--font-mono)", fontSize: 12.5 }}
          />
          <SecondaryButton onClick={handleCopy} testId="webhook-copy">
            {copied ? <Check size={13} /> : <Copy size={13} />} {copied ? "Copied" : "Copy"}
          </SecondaryButton>
        </div>
      </FieldShell>

      <FieldShell
        label="Rotate secret"
        hint={confirmRotate ? "This deactivates the current URL immediately." : "Generate a new URL suffix."}
      >
        <SecondaryButton onClick={handleRotate} testId="webhook-regenerate">
          <RefreshCw size={13} /> {confirmRotate ? "Confirm regenerate" : "Regenerate"}
        </SecondaryButton>
        {confirmRotate ? (
          <button
            type="button"
            onClick={() => setConfirmRotate(false)}
            style={{ marginLeft: 8, padding: "9px 14px", background: "transparent", border: "none", color: "var(--muted-foreground)", fontSize: 13, cursor: "pointer" }}
          >
            Cancel
          </button>
        ) : null}
      </FieldShell>

      <FieldShell label="Test ping" hint="Sends a sample POST to verify the URL responds.">
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <PrimaryButton onClick={handleTestPing} disabled={pingState === "pending"} testId="webhook-test-ping">
            <Send size={12} /> {pingState === "pending" ? "Sending…" : "Send test request"}
          </PrimaryButton>
          {pingResponse ? (
            <span
              data-testid="webhook-ping-status"
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12.5,
                color: pingState === "ok" ? "#15803d" : "#b91c1c",
                fontWeight: 700,
              }}
            >
              {pingState === "ok" ? "✓" : "✗"} HTTP {pingResponse.status}
            </span>
          ) : null}
        </div>
        {pingResponse ? (
          <pre
            data-testid="webhook-ping-body"
            style={{
              marginTop: 10,
              padding: "10px 12px",
              borderRadius: 8,
              background: "var(--foreground)",
              color: "var(--background)",
              fontFamily: "var(--font-mono)",
              fontSize: 11.5,
              whiteSpace: "pre-wrap",
              maxHeight: 160,
              overflow: "auto",
            }}
          >
            {pingResponse.body}
          </pre>
        ) : null}
      </FieldShell>

      <FieldShell label="cURL snippet" hint="Copy-paste-ready test command.">
        <pre
          style={{
            margin: 0,
            padding: "10px 12px",
            borderRadius: 8,
            background: "var(--background)",
            border: "1px solid #ece9f5",
            fontFamily: "var(--font-mono)",
            fontSize: 11.5,
            color: "var(--foreground)",
            whiteSpace: "pre-wrap",
          }}
        >
          {curlSnippet}
        </pre>
      </FieldShell>

      <FieldShell label="Allowed methods" error={errors.allowedMethods}>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {WEBHOOK_METHODS.map((m) => (
            <PillButton
              key={m}
              active={config.allowedMethods.includes(m)}
              onClick={() => toggleMethod(m)}
            >
              {m}
            </PillButton>
          ))}
        </div>
      </FieldShell>

      <FieldShell label="Auth">
        <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
          <PillButton active={config.authMode === "none"} onClick={() => patch({ authMode: "none" })}>
            None
          </PillButton>
          <PillButton active={config.authMode === "header"} onClick={() => patch({ authMode: "header" })}>
            Header check
          </PillButton>
        </div>
        {config.authMode === "header" ? (
          <>
            <TextField
              id="webhook-auth-name"
              label="Header name"
              value={config.authHeaderName ?? ""}
              onChange={(v) => patch({ authHeaderName: v })}
              placeholder="X-Skycot-Auth"
              error={errors.authHeaderName}
            />
            <TextField
              id="webhook-auth-value"
              label="Header value"
              value={config.authHeaderValue ?? ""}
              onChange={(v) => patch({ authHeaderValue: v })}
              type="password"
              placeholder="••••••••"
              error={errors.authHeaderValue}
            />
          </>
        ) : null}
      </FieldShell>
    </div>
  );
}

async function defaultTestPing(url: string): Promise<{ status: number; body: string }> {
  try {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ event: "test", source: "skycot-builder" }),
    });
    const body = await res.text();
    return { status: res.status, body: body.slice(0, 2048) };
  } catch (err) {
    return { status: 0, body: `network error: ${String(err)}` };
  }
}

function flattenErrors(issues: Array<{ path: (string | number)[]; message: string }>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const issue of issues) {
    const key = issue.path.join(".");
    if (!out[key]) out[key] = issue.message;
  }
  return out;
}
