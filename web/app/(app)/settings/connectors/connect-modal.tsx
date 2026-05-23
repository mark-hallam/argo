"use client";

import { useEffect, useId, useRef, useState } from "react";
import { ExternalLink, KeyRound, Loader2, X } from "lucide-react";
import type { ConnectorEntry } from "./connector-catalog";
import {
  buildTenantConnectUrl,
  saveTenantConnectorCredential,
} from "@/lib/api/connectors";

interface ConnectModalProps {
  connector: ConnectorEntry;
  onClose: () => void;
  onConnected: (connectorId: string) => void;
}

/**
 * Modal driving a single connector's connect flow.
 *
 * OAuth shape — opens the gateway's `/connect/<id>` route in a new
 * tab (popup-blocker friendly with the explicit Connect button) and
 * the user finishes the authorization there. Status refreshes on
 * focus return.
 *
 * API-key shape — renders one labeled input per credential field
 * advertised by `apiKeyFieldsFor()` below, POSTs the bundle to the
 * gateway, and resolves the parent's optimistic state on success.
 */
export function ConnectModal({ connector, onClose, onConnected }: ConnectModalProps) {
  const titleId = useId();
  const isOAuth = connector.authType === "oauth";
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="connector-modal-backdrop"
      role="presentation"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="connector-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-testid="connector-modal"
        data-connector-id={connector.id}
      >
        <button
          type="button"
          className="connector-modal-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="connector-modal-close"
        >
          <X size={16} />
        </button>

        <h2 id={titleId} className="connector-modal-title">
          Connect {connector.name}
        </h2>
        <p className="connector-modal-sub">{connector.description}</p>

        {isOAuth ? (
          <OAuthBody connector={connector} onClose={onClose} />
        ) : (
          <ApiKeyBody connector={connector} onClose={onClose} onConnected={onConnected} />
        )}
      </div>
    </div>
  );
}

function OAuthBody({
  connector,
  onClose,
}: {
  connector: ConnectorEntry;
  onClose: () => void;
}) {
  const url =
    typeof window === "undefined"
      ? "#"
      : buildTenantConnectUrl({
          connectorId: connector.id,
          returnOrigin: window.location.origin,
        });
  return (
    <>
      <div className="connector-modal-body">
        <p>
          We&rsquo;ll open {connector.name}&rsquo;s authorization page in a
          new tab. Sign in there, approve the SKYCOT scopes, and the
          connector flips to <strong>Connected</strong> once you&rsquo;re back.
        </p>
        <ul className="connector-modal-bullets">
          <li>Your credentials never leave {connector.name}.</li>
          <li>SKYCOT stores the access token, encrypted-at-rest.</li>
          <li>You can revoke any time from this page.</li>
        </ul>
      </div>
      <div className="connector-modal-actions">
        <button type="button" className="connector-btn connector-btn-ghost" onClick={onClose}>
          Cancel
        </button>
        <a
          href={url}
          target="_blank"
          rel="noopener noreferrer"
          className="connector-btn connector-btn-primary"
          data-testid="connector-modal-oauth-start"
        >
          <ExternalLink size={13} /> Continue to {connector.name}
        </a>
      </div>
    </>
  );
}

interface ApiKeyField {
  key: string;
  label: string;
  placeholder?: string;
  type?: "text" | "password";
  required?: boolean;
}

/** Per-connector credential shape. Default = single `apiKey` field. */
function apiKeyFieldsFor(connectorId: string): readonly ApiKeyField[] {
  switch (connectorId) {
    case "notion":
      return [
        {
          key: "integration_token",
          label: "Internal integration token",
          placeholder: "secret_…",
          type: "password",
          required: true,
        },
      ];
    case "stripe":
    case "stripe-events":
      return [
        {
          key: "secret_key",
          label: "Stripe secret key",
          placeholder: "sk_live_… or sk_test_…",
          type: "password",
          required: true,
        },
      ];
    case "airtable":
      return [
        {
          key: "personal_access_token",
          label: "Personal access token",
          type: "password",
          required: true,
        },
        { key: "base_id", label: "Default base id (optional)", required: false },
      ];
    case "zendesk":
      return [
        { key: "subdomain", label: "Subdomain (e.g. acme)", required: true },
        { key: "email", label: "Agent email", required: true },
        { key: "api_token", label: "API token", type: "password", required: true },
      ];
    case "imessage":
      return [
        {
          key: "bluebubbles_url",
          label: "Blue Bubbles server URL",
          placeholder: "https://mac-mini.local:1234",
          required: true,
        },
        {
          key: "password",
          label: "Server password",
          type: "password",
          required: true,
        },
      ];
    default:
      return [
        {
          key: "api_key",
          label: "API key",
          type: "password",
          required: true,
        },
      ];
  }
}

function ApiKeyBody({
  connector,
  onClose,
  onConnected,
}: {
  connector: ConnectorEntry;
  onClose: () => void;
  onConnected: (connectorId: string) => void;
}) {
  const fields = apiKeyFieldsFor(connector.id);
  const [values, setValues] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canSubmit =
    !submitting &&
    fields.every((f) => !f.required || (values[f.key]?.trim().length ?? 0) > 0);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    try {
      await saveTenantConnectorCredential({
        connectorId: connector.id,
        fields: values,
      });
      onConnected(connector.id);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save credential.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form onSubmit={onSubmit} className="connector-modal-form" data-testid="connector-modal-apikey-form">
      <div className="connector-modal-body">
        <p className="connector-modal-apikey-blurb">
          <KeyRound size={13} /> Paste the credential from {connector.name}.
          SKYCOT stores it encrypted-at-rest.
        </p>
        {fields.map((f) => (
          <label key={f.key} className="connector-modal-label">
            <span>
              {f.label}
              {f.required ? <em className="connector-modal-required">required</em> : null}
            </span>
            <input
              type={f.type ?? "text"}
              className="connector-modal-input"
              placeholder={f.placeholder}
              value={values[f.key] ?? ""}
              onChange={(e) => setValues((v) => ({ ...v, [f.key]: e.target.value }))}
              autoComplete="off"
              spellCheck={false}
              required={f.required}
              data-testid={`connector-modal-field-${f.key}`}
            />
          </label>
        ))}
        {error ? <div className="connector-modal-error">{error}</div> : null}
      </div>
      <div className="connector-modal-actions">
        <button type="button" className="connector-btn connector-btn-ghost" onClick={onClose}>
          Cancel
        </button>
        <button
          type="submit"
          className="connector-btn connector-btn-primary"
          disabled={!canSubmit}
          data-testid="connector-modal-apikey-submit"
        >
          {submitting ? <Loader2 size={13} className="connector-card-state-spin" /> : <KeyRound size={13} />}
          {submitting ? "Saving…" : `Save & connect`}
        </button>
      </div>
    </form>
  );
}
