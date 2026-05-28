"use client";

import { useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  AlertCircle,
  Lock,
  Download,
  Loader2,
  ShieldCheck,
  Network,
  Monitor,
  Archive,
} from "lucide-react";
import {
  getSamlConfig,
  samlMetadataUrl,
  type SamlConfig,
} from "@/lib/api/sso";
import SecurityCard from "@/components/skycot/settings/SecurityCard";
import SamlConfigModal from "./SamlConfigModal";

/**
 * Security settings — `/settings/security`.
 *
 * Five-card layout: SSO + 2FA + IP allowlist + Active sessions + Audit
 * retention. Only SSO is wired end-to-end in v1.0; the other four are
 * read-only surfaces with honest "ships in v1.1" or "contact sales"
 * gating, per `feedback_no_theater_in_demo`.
 */

const EMPTY_CONFIG: SamlConfig = {
  idpEntityId: "",
  ssoUrl: "",
  certificate: "",
  flavor: "custom",
  allowOwnerBypass: true,
  attributeMappingJson: "",
};

type LoadState =
  | { kind: "loading" }
  | { kind: "ready"; configured: boolean; config: SamlConfig | null };

export default function SecuritySettingsPage() {
  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getSamlConfig()
      .then((status) => {
        if (cancelled) return;
        setLoad({ kind: "ready", configured: status.configured, config: status.config });
      })
      .catch(() => {
        if (cancelled) return;
        setLoad({ kind: "ready", configured: false, config: null });
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function onConfigSaved(saved: SamlConfig) {
    setLoad({ kind: "ready", configured: true, config: saved });
    setModalOpen(false);
  }

  const statusLabel = useMemo(() => statusLabelFor(load), [load]);

  return (
    <>
      <style jsx global>{`
        .security-stage { padding: 48px 32px 96px; max-width: 1180px; margin: 0 auto; }
        .security-eyebrow { font-family: var(--font-mono); font-size: 11px;
          color: #52525b; letter-spacing: 0.16em; text-transform: uppercase;
          font-weight: 700; margin-bottom: 8px; }
        .security-title { font-family: "Inter", sans-serif; font-size: 32px;
          font-weight: 600; color: #1e1b4b; margin: 0 0 6px; letter-spacing: -0.01em; }
        .security-sub { font-size: 15px; color: #3f3f46; margin: 0 0 36px; }
        .security-grid { display: grid; grid-template-columns: 1fr 1fr;
          gap: 16px; }
        @media (max-width: 760px) {
          .security-grid { grid-template-columns: 1fr; }
        }
        .security-card { background: white; border: 1px solid #ece9f5; border-radius: 14px;
          padding: 24px; display: flex; flex-direction: column; gap: 14px; }
        .security-card-wide { grid-column: 1 / -1; }
        .security-card-head { display: flex; align-items: flex-start;
          justify-content: space-between; gap: 16px; }
        .security-card-title { font-size: 18px; font-weight: 600; color: #1e1b4b;
          margin: 0; display: flex; align-items: center; gap: 10px; }
        .security-card-blurb { font-size: 14px; color: #3f3f46; margin: 6px 0 0; }
        .security-card-pill, .security-status-pill { display: inline-flex; align-items: center; gap: 6px;
          padding: 4px 10px; border-radius: 999px; font-size: 12px; font-weight: 600;
          border: 1px solid #ece9f5; background: #fafaff; color: #1e1b4b;
          font-family: var(--font-mono); letter-spacing: 0.04em; white-space: nowrap; }
        .security-card-pill-on, .security-status-on { color: #047857; border-color: #a7f3d0; background: #ecfdf5; }
        .security-card-pill-warn { color: #b45309; border-color: #fcd34d; background: #fffbeb; }
        .security-card-rows { display: grid; grid-template-columns: 1fr;
          gap: 6px 16px; padding: 12px; border-radius: 10px; background: #fafaff;
          font-size: 13px; color: #3f3f46; margin: 0; }
        .security-card-row { display: grid; grid-template-columns: 160px 1fr;
          gap: 4px 12px; align-items: baseline; }
        .security-card-row dt { color: #52525b; font-weight: 600;
          font-family: var(--font-mono); font-size: 11px;
          letter-spacing: 0.06em; text-transform: uppercase; margin: 0; }
        .security-card-row dd { margin: 0; word-break: break-word; }
        .security-actions { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
        .security-card-actions { display: flex; flex-wrap: wrap; gap: 10px;
          align-items: center; margin-top: auto; }
        .security-btn, .security-card-btn { padding: 9px 16px; border-radius: 10px; font-size: 13.5px;
          font-weight: 600; cursor: pointer; border: 1px solid #ece9f5;
          background: white; color: #1e1b4b; display: inline-flex; align-items: center;
          gap: 8px; }
        .security-btn:hover, .security-card-btn:hover:not(:disabled) { border-color: #8b5cf6; }
        .security-card-btn:disabled { opacity: 0.55; cursor: not-allowed; }
        .security-btn-primary { background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white; border: none; box-shadow: 0 4px 12px -4px rgba(139, 92, 246, 0.45); }
        .security-btn-primary:hover { filter: brightness(1.05); border: none; }
        .security-card-tooltip { font-size: 12px; color: #52525b;
          font-style: italic; }
        .security-summary { display: grid; grid-template-columns: 1fr 2fr; gap: 8px 16px;
          padding: 12px; border-radius: 10px; background: #fafaff;
          font-size: 13px; color: #3f3f46; }
        .security-summary dt { color: #52525b; font-weight: 600;
          font-family: var(--font-mono); font-size: 11px;
          letter-spacing: 0.06em; text-transform: uppercase; }
        .security-summary dd { margin: 0; word-break: break-all; }
        .security-error { color: #b91c1c; font-size: 13px; margin: 0; }
        .security-fallback { color: #3f3f46; font-size: 13.5px; margin: 0; }
        .security-load { color: #52525b; font-size: 14px;
          display: inline-flex; align-items: center; gap: 8px; }
        .security-sessions-table { width: 100%; border-collapse: collapse;
          font-size: 13px; color: #1e1b4b; }
        .security-sessions-table th, .security-sessions-table td {
          padding: 10px 12px; text-align: left; border-bottom: 1px solid #ece9f5; }
        .security-sessions-table th { font-family: var(--font-mono);
          font-size: 11px; letter-spacing: 0.06em; text-transform: uppercase;
          color: #52525b; font-weight: 700; }
        .security-sessions-table tbody tr:last-child td { border-bottom: none; }
        .security-sessions-empty { font-size: 13px; color: #52525b; margin: 0;
          padding: 12px; border-radius: 10px; background: #fafaff; }
      `}</style>
      <div className="security-stage">
        <header>
          <div className="security-eyebrow">⚙ Workspace · Security</div>
          <h1 className="security-title">Security</h1>
          <p className="security-sub">
            Single sign-on, two-factor authentication, IP allowlisting, active sessions,
            and audit retention.
          </p>
        </header>

        <div className="security-grid">
          <section className="security-card security-card-wide" data-testid="saml-card">
            <div className="security-card-head">
              <div>
                <h2 className="security-card-title">
                  <Lock size={18} aria-hidden /> Single Sign-On (SAML 2.0)
                </h2>
                <p className="security-card-blurb">
                  Federate sign-in through Okta, Azure AD, or a custom SAML 2.0
                  identity provider. New teammates whose email domain matches your
                  workspace will be redirected to your IdP automatically.
                </p>
              </div>
              <span
                className={
                  statusLabel.on ? "security-status-pill security-status-on" : "security-status-pill"
                }
                data-testid="saml-status-pill"
              >
                {statusLabel.on ? <CheckCircle2 size={12} /> : <AlertCircle size={12} />}
                {statusLabel.text}
              </span>
            </div>

            {load.kind === "loading" ? (
              <p className="security-load" data-testid="saml-loading">
                <Loader2 size={14} className="animate-spin" aria-hidden /> Loading SSO configuration…
              </p>
            ) : null}

            {load.kind === "ready" && !load.configured ? (
              <p className="security-fallback" data-testid="saml-fallback">
                Not configured · Set up SAML SSO to connect your identity provider.
              </p>
            ) : null}

            {load.kind === "ready" && load.configured && load.config ? (
              <ConfigSummary config={load.config} />
            ) : null}

            <div className="security-actions">
              <button
                type="button"
                className="security-btn security-btn-primary"
                onClick={() => setModalOpen(true)}
                data-testid="saml-configure"
              >
                {load.kind === "ready" && load.configured ? "Edit SAML configuration" : "Configure SAML"}
              </button>
              <a
                className="security-btn"
                href={samlMetadataUrl()}
                download="skycot-sp-metadata.xml"
                data-testid="saml-metadata"
              >
                <Download size={14} aria-hidden /> Download SP metadata XML
              </a>
            </div>
          </section>

          <SecurityCard
            testId="twofa-card"
            icon={ShieldCheck}
            title="Two-factor authentication"
            blurb="Require time-based one-time codes (TOTP) for every teammate sign-in. Backup codes generated per user."
            statusLabel="Available in v1.1"
            statusTone="warn"
            rows={[
              { label: "Method", value: "TOTP (RFC 6238)" },
              { label: "Enrollment", value: "Per-user, self-serve" },
              { label: "Backup codes", value: "10 single-use codes" },
            ]}
            cta={{
              label: "Configure 2FA",
              disabled: true,
              tooltip: "Per-user 2FA enrollment ships in v1.1. Contact sales for early access.",
            }}
          />

          <SecurityCard
            testId="ip-allowlist-card"
            icon={Network}
            title="IP allowlist"
            blurb="Restrict workspace sign-in to specific IP ranges (CIDR). Owners on the bypass list can still recover during outages."
            statusLabel="Available in v1.1"
            statusTone="warn"
            rows={[
              { label: "Entries", value: "0 ranges configured" },
              { label: "Owner bypass", value: "Enabled by default" },
            ]}
            cta={{
              label: "Add IP range",
              disabled: true,
              tooltip: "IP allowlisting ships in v1.1. Contact sales for early access.",
            }}
          />

          <SecurityCard
            testId="sessions-card"
            icon={Monitor}
            title="Active sessions"
            blurb="Devices currently signed in to this workspace. Revoke a session to force re-authentication on that device."
            statusLabel="1 session"
            statusTone="on"
            span="two"
          >
            <table className="security-sessions-table" data-testid="sessions-table">
              <thead>
                <tr>
                  <th>Device</th>
                  <th>Last active</th>
                  <th>IP</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                <tr data-testid="sessions-current-row">
                  <td>This browser (current session)</td>
                  <td>Just now</td>
                  <td>—</td>
                  <td>Active</td>
                </tr>
              </tbody>
            </table>
            <p className="security-sessions-empty" data-testid="sessions-empty">
              No additional sessions. Multi-device session revocation ships in v1.1.
            </p>
          </SecurityCard>

          <SecurityCard
            testId="audit-retention-card"
            icon={Archive}
            title="Audit retention"
            blurb="How long workspace audit rows are retained. Enterprise tenants can extend retention or stream to long-term storage."
            statusLabel="Indefinite"
            statusTone="on"
            rows={[
              { label: "Retention", value: "All audit rows retained indefinitely" },
              { label: "Export", value: "Available via /settings/data-export" },
            ]}
            cta={{
              label: "Configure retention",
              disabled: true,
              tooltip: "Configurable retention policies are an enterprise feature. Contact sales.",
            }}
            span="two"
          />
        </div>
      </div>

      {modalOpen ? (
        <SamlConfigModal
          initial={(load.kind === "ready" && load.config) || EMPTY_CONFIG}
          onCancel={() => setModalOpen(false)}
          onSaved={onConfigSaved}
        />
      ) : null}
    </>
  );
}

function statusLabelFor(load: LoadState): { text: string; on: boolean } {
  if (load.kind !== "ready" || !load.configured || !load.config) {
    return { text: "Not configured", on: false };
  }
  switch (load.config.flavor) {
    case "okta":
      return { text: "Configured: Okta", on: true };
    case "azure":
      return { text: "Configured: Azure AD", on: true };
    case "custom":
      return { text: "Configured: Custom", on: true };
  }
}

function ConfigSummary({ config }: { config: SamlConfig }) {
  return (
    <dl className="security-summary" data-testid="saml-summary">
      <dt>IdP entity ID</dt>
      <dd>{config.idpEntityId}</dd>
      <dt>SSO URL</dt>
      <dd>{config.ssoUrl}</dd>
      <dt>Certificate</dt>
      <dd>{certificateFingerprint(config.certificate)}</dd>
      <dt>Owner bypass</dt>
      <dd>{config.allowOwnerBypass ? "Allowed (magic-link fallback for owners)" : "Disabled"}</dd>
    </dl>
  );
}

function certificateFingerprint(pem: string): string {
  const trimmed = pem.replace(/-----[^-]+-----/g, "").replace(/\s+/g, "");
  if (trimmed.length === 0) return "—";
  if (trimmed.length <= 16) return trimmed;
  return `${trimmed.slice(0, 8)}…${trimmed.slice(-8)} (${trimmed.length} chars)`;
}
