"use client";

import { useRef, useState } from "react";
import { Loader2, Save, X } from "lucide-react";
import { ingestOnboardingDocuments } from "@/lib/api/genesis-ingest";
import { useFocusTrap } from "@/lib/a11y/use-focus-trap";

/**
 * Inline memory-edit modal for the quiz refinement loop (Phase 2; memory
 * `genesis-is-ai-teammate-onboarding-studio`). When a quiz answer reveals a
 * wrong assumption, the operator types the correction here ("Actually, refunds
 * over $500 require legal sign-off, not just manager approval") and saves it.
 *
 * No theater / no parallel write path: the correction is persisted by
 * re-uploading it as a corrected onboarding document through the SAME
 * `POST /api/v1/genesis/ingest-document` endpoint the uploader uses, so the next
 * quiz answer retrieves over the corrected text. Reusing the document label
 * overwrites the prior entry within the session.
 */

export interface MemoryEditModalProps {
  verticalId: string;
  onClose(): void;
  /** Fired after a successful save so the parent can re-quiz / refresh. */
  onSaved(): void;
}

const DEFAULT_LABEL = "memory-correction.md";

export function MemoryEditModal({ verticalId, onClose, onSaved }: MemoryEditModalProps) {
  const [label, setLabel] = useState(DEFAULT_LABEL);
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onClose);

  const save = async () => {
    const trimmed = content.trim();
    if (trimmed.length < 80) {
      setError("Add a bit more detail — corrections need at least 80 characters to ingest.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const resp = await ingestOnboardingDocuments(verticalId, [
        { filename: label.trim() || DEFAULT_LABEL, format: "text", content: trimmed },
      ]);
      if (resp.ingested < 1) {
        setError(resp.results[0]?.reason ?? "Couldn't save that correction.");
        setSaving(false);
        return;
      }
      onSaved();
    } catch {
      setError("Couldn't save the correction. Try again.");
      setSaving(false);
    }
  };

  return (
    <>
      <style jsx>{`
        .gen-mem-overlay {
          position: fixed;
          inset: 0;
          background: rgba(30, 27, 75, 0.42);
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 24px;
          z-index: 60;
        }
        .gen-mem-modal {
          background: white;
          border-radius: 20px;
          box-shadow: 0 30px 70px -20px rgba(30, 27, 75, 0.45);
          width: 100%;
          max-width: 560px;
          padding: 28px;
        }
        .gen-mem-head {
          display: flex;
          align-items: flex-start;
          justify-content: space-between;
          gap: 16px;
        }
        .gen-mem-title {
          font-family: var(--font-audiowide), "Inter", sans-serif;
          font-size: 19px;
          font-weight: 400;
          color: #1e1b4b;
          margin: 0;
        }
        .gen-mem-sub {
          margin-top: 6px;
          font-size: 13px;
          color: #52525b;
          line-height: 1.5;
        }
        .gen-mem-close {
          background: none;
          border: none;
          cursor: pointer;
          color: #52525b;
          padding: 4px;
        }
        .gen-mem-field {
          margin-top: 18px;
        }
        .gen-mem-field label {
          display: block;
          font-family: var(--font-mono);
          font-size: 10.5px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
          color: #6d28d9;
          margin-bottom: 6px;
        }
        .gen-mem-input,
        .gen-mem-textarea {
          width: 100%;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          color: #1e1b4b;
          border: 1px solid #ddd6f3;
          border-radius: 11px;
          padding: 11px 13px;
          outline: none;
          box-sizing: border-box;
        }
        .gen-mem-input:focus,
        .gen-mem-textarea:focus {
          border-color: #8b5cf6;
        }
        .gen-mem-textarea {
          min-height: 140px;
          resize: vertical;
          line-height: 1.5;
        }
        .gen-mem-error {
          margin-top: 10px;
          font-size: 13px;
          color: #b91c1c;
        }
        .gen-mem-actions {
          margin-top: 22px;
          display: flex;
          gap: 12px;
          justify-content: flex-end;
        }
        .gen-mem-cancel {
          min-height: 44px;
          padding: 0 18px;
          font-size: 14px;
          font-weight: 600;
          color: #52525b;
          background: white;
          border: 1px solid #e4e4e7;
          border-radius: 12px;
          cursor: pointer;
        }
        .gen-mem-save {
          min-height: 44px;
          padding: 0 22px;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          font-weight: 700;
          color: white;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          border: none;
          border-radius: 12px;
          cursor: pointer;
        }
        .gen-mem-save:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
      `}</style>
      <div
        className="gen-mem-overlay"
        role="presentation"
        onClick={onClose}
        data-testid="genesis-memory-edit-overlay"
      >
        <div
          ref={dialogRef}
          className="gen-mem-modal"
          role="dialog"
          aria-modal="true"
          aria-label="Edit teammate memory"
          tabIndex={-1}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="gen-mem-head">
            <div>
              <h3 className="gen-mem-title">Fix a memory</h3>
              <p className="gen-mem-sub">
                Correct a wrong assumption. This is saved into the teammate&apos;s
                memory and used the next time you ask — no fabrication, just the
                text you write here.
              </p>
            </div>
            <button
              type="button"
              className="gen-mem-close"
              onClick={onClose}
              aria-label="Close"
            >
              <X size={18} aria-hidden />
            </button>
          </div>

          <div className="gen-mem-field">
            <label htmlFor="gen-mem-label">Memory label</label>
            <input
              id="gen-mem-label"
              className="gen-mem-input"
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              data-testid="genesis-memory-edit-label"
            />
          </div>

          <div className="gen-mem-field">
            <label htmlFor="gen-mem-content">Correction</label>
            <textarea
              id="gen-mem-content"
              className="gen-mem-textarea"
              placeholder="Actually, refunds over $500 require legal sign-off, not just manager approval. Route them to the legal queue before issuing."
              value={content}
              onChange={(e) => setContent(e.target.value)}
              data-testid="genesis-memory-edit-content"
            />
          </div>

          {error && <p className="gen-mem-error">{error}</p>}

          <div className="gen-mem-actions">
            <button type="button" className="gen-mem-cancel" onClick={onClose}>
              Cancel
            </button>
            <button
              type="button"
              className="gen-mem-save"
              onClick={save}
              disabled={saving || !content.trim()}
              data-testid="genesis-memory-edit-save"
            >
              {saving ? (
                <Loader2 size={15} className="gen-spin" aria-hidden />
              ) : (
                <Save size={15} aria-hidden />
              )}
              Save & re-quiz
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
