export default function WorkspaceSettingsPage() {
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
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Display name</span>
            <p>Shown in audit exports, invites, and approval notifications.</p>
          </div>
          <strong>SKYCOT Demo</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Default vertical</span>
            <p>Used when an operator creates a teammate without selecting a pack.</p>
          </div>
          <strong>Marketing operations</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Time zone</span>
            <p>Controls SLA windows, digest timing, and audit timestamps.</p>
          </div>
          <code>Australia/Melbourne</code>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Billing contact</span>
            <p>Receives invoices, export receipts, and plan-limit alerts.</p>
          </div>
          <strong>owner@skycot.example</strong>
        </div>
      </div>
    </section>
  );
}
