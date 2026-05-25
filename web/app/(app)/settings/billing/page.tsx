import Link from "next/link";

export default function BillingSettingsPage() {
  return (
    <section className="settings-panel" data-testid="settings-billing-panel">
      <div className="settings-panel-head">
        <div>
          <p className="settings-kicker">Billing</p>
          <h2>Plan and usage</h2>
          <p>
            Billing remains tied to the account-level billing surface while
            settings keeps the workspace context visible.
          </p>
        </div>
        <Link className="settings-inline-action" href="/billing">
          Open billing
        </Link>
      </div>

      <div className="settings-fields">
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Plan</span>
            <p>Controls connectors, notification channels, and worker concurrency.</p>
          </div>
          <strong>Pro</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Invoice owner</span>
            <p>Receives subscription receipts and usage-threshold alerts.</p>
          </div>
          <strong>Workspace owner</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Usage window</span>
            <p>Worker runs and connector calls reset on the plan anniversary.</p>
          </div>
          <code>Monthly</code>
        </div>
      </div>
    </section>
  );
}
