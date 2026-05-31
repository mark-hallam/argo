"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useToast } from "@/lib/ui/use-toast";
import {
  DEFAULT_PROVIDER_SETTINGS,
  FALLBACK_OPTIONS,
  PROVIDER_OPTIONS,
  fetchProviderSettings,
  updateProviderSettings,
  type ProviderSettings,
} from "@/lib/api/settings";
import {
  fetchProviderKeys,
  type ProviderKeyStatus,
} from "@/lib/api/provider-keys";
import { ProviderKeyForm } from "@/components/skycot/settings/ProviderKeyForm";

export default function ProvidersSettingsPage() {
  const { success, error: toastError } = useToast();
  const [persisted, setPersisted] = useState<ProviderSettings>(DEFAULT_PROVIDER_SETTINGS);
  const [draft, setDraft] = useState<ProviderSettings>(DEFAULT_PROVIDER_SETTINGS);
  const [isLoading, setLoading] = useState(true);
  const [isSaving, setSaving] = useState(false);
  const [keys, setKeys] = useState<ProviderKeyStatus[]>([]);
  const [keysError, setKeysError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetchProviderSettings()
      .then(({ value }) => {
        if (!active) return;
        setPersisted(value);
        setDraft(value);
      })
      .catch(() => {
        if (!active) return;
        setPersisted(DEFAULT_PROVIDER_SETTINGS);
        setDraft(DEFAULT_PROVIDER_SETTINGS);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    fetchProviderKeys()
      .then(({ keys: next }) => {
        if (!active) return;
        setKeys(next);
        setKeysError(null);
      })
      .catch((err) => {
        if (!active) return;
        setKeys([]);
        setKeysError(
          err instanceof Error
            ? err.message
            : "Provider credential storage endpoint is unavailable.",
        );
      });
    return () => {
      active = false;
    };
  }, []);

  const dirty = isDirty(persisted, draft);

  function update<K extends keyof ProviderSettings>(key: K, value: ProviderSettings[K]) {
    setDraft((curr) => ({ ...curr, [key]: value }));
  }

  async function save() {
    setSaving(true);
    const next = draft;
    setPersisted(next);
    try {
      await updateProviderSettings(persisted, next);
      success("Provider policy saved", {
        description: "Workers will pick up the new routing on next turn.",
      });
    } catch (err) {
      setPersisted(persisted);
      toastError("Couldn't save provider policy", {
        description: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="settings-panel" data-testid="settings-providers-panel">
      <div className="settings-panel-head">
        <div>
          <p className="settings-kicker">Providers</p>
          <h2>Model provider policy</h2>
          <p>
            Provider routing is governed by the workspace config and surfaced
            here as an operator-readable control plane summary.
          </p>
        </div>
        <Link className="settings-inline-action" href="/settings/api-keys">
          Manage all keys
        </Link>
      </div>

      <div className="settings-fields">
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Primary provider</span>
            <p>Default model lane for workers that do not set an override.</p>
          </div>
          <select
            className="settings-input"
            data-testid="settings-providers-primary"
            value={draft.primary_provider}
            onChange={(e) => update("primary_provider", e.target.value as ProviderSettings["primary_provider"])}
            disabled={isLoading || isSaving}
            aria-label="Primary provider"
          >
            {PROVIDER_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Fallback behavior</span>
            <p>What happens when the primary provider returns an error.</p>
          </div>
          <select
            className="settings-input"
            data-testid="settings-providers-fallback"
            value={draft.fallback_behavior}
            onChange={(e) => update("fallback_behavior", e.target.value as ProviderSettings["fallback_behavior"])}
            disabled={isLoading || isSaving}
            aria-label="Fallback behavior"
          >
            {FALLBACK_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Per-worker overrides</span>
            <p>Workers can override provider routing inside their manifest. All overrides land in the audit log.</p>
          </div>
          <label className="settings-toggle">
            <input
              type="checkbox"
              data-testid="settings-providers-allow-overrides"
              checked={draft.allow_worker_overrides}
              onChange={(e) => update("allow_worker_overrides", e.target.checked)}
              disabled={isLoading || isSaving}
              aria-label="Allow per-worker provider overrides"
            />
            <span>{draft.allow_worker_overrides ? "Allowed" : "Locked"}</span>
          </label>
        </div>
      </div>

      <SaveBar
        dirty={dirty}
        isSaving={isSaving}
        onSave={save}
        onRevert={() => setDraft(persisted)}
        testid="settings-providers-save"
      />

      <div className="settings-keys-section" data-testid="provider-keys-section">
        <div className="settings-keys-head">
          <h3>Provider API keys</h3>
          <p>
            Plug in your own Anthropic, OpenAI or other provider keys. Workers
            switch from the mock provider as soon as a real key lands.
          </p>
        </div>
        {keysError ? (
          <div className="settings-keys-unavailable" role="status" aria-live="polite">
            <strong>Provider-key storage is not live.</strong>
            <span>
              The gateway did not serve <code>/api/v1/providers/keys</code>. No provider
              credentials are being mocked in demo-live mode.
            </span>
          </div>
        ) : null}
        <div className="settings-keys-list">
          {PROVIDER_OPTIONS.map((opt) => {
            const status =
              keys.find((k) => k.provider === opt.value) ?? {
                provider: opt.value,
                configured: false,
                masked_preview: null,
                updated_at: null,
              };
            return (
              <ProviderKeyForm
                key={opt.value}
                provider={opt.value}
                label={opt.label}
                status={status}
                unavailableReason={keysError ?? undefined}
                onSaved={(next) => {
                  setKeys((prev) => {
                    const without = prev.filter(
                      (k) => k.provider !== next.provider,
                    );
                    return [...without, next];
                  });
                  success(`${opt.label} key saved`, {
                    description: "Workers will pick up the new credential on next turn.",
                  });
                }}
                onError={(message) => {
                  toastError(`Couldn't save ${opt.label} key`, {
                    description: message,
                  });
                }}
              />
            );
          })}
        </div>
      </div>

      <SettingsStyles />
    </section>
  );
}

function isDirty(a: ProviderSettings, b: ProviderSettings): boolean {
  return (
    a.primary_provider !== b.primary_provider ||
    a.fallback_behavior !== b.fallback_behavior ||
    a.allow_worker_overrides !== b.allow_worker_overrides
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
.settings-field-editable { align-items: center; }
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
.settings-input:disabled { opacity: 0.55; cursor: not-allowed; }
.settings-toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  color: var(--settings-ink, #1f2937);
}
.settings-toggle input { width: 18px; height: 18px; cursor: pointer; }
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
.settings-revert-btn:disabled { opacity: 0.55; cursor: not-allowed; }
.settings-keys-section {
  margin-top: 22px;
  padding-top: 18px;
  border-top: 1px solid var(--settings-border, #e5e7eb);
}
.settings-keys-head h3 {
  margin: 0 0 4px;
  font-family: "Inter", sans-serif;
  font-size: 15px;
  font-weight: 700;
  color: var(--settings-ink, #1f2937);
}
.settings-keys-head p {
  margin: 0 0 14px;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--settings-muted, #52525b);
}
.settings-keys-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.settings-keys-unavailable {
  display: grid;
  gap: 4px;
  margin: 0 0 14px;
  padding: 12px 14px;
  border: 1px solid rgba(236, 72, 153, 0.32);
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent, #ec4899) 8%, var(--settings-card, #ffffff));
  color: var(--settings-ink, #1f2937);
  font-size: 12.5px;
  line-height: 1.45;
}
.settings-keys-unavailable strong {
  font-size: 13px;
}
.settings-keys-unavailable code {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 12px;
}
.provider-key-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 14px;
  border: 1px solid var(--settings-border, #e5e7eb);
  border-radius: 8px;
  background: var(--settings-card, #ffffff);
}
.provider-key-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  min-width: 0;
}
.provider-key-label {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--settings-ink, #1f2937);
}
.provider-key-pill {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.provider-key-pill-configured {
  background: rgba(16, 185, 129, 0.12);
  color: #047857;
}
.provider-key-pill-mock {
  background: rgba(148, 163, 184, 0.18);
  color: #475569;
}
.provider-key-pill-unavailable {
  background: rgba(236, 72, 153, 0.12);
  color: #be185d;
}
.provider-key-preview {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 12px;
  color: var(--settings-muted, #52525b);
}
.provider-key-action,
.provider-key-save,
.provider-key-cancel {
  min-height: 32px;
  border-radius: 7px;
  padding: 0 12px;
  font-family: "Inter", sans-serif;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  border: 1px solid var(--settings-border, #e5e7eb);
  background: var(--settings-card, #ffffff);
  color: var(--settings-ink, #1f2937);
}
.provider-key-save {
  border-color: transparent;
  background: linear-gradient(90deg, #8b5cf6 0%, #ec4899 100%);
  color: #fff;
}
.provider-key-action:disabled,
.provider-key-save:disabled,
.provider-key-cancel:disabled { opacity: 0.55; cursor: not-allowed; }
.provider-key-form {
  display: flex;
  align-items: center;
  gap: 8px;
}
.provider-key-input {
  min-width: 240px;
  border: 1px solid var(--settings-border, #e5e7eb);
  border-radius: 7px;
  padding: 7px 10px;
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 12.5px;
  color: var(--settings-ink, #1f2937);
  background: var(--settings-card, #ffffff);
}
.provider-key-input:focus {
  outline: 2px solid color-mix(in srgb, var(--primary, #8b5cf6) 45%, transparent);
  outline-offset: 1px;
}
`,
      }}
    />
  );
}
