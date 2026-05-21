"use client";

/**
 * TriggerConfigDrawer — dispatcher for per-kind trigger config UX.
 *
 * The builder hands this drawer a `triggerKind` (one of manual /
 * schedule / webhook / email / form / slack) and the current config
 * object. The drawer renders the matching tailored form and pipes
 * onChange back to the builder, which serializes into TOML via the
 * existing step-config save path.
 *
 * Manual + slack triggers are recognised so the dispatcher doesn't
 * crash; they get a "no extra config" message because the existing
 * generic step drawer already covers their needs.
 */

import { useEffect, useMemo } from "react";
import { X } from "lucide-react";

import WebhookConfigForm from "./WebhookConfigForm";
import ScheduleConfigForm from "./ScheduleConfigForm";
import EmailConfigForm from "./EmailConfigForm";
import FormConfigForm from "./FormConfigForm";
import {
  type TriggerConfig,
  type TriggerKind,
  defaultWebhookConfig,
  defaultScheduleConfig,
  defaultEmailConfig,
  defaultFormConfig,
} from "./types";
import { PrimaryButton, SecondaryButton } from "./shared";

interface Props {
  triggerKind: TriggerKind;
  workflowId: string;
  workspaceTimezone: string;
  config: TriggerConfig | null;
  onChange: (next: TriggerConfig) => void;
  onClose: () => void;
  onSave?: () => void;
}

const KIND_LABEL: Record<TriggerKind, string> = {
  manual: "Manual trigger",
  schedule: "Schedule",
  webhook: "Webhook",
  email: "Email",
  form: "Public form",
  slack: "Slack",
};

export default function TriggerConfigDrawer({
  triggerKind,
  workflowId,
  workspaceTimezone,
  config,
  onChange,
  onClose,
  onSave,
}: Props) {
  // Resolve a working config: either the parent's existing one, or a
  // freshly hydrated default for this trigger kind. The default is
  // memoised on (kind, workflowId, timezone) so render is stable.
  const hydrated = useMemo(
    () => hydrateDefault(triggerKind, workflowId, workspaceTimezone),
    [triggerKind, workflowId, workspaceTimezone],
  );
  const resolved = config ?? hydrated;

  // Push the hydrated default up to the parent on the next tick so the
  // parent's state owns it from then on. Empty dep — fires only once
  // per (kind) drawer mount when the parent didn't seed a config.
  useEffect(() => {
    if (!config && hydrated) {
      onChange(hydrated);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <aside
      className="trigger-config-drawer"
      data-testid={`trigger-drawer-${triggerKind}`}
      style={{
        width: 420,
        background: "white",
        borderLeft: "1px solid var(--border)",
        padding: 24,
        overflowY: "auto",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <div>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              letterSpacing: "0.16em",
              textTransform: "uppercase",
              fontWeight: 700,
              color: "var(--muted-foreground)",
            }}
          >
            TRIGGER CONFIG
          </div>
          <h2
            style={{
              fontFamily: "Inter, sans-serif",
              fontSize: 20,
              fontWeight: 700,
              color: "var(--foreground)",
              margin: "4px 0 0",
            }}
          >
            {KIND_LABEL[triggerKind]}
          </h2>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close trigger config"
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            color: "var(--muted-foreground)",
            padding: 6,
            borderRadius: 6,
          }}
        >
          <X size={18} />
        </button>
      </header>

      <div style={{ flex: 1, minHeight: 0 }}>
        {renderForm(triggerKind, resolved, onChange)}
      </div>

      {onSave ? (
        <footer
          style={{
            marginTop: 18,
            paddingTop: 14,
            borderTop: "1px solid var(--border)",
            display: "flex",
            gap: 8,
            justifyContent: "flex-end",
          }}
        >
          <SecondaryButton onClick={onClose}>Cancel</SecondaryButton>
          <PrimaryButton onClick={onSave} testId="trigger-drawer-save">Save</PrimaryButton>
        </footer>
      ) : null}
    </aside>
  );
}

function renderForm(
  kind: TriggerKind,
  config: TriggerConfig | null,
  onChange: (next: TriggerConfig) => void,
) {
  if (!config) {
    return <NoExtraConfig kind={kind} />;
  }
  switch (kind) {
    case "webhook":
      return config.kind === "webhook"
        ? <WebhookConfigForm config={config} onChange={onChange} />
        : null;
    case "schedule":
      return config.kind === "schedule"
        ? <ScheduleConfigForm config={config} onChange={onChange} />
        : null;
    case "email":
      return config.kind === "email"
        ? <EmailConfigForm config={config} onChange={onChange} />
        : null;
    case "form":
      return config.kind === "form"
        ? <FormConfigForm config={config} onChange={onChange} />
        : null;
    case "manual":
    case "slack":
      return <NoExtraConfig kind={kind} />;
  }
}

function NoExtraConfig({ kind }: { kind: TriggerKind }) {
  const message =
    kind === "manual"
      ? "Manual triggers fire when an operator clicks Run. No extra config needed."
      : "Slack triggers use the workspace's existing Slack connection. Configure that in Settings → Integrations.";
  return (
    <div
      data-testid={`trigger-no-extra-config-${kind}`}
      style={{
        padding: "14px 16px",
        borderRadius: 10,
        background: "rgba(139,92,246,0.08)",
        border: "1px solid rgba(139,92,246,0.2)",
        color: "var(--primary)",
        fontSize: 13.5,
        fontFamily: "Inter, sans-serif",
        lineHeight: 1.55,
      }}
    >
      {message}
    </div>
  );
}

function hydrateDefault(kind: TriggerKind, workflowId: string, timezone: string): TriggerConfig | null {
  switch (kind) {
    case "webhook": return defaultWebhookConfig(workflowId);
    case "schedule": return defaultScheduleConfig(timezone);
    case "email": return defaultEmailConfig();
    case "form": return defaultFormConfig(workflowId);
    case "manual":
    case "slack":
      return null;
  }
}
