"use client";

import { useState } from "react";
import {
  saveProviderKey,
  type ProviderKeyStatus,
} from "@/lib/api/provider-keys";
import type { ProviderName } from "@/lib/api/settings";

export interface ProviderKeyFormProps {
  provider: ProviderName;
  label: string;
  status: ProviderKeyStatus;
  onSaved: (next: ProviderKeyStatus) => void;
  onError?: (message: string) => void;
  unavailableReason?: string;
}

export function ProviderKeyForm({
  provider,
  label,
  status,
  onSaved,
  onError,
  unavailableReason,
}: ProviderKeyFormProps) {
  const [expanded, setExpanded] = useState(false);
  const [draft, setDraft] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const testid = `provider-key-${provider}`;

  function cancel() {
    setExpanded(false);
    setDraft("");
  }

  async function submit() {
    const trimmed = draft.trim();
    if (!trimmed) return;
    setSubmitting(true);
    try {
      const { status: next } = await saveProviderKey({
        provider,
        key: trimmed,
      });
      onSaved(next);
      setExpanded(false);
      setDraft("");
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Couldn't save provider key";
      onError?.(message);
    } finally {
      setSubmitting(false);
    }
  }

  const isUnavailable = Boolean(unavailableReason);
  const buttonLabel = isUnavailable
    ? "Unavailable"
    : status.configured
      ? "Rotate"
      : "Add key";
  const statusLabel = isUnavailable
    ? "Endpoint pending"
    : status.configured
      ? "Configured"
      : "Mock provider";

  return (
    <div className="provider-key-row" data-testid={testid}>
      <div className="provider-key-meta">
        <span className="provider-key-label">{label}</span>
        <span
          className={`provider-key-pill ${
            isUnavailable
              ? "provider-key-pill-unavailable"
              : status.configured
                ? "provider-key-pill-configured"
                : "provider-key-pill-mock"
          }`}
          data-testid={`${testid}-status`}
        >
          {statusLabel}
        </span>
        {status.masked_preview ? (
          <code
            className="provider-key-preview"
            data-testid={`${testid}-preview`}
          >
            {status.masked_preview}
          </code>
        ) : null}
      </div>
      {expanded ? (
        <form
          className="provider-key-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <input
            type="password"
            className="provider-key-input"
            placeholder={`Paste your ${label} API key`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            disabled={submitting}
            autoFocus
            aria-label={`${label} API key`}
            data-testid={`${testid}-input`}
          />
          <button
            type="submit"
            className="provider-key-save"
            disabled={submitting || !draft.trim()}
            data-testid={`${testid}-save`}
          >
            {submitting ? "Saving…" : "Save"}
          </button>
          <button
            type="button"
            className="provider-key-cancel"
            onClick={cancel}
            disabled={submitting}
            data-testid={`${testid}-cancel`}
          >
            Cancel
          </button>
        </form>
      ) : (
        <button
          type="button"
          className="provider-key-action"
          onClick={() => setExpanded(true)}
          disabled={isUnavailable}
          title={unavailableReason}
          data-testid={`${testid}-action`}
        >
          {buttonLabel}
        </button>
      )}
    </div>
  );
}
