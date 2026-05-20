"use client";

import { useState } from "react";

import { emailConfigSchema, type EmailConfig } from "./types";
import {
  TextField,
  NumberField,
  TextareaField,
  ToggleField,
  FieldShell,
  PrimaryButton,
} from "./shared";

interface Props {
  config: EmailConfig;
  onChange: (next: EmailConfig) => void;
  onTestConnection?: (cfg: EmailConfig) => Promise<{ ok: boolean; message: string }>;
}

export default function EmailConfigForm({ config, onChange, onTestConnection }: Props) {
  const [testState, setTestState] = useState<"idle" | "pending" | "ok" | "fail">("idle");
  const [testMessage, setTestMessage] = useState<string>("");

  const parsed = emailConfigSchema.safeParse(config);
  const errors = parsed.success ? {} : flattenErrors(parsed.error.issues);

  function patch(p: Partial<EmailConfig>) {
    onChange({ ...config, ...p });
  }

  async function handleTest() {
    setTestState("pending");
    try {
      if (!onTestConnection) {
        // Endpoint not yet wired — surface the gateway-coming-soon hint
        setTestState("fail");
        setTestMessage("Test endpoint coming in v1.x — your settings will still save.");
        return;
      }
      const res = await onTestConnection(config);
      setTestState(res.ok ? "ok" : "fail");
      setTestMessage(res.message);
    } catch (err) {
      setTestState("fail");
      setTestMessage(String(err));
    }
  }

  return (
    <div data-testid="email-config-form">
      <SectionHeading>IMAP credentials</SectionHeading>

      <div style={{ display: "flex", gap: 12 }}>
        <div style={{ flex: 2 }}>
          <TextField
            id="email-host"
            label="Server (host)"
            value={config.imapHost}
            onChange={(v) => patch({ imapHost: v })}
            placeholder="imap.example.com"
            error={errors.imapHost}
          />
        </div>
        <div style={{ flex: 1 }}>
          <NumberField
            id="email-port"
            label="Port"
            value={config.imapPort}
            min={1}
            max={65535}
            onChange={(v) => patch({ imapPort: v })}
            error={errors.imapPort}
          />
        </div>
      </div>

      <TextField
        id="email-username"
        label="Username"
        value={config.imapUsername}
        onChange={(v) => patch({ imapUsername: v })}
        placeholder="leads@yourcompany.com"
        error={errors.imapUsername}
      />

      <TextField
        id="email-password"
        label="Password"
        type="password"
        value={config.imapPassword}
        onChange={(v) => patch({ imapPassword: v })}
        placeholder="••••••••"
        error={errors.imapPassword}
        hint="Stored encrypted in the gateway secret store. TODO: reference a secret-ref once available."
      />

      <ToggleField
        id="email-tls"
        label="Use SSL / TLS"
        checked={config.useTls}
        onChange={(v) => patch({ useTls: v })}
        hint="Required for almost every IMAP host. Leave on unless your host explicitly forbids it."
      />

      <SectionHeading>Filter rules</SectionHeading>

      <TextareaField
        id="email-filter-from"
        label="From address (optional)"
        value={config.filterFrom ?? ""}
        onChange={(v) => patch({ filterFrom: v })}
        placeholder={"jane@example.com\n@acme.com"}
        rows={3}
        hint="One address or @domain per line. Matches inbound sender."
      />

      <TextField
        id="email-filter-subject"
        label="Subject contains (optional)"
        value={config.filterSubject ?? ""}
        onChange={(v) => patch({ filterSubject: v })}
        placeholder="[lead]"
      />

      <ToggleField
        id="email-filter-attach"
        label="Has attachment"
        checked={config.filterHasAttachment}
        onChange={(v) => patch({ filterHasAttachment: v })}
        hint="Only fire on emails that include at least one attachment."
      />

      <FieldShell label="Test connection">
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <PrimaryButton onClick={handleTest} disabled={testState === "pending"} testId="email-test-connection">
            {testState === "pending" ? "Testing…" : "Test connection"}
          </PrimaryButton>
          {testMessage ? (
            <span
              data-testid="email-test-result"
              style={{
                fontFamily: "Inter, sans-serif",
                fontSize: 12.5,
                color: testState === "ok" ? "#15803d" : "#b91c1c",
                fontWeight: 600,
              }}
            >
              {testState === "ok" ? "✓" : "✗"} {testMessage}
            </span>
          ) : null}
        </div>
      </FieldShell>
    </div>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "JetBrains Mono, monospace",
        fontSize: 10.5,
        letterSpacing: "0.18em",
        textTransform: "uppercase",
        fontWeight: 700,
        color: "#5b21b6",
        margin: "8px 0 14px",
      }}
    >
      {children}
    </div>
  );
}

function flattenErrors(issues: Array<{ path: (string | number)[]; message: string }>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const issue of issues) {
    const key = issue.path.join(".");
    if (!out[key]) out[key] = issue.message;
  }
  return out;
}
