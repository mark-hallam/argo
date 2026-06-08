"use client";

import { useRef, useState } from "react";
import { CheckCircle2, AlertCircle } from "lucide-react";
import {
  saveSamlConfig,
  testSamlConnection,
  type SamlConfig,
  type SamlTestResult,
} from "@/lib/api/sso";
import { useFocusTrap } from "@/lib/a11y/use-focus-trap";

interface ModalProps {
  initial: SamlConfig;
  onCancel: () => void;
  onSaved: (saved: SamlConfig) => void;
}

export default function SamlConfigModal({ initial, onCancel, onSaved }: ModalProps) {
  const [draft, setDraft] = useState<SamlConfig>(initial);
  const [busy, setBusy] = useState<"idle" | "saving" | "testing">("idle");
  const [testResult, setTestResult] = useState<SamlTestResult | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

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
      // Persist trimmed identifiers so stray whitespace never reaches the
      // IdP comparison; the attribute mapping is validated below before
      // Save is enabled, so it is safe to send as typed.
      const saved = await saveSamlConfig({
        ...draft,
        idpEntityId: draft.idpEntityId.trim(),
        ssoUrl: draft.ssoUrl.trim(),
      });
      onSaved(saved);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : "Save failed.");
      setBusy("idle");
    }
  }

  const disabled = busy !== "idle";

  // The optional attribute mapping must be valid JSON (an object) before
  // Save is enabled; empty / whitespace counts as "no mapping" and is OK.
  const attrRaw = (draft.attributeMappingJson ?? "").trim();
  const attributeMappingError = (() => {
    if (attrRaw === "") return null;
    try {
      const parsed = JSON.parse(attrRaw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
        return "Attribute mapping must be a JSON object.";
      }
      return null;
    } catch {
      return "Attribute mapping is not valid JSON.";
    }
  })();

  const entityFilled = draft.idpEntityId.trim().length > 0;
  const ssoFilled = draft.ssoUrl.trim().length > 0;

  useFocusTrap(dialogRef, onCancel, { active: !disabled });

  return (
    <div className="saml-modal-backdrop">
      <style jsx>{`
        .saml-modal-backdrop {
          position: fixed; inset: 0; z-index: 50;
          background: color-mix(in oklab, var(--foreground) 40%, transparent);
          backdrop-filter: blur(4px);
          display: grid; place-items: center; padding: 24px;
        }
        .saml-modal {
          background: var(--popover); color: var(--popover-foreground);
          border-radius: var(--radius-lg); max-width: 640px; width: 100%;
          max-height: 90vh; overflow-y: auto; padding: 28px;
          box-shadow: 0 25px 50px -12px color-mix(in oklab, var(--foreground) 35%, transparent);
          display: flex; flex-direction: column; gap: 16px;
          outline: none;
        }
        .saml-modal:focus-visible {
          box-shadow:
            0 25px 50px -12px color-mix(in oklab, var(--foreground) 35%, transparent),
            0 0 0 3px color-mix(in oklab, var(--ring) 28%, transparent);
        }
        .saml-modal h2 { margin: 0; font-size: 20px; font-weight: 600; color: var(--foreground); }
        .saml-sr-only {
          position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
          overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
        }
        .saml-row { display: flex; flex-direction: column; gap: 6px; }
        .saml-label { font-family: var(--font-mono); font-size: 11px;
          letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted-foreground);
          font-weight: 700; }
        .saml-input, .saml-textarea, .saml-select {
          padding: 10px 14px; border-radius: var(--radius-md); border: 1px solid var(--input);
          background: var(--background); font-family: var(--font-sans); font-size: 14px;
          color: var(--foreground); outline: none;
        }
        .saml-input:focus, .saml-textarea:focus, .saml-select:focus {
          border-color: var(--ring); box-shadow: 0 0 0 3px color-mix(in oklab, var(--ring) 16%, transparent);
        }
        .saml-textarea { min-height: 140px; font-family: var(--font-mono);
          font-size: 12.5px; line-height: 1.45; }
        .saml-toggle { display: flex; align-items: center; gap: 12px;
          padding: 12px; border-radius: var(--radius-md); background: var(--background);
          border: 1px solid var(--border); }
        .saml-toggle-text { flex: 1; }
        .saml-toggle-label { font-size: 13.5px; font-weight: 600; color: var(--foreground);
          margin: 0 0 2px; }
        .saml-toggle-blurb { font-size: 12px; color: var(--muted-foreground); margin: 0; }
        .saml-actions { display: flex; gap: 10px; justify-content: flex-end;
          margin-top: 8px; flex-wrap: wrap; }
        .saml-btn { padding: 9px 16px; border-radius: var(--radius-md); font-size: 13px;
          font-weight: 600; border: 1px solid var(--border); background: var(--card);
          color: var(--foreground); cursor: pointer; }
        .saml-btn:focus-visible {
          outline: 2px solid var(--ring); outline-offset: 2px;
        }
        .saml-btn:disabled { opacity: 0.6; cursor: not-allowed; }
        .saml-btn-primary { background: linear-gradient(135deg, var(--primary), var(--gradient-end));
          color: var(--primary-foreground); border: none;
          box-shadow: 0 4px 12px -4px color-mix(in oklab, var(--primary) 45%, transparent); }
        .saml-result { padding: 10px 14px; border-radius: var(--radius-md); font-size: 13px;
          font-weight: 500; display: flex; gap: 8px; align-items: center; }
        .saml-result-ok {
          background: color-mix(in oklab, var(--success) 12%, var(--card));
          color: var(--success); border: 1px solid color-mix(in oklab, var(--success) 32%, var(--border));
        }
        .saml-result-err {
          background: color-mix(in oklab, var(--destructive) 10%, var(--card));
          color: var(--destructive); border: 1px solid color-mix(in oklab, var(--destructive) 28%, var(--border));
        }
        .saml-save-error { color: var(--destructive); font-size: 13px; margin: 0; }
      `}</style>
      <div
        className="saml-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="saml-modal-title"
        aria-describedby="saml-modal-description"
        data-testid="saml-modal"
        ref={dialogRef}
        tabIndex={-1}
      >
        <h2 id="saml-modal-title">Configure SAML SSO</h2>
        <p id="saml-modal-description" className="saml-sr-only">
          Enter identity provider details, test the connection, and save the SAML configuration.
        </p>

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
            aria-invalid={attributeMappingError !== null}
          />
          {attributeMappingError ? (
            <p className="saml-save-error" role="alert" data-testid="saml-attr-error">
              {attributeMappingError}
            </p>
          ) : null}
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
          <div
            className={testResult.ok ? "saml-result saml-result-ok" : "saml-result saml-result-err"}
            role={testResult.ok ? "status" : "alert"}
          >
            {testResult.ok ? (
              <CheckCircle2 size={14} aria-hidden="true" />
            ) : (
              <AlertCircle size={14} aria-hidden="true" />
            )}
            {testResult.message}
          </div>
        ) : null}

        {saveError ? (
          <p className="saml-save-error" role="alert" data-testid="saml-save-error">
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
            disabled={disabled || !entityFilled || !ssoFilled}
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
              !entityFilled ||
              !ssoFilled ||
              draft.certificate.trim().length === 0 ||
              attributeMappingError !== null
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
