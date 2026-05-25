import type { Metadata } from "next";
import type { ReactNode } from "react";
import { SettingsTabs } from "@/components/skycot/settings/SettingsTabs";

export const metadata: Metadata = {
  title: "Settings",
};

export default function SettingsLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <style dangerouslySetInnerHTML={{ __html: SETTINGS_LAYOUT_STYLES }} />
      <div className="settings-shell" data-testid="settings-shell">
        <header className="settings-shell-header">
          <div>
            <p className="settings-shell-eyebrow">Workspace settings</p>
            <h1>Settings</h1>
          </div>
          <p>
            Workspace, team, notifications, security, providers, integrations,
            billing, and data controls.
          </p>
        </header>

        <SettingsTabs />

        <div className="settings-content-grid" data-testid="settings-content-grid">
          <div className="settings-content-slot">{children}</div>
          <aside className="settings-aside" aria-label="Settings context">
            <section className="settings-aside-card">
              <span className="settings-aside-kicker">Workspace state</span>
              <h2>Production ready</h2>
              <p>
                Settings changes are tenant-scoped and visible in audit-linked
                operator flows.
              </p>
            </section>
            <section className="settings-aside-card">
              <span className="settings-aside-kicker">Audit posture</span>
              <h2>Every change leaves a trail</h2>
              <p>
                Security, access, connector, and export actions are designed to
                land in the same replayable ledger.
              </p>
            </section>
            <section className="settings-aside-card settings-aside-danger" id="danger-zone">
              <span className="settings-aside-kicker">Danger zone</span>
              <h2>Destructive changes are gated</h2>
              <p>
                Workspace transfer and deletion require owner confirmation and
                audit review before they can execute.
              </p>
            </section>
          </aside>
        </div>
      </div>
    </>
  );
}

const SETTINGS_LAYOUT_STYLES = `
.settings-shell {
  --settings-ink: var(--foreground, #1f2937);
  --settings-muted: var(--muted-foreground, #52525b);
  --settings-border: var(--border, #e5e7eb);
  --settings-card: var(--card, #ffffff);
  --settings-soft: color-mix(in srgb, var(--muted, #f4f4f5) 68%, transparent);
  font-family: "Inter", sans-serif;
  max-width: 1280px;
  margin: 0 auto;
  padding: 32px 28px 80px;
  color: var(--settings-ink);
}
.settings-shell-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 24px;
  margin-bottom: 22px;
}
.settings-shell-eyebrow,
.settings-kicker,
.settings-aside-kicker {
  margin: 0 0 7px;
  font-family: var(--font-mono);
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--settings-muted);
}
.settings-shell-header h1 {
  margin: 0;
  font-family: "Inter", sans-serif;
  font-size: 32px;
  line-height: 1.1;
  font-weight: 650;
  letter-spacing: 0;
}
.settings-shell-header > p {
  max-width: 520px;
  margin: 0;
  font-size: 14px;
  line-height: 1.55;
  color: var(--settings-muted);
}
.settings-tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  border: 1px solid var(--settings-border);
  border-radius: 10px;
  background: var(--settings-card);
  padding: 6px;
  margin-bottom: 22px;
  box-shadow: 0 12px 28px -24px rgba(15, 23, 42, 0.35);
}
.settings-tab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-height: 32px;
  padding: 0 7px;
  border-radius: 7px;
  color: var(--settings-muted);
  text-decoration: none;
  font-size: 11.5px;
  font-weight: 650;
  white-space: nowrap;
  border: 1px solid transparent;
  transition: background 140ms ease, color 140ms ease, border-color 140ms ease;
}
.settings-tab:hover {
  color: var(--settings-ink);
  background: var(--settings-soft);
}
.settings-tab[aria-current="page"] {
  color: var(--settings-ink);
  background: var(--settings-soft);
  border-color: var(--settings-border);
}
.settings-tab svg {
  width: 13px;
  height: 13px;
  flex: 0 0 auto;
}
.settings-danger-tab {
  margin-left: auto;
  color: var(--destructive, #b91c1c);
}
.settings-danger-tab:hover,
.settings-danger-tab[aria-current="page"] {
  color: var(--destructive, #b91c1c);
  background: color-mix(in srgb, var(--destructive, #b91c1c) 8%, transparent);
  border-color: color-mix(in srgb, var(--destructive, #b91c1c) 22%, var(--settings-border));
}
.settings-content-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(260px, 320px);
  gap: 22px;
  align-items: start;
}
.settings-content-slot {
  min-width: 0;
}
.settings-aside {
  display: grid;
  gap: 12px;
  position: sticky;
  top: 24px;
}
.settings-aside-card,
.settings-panel {
  border: 1px solid var(--settings-border);
  border-radius: 10px;
  background: var(--settings-card);
  box-shadow: 0 16px 34px -30px rgba(15, 23, 42, 0.35);
}
.settings-aside-card {
  padding: 16px;
}
.settings-aside-card h2 {
  margin: 0 0 6px;
  font-size: 14px;
  line-height: 1.3;
  font-weight: 700;
}
.settings-aside-card p {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--settings-muted);
}
.settings-aside-danger {
  border-color: color-mix(in srgb, var(--destructive, #b91c1c) 22%, var(--settings-border));
}
.settings-panel {
  padding: 22px;
}
.settings-panel-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 18px;
  margin-bottom: 18px;
}
.settings-panel h2 {
  margin: 0 0 6px;
  font-family: "Inter", sans-serif;
  font-size: 22px;
  line-height: 1.2;
  font-weight: 650;
  letter-spacing: 0;
}
.settings-panel p {
  margin: 0;
  color: var(--settings-muted);
  line-height: 1.55;
  font-size: 13.5px;
}
.settings-inline-action {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  min-height: 34px;
  border-radius: 7px;
  border: 1px solid var(--settings-border);
  padding: 0 12px;
  color: var(--settings-ink);
  text-decoration: none;
  font-size: 12.5px;
  font-weight: 700;
  background: var(--settings-soft);
}
.settings-fields {
  display: grid;
  gap: 10px;
}
.settings-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) max-content;
  gap: 18px;
  align-items: center;
  border: 1px solid var(--settings-border);
  border-radius: 9px;
  padding: 14px;
  background: var(--settings-soft);
}
.settings-field-label {
  display: block;
  margin-bottom: 3px;
  font-size: 13.5px;
  font-weight: 700;
  color: var(--settings-ink);
}
.settings-field strong,
.settings-field code {
  font-size: 13px;
  font-weight: 700;
  color: var(--settings-ink);
  white-space: nowrap;
}
.settings-field code {
  font-family: var(--font-mono);
  font-size: 12px;
}
.settings-shell .connectors-hero,
.settings-shell .notify-hero,
.settings-shell .knowledge-title {
  font-family: "Inter", sans-serif !important;
  letter-spacing: 0;
}
@media (max-width: 1100px) {
  .settings-shell {
    padding: 24px 18px 72px;
  }
  .settings-content-grid {
    grid-template-columns: 1fr;
  }
  .settings-aside {
    position: static;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
@media (max-width: 760px) {
  .settings-shell-header,
  .settings-panel-head {
    display: block;
  }
  .settings-shell-header > p,
  .settings-inline-action {
    margin-top: 12px;
  }
  .settings-aside {
    grid-template-columns: 1fr;
  }
  .settings-field {
    grid-template-columns: 1fr;
  }
  .settings-field strong,
  .settings-field code {
    white-space: normal;
  }
}
`;
