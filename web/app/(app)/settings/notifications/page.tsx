import Link from "next/link";

export default function NotificationsSettingsPage() {
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
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Approval escalation</span>
            <p>Unanswered approvals are re-notified to admins after one business hour.</p>
          </div>
          <strong>Admin fallback</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Daily digest</span>
            <p>Summarizes open approvals, failed connector probes, and policy changes.</p>
          </div>
          <strong>08:30 local time</strong>
        </div>
        <div className="settings-field">
          <div>
            <span className="settings-field-label">Quiet hours</span>
            <p>Non-urgent messages wait until the next workspace day.</p>
          </div>
          <code>18:00-08:00</code>
        </div>
      </div>
    </section>
  );
}
