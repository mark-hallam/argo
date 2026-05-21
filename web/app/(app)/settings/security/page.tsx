"use client";

import { useEffect, useMemo, useState } from "react";
import { CheckCircle2, AlertCircle, Lock, Download, Loader2 } from "lucide-react";
import {
  getSamlConfig,
  samlMetadataUrl,
  saveSamlConfig,
  testSamlConnection,
  type SamlConfig,
  type SamlTestResult,
} from "@/lib/api/sso";

/**
 * Security settings — `/settings/security`.
 *
 * Currently surfaces SAML 2.0 SSO configuration. Future sub-tabs
 * (sessions, audit retention, encryption-at-rest) plug in alongside.
 *
 * Brand-restrained register: Inter throughout, gradient sparingly,
 * one editorial card per concern.
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
  | { kind: "ready"; configured: boolean; config: SamlConfig | null }
  | { kind: "error"; message: string };

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
      .catch((err: unknown) => {
        if (cancelled) return;
        const message = err instanceof Error ? err.message : "Couldn't load SSO config.";
        setLoad({ kind: "error", message });
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
        .security-stage { padding: 48px 32px 96px; max-width: 980px; margin: 0 auto; }
        .security-eyebrow { font-family: "JetBrains Mono", monospace; font-size: 11px;
          color: #52525b; letter-spacing: 0.16em; text-transform: uppercase;
          font-weight: 700; margin-bottom: 8px; }
        .security-title { font-family: "Inter", sans-serif; font-size: 32px;
          font-weight: 600; color: #1e1b4b; margin: 0 0 6px; letter-spacing: -0.01em; }
        .security-sub { font-size: 15px; color: #3f3f46; margin: 0 0 36px; }
        .security-card { background: white; border: 1px solid #ece9f5; border-radius: 14px;
          padding: 24px; display: flex; flex-direction: column; gap: 14px; }
        .security-card-head { display: flex; align-items: flex-start;
          justify-content: space-between; gap: 16px; }
        .security-card-title { font-size: 18px; font-weight: 600; color: #1e1b4b;
          margin: 0; display: flex; align-items: center; gap: 10px; }
        .security-card-blurb { font-size: 14px; color: #3f3f46; margin: 0; }
        .security-status-pill { display: inline-flex; align-items: center; gap: 6px;
          padding: 4px 10px; border-radius: 999px; font-size: 12px; font-weight: 600;
          border: 1px solid #ece9f5; background: #fafaff; color: #1e1b4b;
          font-family: "JetBrains Mono", monospace; letter-spacing: 0.04em; }
        .security-status-on { color: #047857; border-color: #a7f3d0; background: #ecfdf5; }
        .security-actions { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
        .security-btn { padding: 9px 16px; border-radius: 10px; font-size: 13.5px;
          font-weight: 600; cursor: pointer; border: 1px solid #ece9f5;
          background: white; color: #1e1b4b; display: inline-flex; align-items: center;
          gap: 8px; }
        .security-btn:hover { border-color: #8b5cf6; }
        .security-btn-primary { background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white; border: none; box-shadow: 0 4px 12px -4px rgba(139, 92, 246, 0.45); }
        .security-btn-primary:hover { filter: brightness(1.05); border: none; }
        .security-toggle { display: flex; align-items: center; gap: 12px;
          padding: 14px; border-radius: 10px; border: 1px solid #ece9f5;
          background: #fafaff; }
        .security-toggle-text { flex: 1; }
        .security-toggle-label { font-size: 14px; font-weight: 600; color: #1e1b4b;
          margin: 0 0 2px; }
        .security-toggle-blurb { font-size: 12.5px; color: #52525b; margin: 0; }
        .security-summary { display: grid; grid-template-columns: 1fr 2fr; gap: 8px 16px;
          padding: 12px; border-radius: 10px; background: #fafaff;
          font-size: 13px; color: #3f3f46; }
        .security-summary dt { color: #52525b; font-weight: 600;
          font-family: "JetBrains Mono", monospace; font-size: 11px;
          letter-spacing: 0.06em; text-transform: uppercase; }
        .security-summary dd { margin: 0; word-break: break-all; }
        .security-error { color: #b91c1c; font-size: 13px; margin: 0; }
        .security-load { color: #52525b; font-size: 14px;
          display: inline-flex; align-items: center; gap: 8px; }
      `}</style>
      <div className="security-stage">
        <header>
          <div className="security-eyebrow">⚙ Workspace · Security</div>
          <h1 className="security-title">Security</h1>
          <p className="security-sub">
            Single sign-on, audit retention, and the rest of your workspace&apos;s
            security posture.
          </p>
        </header>

        <section className="security-card" data-testid="saml-card">
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

          {load.kind === "error" ? (
            <p className="security-error" role="alert" data-testid="saml-error">
              {load.message}
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

interface ModalProps {
  initial: SamlConfig;
  onCancel: () => void;
  onSaved: (saved: SamlConfig) => void;
}

function SamlConfigModal({ initial, onCancel, onSaved }: ModalProps) {
  const [draft, setDraft] = useState<SamlConfig>(initial);
  const [busy, setBusy] = useState<"idle" | "saving" | "testing">("idle");
  const [testResult, setTestResult] = useState<SamlTestResult | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  function update<K extends keyof SamlConfig>(key: K, value: SamlConfig[K]) {
    setDraft((d) => ({ ...d, [key]: value }));
    setTestResult(null);
  }

  async function onTest() {
    setBusy("testing");
    setTestResult(null);
    try {
      const result = await testSamlConnection(draft);
      setTestResult(result);
    } catch (err) {
      setTestResult({
        ok: false,
        message: err instanceof Error ? err.message : "Test failed.",
      });
    } finally {
      setBusy("idle");
    }
  }

  async function onSave() {
    setBusy("saving");
    setSaveError(null);
    try {
      const saved = await saveSamlConfig(draft);
      onSaved(saved);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Save failed.");
      setBusy("idle");
    }
  }

  const disabled = busy !== "idle";

  return (
    <div
      className="saml-modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="saml-modal-title"
      data-testid="saml-modal"
    >
      <style jsx>{`
        .saml-modal-backdrop {
          position: fixed; inset: 0; z-index: 50;
          background: rgba(30, 27, 75, 0.4); backdrop-filter: blur(4px);
          display: grid; place-items: center; padding: 24px;
        }
        .saml-modal {
          background: white; border-radius: 16px; max-width: 640px; width: 100%;
          max-height: 90vh; overflow-y: auto; padding: 28px;
          box-shadow: 0 25px 50px -12px rgba(30, 27, 75, 0.35);
          display: flex; flex-direction: column; gap: 16px;
        }
        .saml-modal h2 { margin: 0; font-size: 20px; font-weight: 600; color: #1e1b4b; }
        .saml-row { display: flex; flex-direction: column; gap: 6px; }
        .saml-label { font-family: "JetBrains Mono", monospace; font-size: 11px;
          letter-spacing: 0.12em; text-transform: uppercase; color: #52525b;
          font-weight: 700; }
        .saml-input, .saml-textarea, .saml-select {
          padding: 10px 14px; border-radius: 9px; border: 1px solid #ece9f5;
          background: #fafaff; font-family: "Inter", sans-serif; font-size: 14px;
          color: #1e1b4b; outline: none;
        }
        .saml-input:focus, .saml-textarea:focus, .saml-select:focus {
          border-color: #8b5cf6; box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.12);
        }
        .saml-textarea { min-height: 140px; font-family: "JetBrains Mono", monospace;
          font-size: 12.5px; line-height: 1.45; }
        .saml-toggle { display: flex; align-items: center; gap: 12px;
          padding: 12px; border-radius: 9px; background: #fafaff;
          border: 1px solid #ece9f5; }
        .saml-toggle-text { flex: 1; }
        .saml-toggle-label { font-size: 13.5px; font-weight: 600; color: #1e1b4b;
          margin: 0 0 2px; }
        .saml-toggle-blurb { font-size: 12px; color: #52525b; margin: 0; }
        .saml-actions { display: flex; gap: 10px; justify-content: flex-end;
          margin-top: 8px; flex-wrap: wrap; }
        .saml-btn { padding: 9px 16px; border-radius: 9px; font-size: 13px;
          font-weight: 600; border: 1px solid #ece9f5; background: white;
          color: #1e1b4b; cursor: pointer; }
        .saml-btn:disabled { opacity: 0.6; cursor: not-allowed; }
        .saml-btn-primary { background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white; border: none; box-shadow: 0 4px 12px -4px rgba(139, 92, 246, 0.45); }
        .saml-result { padding: 10px 14px; border-radius: 9px; font-size: 13px;
          font-weight: 500; display: flex; gap: 8px; align-items: center; }
        .saml-result-ok { background: #ecfdf5; color: #047857; border: 1px solid #a7f3d0; }
        .saml-result-err { background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; }
      `}</style>
      <div className="saml-modal">
        <h2 id="saml-modal-title">Configure SAML SSO</h2>

        <div className="saml-row">
          <label className="saml-label" htmlFor="saml-flavor">Identity provider</label>
          <select
            id="saml-flavor"
            className="saml-select"
            value={draft.flavor}
            onChange={(e) => update("flavor", e.target.value as SamlConfig["flavor"])}
            disabled={disabled}
          >
            <option value="okta">Okta</option>
            <option value="azure">Azure AD</option>
            <option value="custom">Custom SAML 2.0 IdP</option>
          </select>
        </div>

        <div className="saml-row">
          <label className="saml-label" htmlFor="saml-entity">IdP entity ID</label>
          <input
            id="saml-entity"
            className="saml-input"
            value={draft.idpEntityId}
            onChange={(e) => update("idpEntityId", e.target.value)}
            placeholder="http://www.okta.com/abc123"
            disabled={disabled}
            data-testid="saml-input-entity"
          />
        </div>

        <div className="saml-row">
          <label className="saml-label" htmlFor="saml-sso">SSO URL</label>
          <input
            id="saml-sso"
            className="saml-input"
            value={draft.ssoUrl}
            onChange={(e) => update("ssoUrl", e.target.value)}
            placeholder="https://example.okta.com/app/.../sso/saml"
            disabled={disabled}
            data-testid="saml-input-sso"
          />
        </div>

        <div className="saml-row">
          <label className="saml-label" htmlFor="saml-cert">X.509 certificate (PEM)</label>
          <textarea
            id="saml-cert"
            className="saml-textarea"
            value={draft.certificate}
            onChange={(e) => update("certificate", e.target.value)}
            placeholder={"-----BEGIN CERTIFICATE-----\nMIID...\n-----END CERTIFICATE-----"}
            disabled={disabled}
            data-testid="saml-input-cert"
          />
        </div>

        <div className="saml-row">
          <label className="saml-label" htmlFor="saml-attr">
            Attribute mapping (optional JSON)
          </label>
          <textarea
            id="saml-attr"
            className="saml-textarea"
            style={{ minHeight: 80 }}
            value={draft.attributeMappingJson ?? ""}
            onChange={(e) => update("attributeMappingJson", e.target.value)}
            placeholder='{"role": "https://schemas.skycot/role", "cohort": "department"}'
            disabled={disabled}
          />
        </div>

        <div className="saml-toggle">
          <div className="saml-toggle-text">
            <p className="saml-toggle-label">Allow non-SSO login for owners</p>
            <p className="saml-toggle-blurb">
              Owners can still sign in with magic-link during IdP outages.
              Recommended on for break-glass access.
            </p>
          </div>
          <input
            type="checkbox"
            checked={draft.allowOwnerBypass}
            onChange={(e) => update("allowOwnerBypass", e.target.checked)}
            disabled={disabled}
            data-testid="saml-input-bypass"
            aria-label="Allow non-SSO login for owners"
          />
        </div>

        {testResult ? (
          <div className={testResult.ok ? "saml-result saml-result-ok" : "saml-result saml-result-err"}>
            {testResult.ok ? <CheckCircle2 size={14} /> : <AlertCircle size={14} />}
            {testResult.message}
          </div>
        ) : null}

        {saveError ? (
          <p className="security-error" role="alert" data-testid="saml-save-error">
            {saveError}
          </p>
        ) : null}

        <div className="saml-actions">
          <button
            type="button"
            className="saml-btn"
            onClick={onCancel}
            disabled={disabled}
            data-testid="saml-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            className="saml-btn"
            onClick={onTest}
            disabled={disabled || draft.idpEntityId.length === 0 || draft.ssoUrl.length === 0}
            data-testid="saml-test"
          >
            {busy === "testing" ? "Testing…" : "Test connection"}
          </button>
          <button
            type="button"
            className="saml-btn saml-btn-primary"
            onClick={onSave}
            disabled={
              disabled ||
              draft.idpEntityId.length === 0 ||
              draft.ssoUrl.length === 0 ||
              draft.certificate.length === 0
            }
            data-testid="saml-save"
          >
            {busy === "saving" ? "Saving…" : "Save configuration"}
          </button>
        </div>
      </div>
    </div>
  );
}
