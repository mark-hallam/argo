import Link from "next/link";

export default function ProvidersSettingsPage() {
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
          Review API keys
        </Link>
      </div>

      <div className="settings-fields">
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Primary provider</span>
            <p>Default model lane for workers that do not set an override.</p>
          </div>
          <strong>Anthropic</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Fallback behavior</span>
            <p>Provider failures stop the action and write an audit row.</p>
          </div>
          <strong>Fail closed</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Per-worker overrides</span>
            <p>Worker-specific provider choices remain visible in worker settings.</p>
          </div>
          <strong>Audit logged</strong>
        </div>
      </div>
    </section>
  );
}
