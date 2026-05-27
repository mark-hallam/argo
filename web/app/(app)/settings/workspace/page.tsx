"use client";

import { useEffect, useState } from "react";
import { ThemePicker } from "@/components/skycot/ThemePicker";
import { useToast } from "@/lib/ui/use-toast";
import {
  DEFAULT_WORKSPACE_SETTINGS,
  TIMEZONE_OPTIONS,
  VERTICAL_OPTIONS,
  fetchWorkspaceSettings,
  updateWorkspaceSettings,
  type WorkspaceSettings,
} from "@/lib/api/settings";

export default function WorkspaceSettingsPage() {
  const { success, error: toastError } = useToast();
  const [persisted, setPersisted] = useState<WorkspaceSettings>(DEFAULT_WORKSPACE_SETTINGS);
  const [draft, setDraft] = useState<WorkspaceSettings>(DEFAULT_WORKSPACE_SETTINGS);
  const [isLoading, setLoading] = useState(true);
  const [isSaving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    fetchWorkspaceSettings()
      .then(({ value }) => {
        if (!active) return;
        setPersisted(value);
        setDraft(value);
      })
      .catch(() => {
        if (!active) return;
        setPersisted(DEFAULT_WORKSPACE_SETTINGS);
        setDraft(DEFAULT_WORKSPACE_SETTINGS);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const dirty = isDirty(persisted, draft);

  function update<K extends keyof WorkspaceSettings>(key: K, value: WorkspaceSettings[K]) {
    setDraft((curr) => ({ ...curr, [key]: value }));
  }

  async function save() {
    setSaving(true);
    const next = draft;
    setPersisted(next);
    try {
      await updateWorkspaceSettings(persisted, next);
      success("Workspace saved", { description: "Operators will see the new values on next reload." });
    } catch (err) {
      setPersisted(persisted);
      toastError("Couldn't save workspace", {
        description: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="settings-panel" data-testid="settings-workspace-panel">
      <div className="settings-panel-head">
        <div>
          <p className="settings-kicker">Workspace</p>
          <h2>Workspace identity</h2>
          <p>
            The values operators check first before inviting teammates or wiring
            production connectors.
          </p>
        </div>
      </div>

      <div className="settings-fields">
        <div className="settings-field" data-testid="settings-workspace-theme-picker">
          <div>
            <span className="settings-field-label">Appearance</span>
            <p>Set the console color mode and tenant brand preview for this browser.</p>
          </div>
          <ThemePicker />
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Display name</span>
            <p>Shown in audit exports, invites, and approval notifications.</p>
          </div>
          <input
            type="text"
            className="settings-input"
            data-testid="settings-workspace-name"
            value={draft.name}
            onChange={(e) => update("name", e.target.value)}
            disabled={isLoading || isSaving}
            aria-label="Workspace display name"
          />
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Description</span>
            <p>One-line summary shown to invited operators and on audit headers.</p>
          </div>
          <input
            type="text"
            className="settings-input"
            data-testid="settings-workspace-description"
            value={draft.description}
            onChange={(e) => update("description", e.target.value)}
            disabled={isLoading || isSaving}
            aria-label="Workspace description"
          />
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Default vertical</span>
            <p>Used when an operator creates a teammate without selecting a pack.</p>
          </div>
          <select
            className="settings-input"
            data-testid="settings-workspace-vertical"
            value={draft.default_vertical}
            onChange={(e) => update("default_vertical", e.target.value)}
            disabled={isLoading || isSaving}
            aria-label="Default vertical"
          >
            {VERTICAL_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Time zone</span>
            <p>Controls SLA windows, digest timing, and audit timestamps.</p>
          </div>
          <select
            className="settings-input"
            data-testid="settings-workspace-timezone"
            value={draft.timezone}
            onChange={(e) => update("timezone", e.target.value)}
            disabled={isLoading || isSaving}
            aria-label="Workspace timezone"
          >
            {TIMEZONE_OPTIONS.map((tz) => (
              <option key={tz} value={tz}>
                {tz}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Billing contact</span>
            <p>Receives invoices, export receipts, and plan-limit alerts.</p>
          </div>
          <input
            type="email"
            className="settings-input"
            data-testid="settings-workspace-billing"
            value={draft.billing_contact}
            onChange={(e) => update("billing_contact", e.target.value)}
            disabled={isLoading || isSaving}
            aria-label="Billing contact email"
          />
        </div>
      </div>

      <SaveBar
        dirty={dirty}
        isSaving={isSaving}
        onSave={save}
        onRevert={() => setDraft(persisted)}
        testid="settings-workspace-save"
      />
      <SettingsStyles />
    </section>
  );
}

function isDirty(a: WorkspaceSettings, b: WorkspaceSettings): boolean {
  return (
    a.name !== b.name ||
    a.description !== b.description ||
    a.default_vertical !== b.default_vertical ||
    a.timezone !== b.timezone ||
    a.billing_contact !== b.billing_contact
  );
}

function SaveBar({
  dirty,
  isSaving,
  onSave,
  onRevert,
  testid,
}: {
  dirty: boolean;
  isSaving: boolean;
  onSave: () => void;
  onRevert: () => void;
  testid: string;
}) {
  if (!dirty && !isSaving) return null;
  return (
    <div className="settings-save-bar" data-testid={`${testid}-bar`}>
      <span className="settings-save-hint">Unsaved changes</span>
      <button
        type="button"
        className="settings-revert-btn"
        onClick={onRevert}
        disabled={isSaving}
      >
        Revert
      </button>
      <button
        type="button"
        className="settings-save-btn"
        data-testid={testid}
        onClick={onSave}
        disabled={isSaving}
      >
        {isSaving ? "Saving…" : "Save changes"}
      </button>
    </div>
  );
}

function SettingsStyles() {
  return (
    <style
      dangerouslySetInnerHTML={{
        __html: `
.settings-field-editable {
  align-items: center;
}
.settings-input {
  flex: 0 0 auto;
  min-width: 220px;
  border: 1px solid var(--settings-border, #e5e7eb);
  border-radius: 7px;
  padding: 8px 10px;
  font-family: "Inter", sans-serif;
  font-size: 13px;
  font-weight: 600;
  color: var(--settings-ink, #1f2937);
  background: var(--settings-card, #ffffff);
}
.settings-input:focus {
  outline: 2px solid color-mix(in srgb, var(--primary, #8b5cf6) 45%, transparent);
  outline-offset: 1px;
}
.settings-input:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.settings-save-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 18px;
  padding-top: 14px;
  border-top: 1px solid var(--settings-border, #e5e7eb);
}
.settings-save-hint {
  margin-right: auto;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--settings-muted, #52525b);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.settings-save-btn,
.settings-revert-btn {
  min-height: 34px;
  border-radius: 7px;
  padding: 0 14px;
  font-family: "Inter", sans-serif;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  border: 1px solid var(--settings-border, #e5e7eb);
  background: var(--settings-card, #ffffff);
  color: var(--settings-ink, #1f2937);
}
.settings-save-btn {
  border-color: transparent;
  background: linear-gradient(90deg, #8b5cf6 0%, #ec4899 100%);
  color: #fff;
}
.settings-save-btn:disabled,
.settings-revert-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
`,
      }}
    />
  );
}
