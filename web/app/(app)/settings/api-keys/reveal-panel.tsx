"use client";

import { useState } from "react";
import { CircleCheck, Copy, ShieldAlert } from "lucide-react";
import type { ApiKeyRow, CreateApiKeyResponse } from "@/lib/api/api-keys";

/** Reveal-once panel + confirm dialog for revoke. */

export interface RevealPanelProps {
  response: CreateApiKeyResponse;
  confirmingClose: boolean;
  onRequestClose: () => void;
  onCancelClose: () => void;
  onConfirmClose: () => void;
}

export function RevealPanel({
  response,
  confirmingClose,
  onRequestClose,
  onCancelClose,
  onConfirmClose,
}: RevealPanelProps) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      if (navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
        await navigator.clipboard.writeText(response.token);
      }
    } catch {
      // best-effort; user can still select+copy from the code block
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 2200);
  }

  return (
    <div className="ak-overlay">
      <div className="ak-modal" role="dialog" aria-labelledby="ak-reveal-h2">
        <h2 id="ak-reveal-h2">Your new API key</h2>
        <p className="ak-modal-blurb">
          Save this token in your secret manager. We don&apos;t store the plaintext server-side —
          once you close this dialog you&apos;ll only ever see the masked prefix.
        </p>

        <div className="ak-warning" role="alert">
          <ShieldAlert size={16} />
          <span>Copy this token now. You won&apos;t see it again.</span>
        </div>

        <div className="ak-token-box">
          <code>{response.token}</code>
          <button type="button" className="ak-copy-btn" onClick={copy}>
            {copied ? (
              <>
                <CircleCheck size={14} /> Copied
              </>
            ) : (
              <>
                <Copy size={14} /> Copy
              </>
            )}
          </button>
        </div>

        <p className="ak-modal-meta">
          <strong>{response.key.name}</strong> · {response.key.scopes.length} scope
          {response.key.scopes.length === 1 ? "" : "s"} ·{" "}
          {response.key.all_cohorts ? "all cohorts" : `${response.key.cohort_ids.length} cohorts`}
        </p>

        {confirmingClose ? (
          <div className="ak-confirm-inline">
            <ShieldAlert size={16} />
            <span>Are you sure? You will lose access to this token.</span>
            <div className="ak-confirm-inline-actions">
              <button type="button" className="ak-cancel" onClick={onCancelClose}>
                Keep showing
              </button>
              <button type="button" className="ak-submit ak-submit-danger" onClick={onConfirmClose}>
                Yes, close
              </button>
            </div>
          </div>
        ) : (
          <div className="ak-actions">
            <button type="button" className="ak-cancel" onClick={onRequestClose}>
              Close
            </button>
            <button type="button" className="ak-submit" onClick={onConfirmClose}>
              I&apos;ve saved it — close
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export interface RevokeDialogProps {
  row: ApiKeyRow;
  isPending: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export function RevokeDialog({ row, isPending, onCancel, onConfirm }: RevokeDialogProps) {
  return (
    <div
      className="ak-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div className="ak-modal ak-modal-small" role="dialog" aria-labelledby="ak-revoke-h2">
        <h2 id="ak-revoke-h2">Revoke &ldquo;{row.name}&rdquo;?</h2>
        <p className="ak-modal-blurb">
          Any system using this key will stop working immediately. This action cannot be undone.
        </p>
        <div className="ak-actions">
          <button type="button" className="ak-cancel" onClick={onCancel} disabled={isPending}>
            Cancel
          </button>
          <button
            type="button"
            className="ak-submit ak-submit-danger"
            onClick={onConfirm}
            disabled={isPending}
          >
            {isPending ? "Revoking…" : "Revoke key"}
          </button>
        </div>
      </div>
    </div>
  );
}
