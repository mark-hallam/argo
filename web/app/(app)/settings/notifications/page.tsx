"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useToast } from "@/lib/ui/use-toast";
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  fetchNotificationSettings,
  updateNotificationSettings,
  type NotificationSettings,
} from "@/lib/api/settings";

export default function NotificationsSettingsPage() {
  const { success, error: toastError } = useToast();
  const [persisted, setPersisted] = useState<NotificationSettings>(DEFAULT_NOTIFICATION_SETTINGS);
  const [draft, setDraft] = useState<NotificationSettings>(DEFAULT_NOTIFICATION_SETTINGS);
  const [isLoading, setLoading] = useState(true);
  const [isSaving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    fetchNotificationSettings()
      .then(({ value }) => {
        if (!active) return;
        setPersisted(value);
        setDraft(value);
      })
      .catch(() => {
        if (!active) return;
        setPersisted(DEFAULT_NOTIFICATION_SETTINGS);
        setDraft(DEFAULT_NOTIFICATION_SETTINGS);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const dirty = isDirty(persisted, draft);

  function update<K extends keyof NotificationSettings>(key: K, value: NotificationSettings[K]) {
    setDraft((curr) => ({ ...curr, [key]: value }));
  }

  async function save() {
    setSaving(true);
    const next = draft;
    setPersisted(next);
    try {
      await updateNotificationSettings(persisted, next);
      success("Notification preferences saved", {
        description: "New routing applies to the next inbound approval.",
      });
    } catch (err) {
      setPersisted(persisted);
      toastError("Couldn't save notifications", {
        description: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="settings-panel" data-testid="settings-notifications-panel">
      <div className="settings-panel-head">
        <div>
          <p className="settings-kicker">Notifications</p>
          <h2>Routing and digests</h2>
          <p>
            Workspace-level notification defaults keep approval work visible
            without forcing every teammate into every channel.
          </p>
        </div>
        <Link className="settings-inline-action" href="/settings/notify-channels">
          Manage channels
        </Link>
      </div>

      <div className="settings-fields">
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Approval escalation</span>
            <p>Minutes of inactivity before unanswered approvals are re-notified to admins.</p>
          </div>
          <input
            type="number"
            className="settings-input settings-input-narrow"
            data-testid="settings-notifications-escalation"
            min={5}
            max={1440}
            value={draft.approval_escalation_minutes}
            onChange={(e) =>
              update("approval_escalation_minutes", clampMinutes(e.target.valueAsNumber))
            }
            disabled={isLoading || isSaving}
            aria-label="Approval escalation minutes"
          />
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Daily digest</span>
            <p>Summarises open approvals, failed connector probes, and policy changes.</p>
          </div>
          <input
            type="time"
            className="settings-input settings-input-narrow"
            data-testid="settings-notifications-digest"
            value={draft.daily_digest_time}
            onChange={(e) => update("daily_digest_time", e.target.value || "08:30")}
            disabled={isLoading || isSaving}
            aria-label="Daily digest time"
          />
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Quiet hours</span>
            <p>Non-urgent messages wait until the next workspace day.</p>
          </div>
          <div className="settings-row">
            <input
              type="time"
              className="settings-input settings-input-narrow"
              data-testid="settings-notifications-quiet-start"
              value={draft.quiet_hours_start}
              onChange={(e) => update("quiet_hours_start", e.target.value || "18:00")}
              disabled={isLoading || isSaving}
              aria-label="Quiet hours start"
            />
            <span className="settings-row-sep">→</span>
            <input
              type="time"
              className="settings-input settings-input-narrow"
              data-testid="settings-notifications-quiet-end"
              value={draft.quiet_hours_end}
              onChange={(e) => update("quiet_hours_end", e.target.value || "08:00")}
              disabled={isLoading || isSaving}
              aria-label="Quiet hours end"
            />
          </div>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Email notifications</span>
            <p>Send approval and digest emails to the workspace billing contact.</p>
          </div>
          <label className="settings-toggle">
            <input
              type="checkbox"
              data-testid="settings-notifications-email"
              checked={draft.email_enabled}
              onChange={(e) => update("email_enabled", e.target.checked)}
              disabled={isLoading || isSaving}
              aria-label="Enable email notifications"
            />
            <span>{draft.email_enabled ? "On" : "Off"}</span>
          </label>
        </div>
        <div className="settings-field settings-field-editable">
          <div>
            <span className="settings-field-label">Push notifications</span>
            <p>Browser push pings for high-risk approvals during business hours.</p>
          </div>
          <label className="settings-toggle">
            <input
              type="checkbox"
              data-testid="settings-notifications-push"
              checked={draft.push_enabled}
              onChange={(e) => update("push_enabled", e.target.checked)}
              disabled={isLoading || isSaving}
              aria-label="Enable push notifications"
            />
            <span>{draft.push_enabled ? "On" : "Off"}</span>
          </label>
        </div>
      </div>

      <SaveBar
        dirty={dirty}
        isSaving={isSaving}
        onSave={save}
        onRevert={() => setDraft(persisted)}
        testid="settings-notifications-save"
      />
      <SettingsStyles />
    </section>
  );
}

function clampMinutes(raw: number): number {
  if (!Number.isFinite(raw)) return 60;
  return Math.min(1440, Math.max(5, Math.round(raw)));
}

function isDirty(a: NotificationSettings, b: NotificationSettings): boolean {
  return (
    a.approval_escalation_minutes !== b.approval_escalation_minutes ||
    a.daily_digest_time !== b.daily_digest_time ||
    a.quiet_hours_start !== b.quiet_hours_start ||
    a.quiet_hours_end !== b.quiet_hours_end ||
    a.email_enabled !== b.email_enabled ||
    a.push_enabled !== b.push_enabled
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
.settings-input-narrow { min-width: 120px; }
.settings-input:focus {
  outline: 2px solid color-mix(in srgb, var(--primary, #8b5cf6) 45%, transparent);
  outline-offset: 1px;
}
.settings-input:disabled { opacity: 0.55; cursor: not-allowed; }
.settings-row { display: inline-flex; align-items: center; gap: 10px; }
.settings-row-sep { color: var(--settings-muted, #52525b); font-weight: 700; }
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
`,
      }}
    />
  );
}
