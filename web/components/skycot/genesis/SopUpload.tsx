"use client";

import { useCallback, useRef, useState } from "react";
import { FileText, Paperclip, X } from "lucide-react";

/**
 * Genesis SOP-upload control (Genesis P6 — the moat; DEC-025 #5).
 *
 * Lets the operator attach ONE Standard-Operating-Procedure / playbook
 * document (`.txt` / `.md` read as text, `.pdf` read as base64) that the new
 * teammate must comprehend before it can be hired. The teammate is graded on a
 * doc-grounded comprehension quiz at the hire gate — so this is the real input
 * behind the "checks comprehension before they start work" promise, not
 * theater.
 *
 * Presentational + file-mechanics only: the parent `/genesis` page owns the
 * resulting `SopSelection` and threads it into the SSE request. Consumes ONLY
 * the locked brand tokens from `globals.css` (purple→PINK gradient, Audiowide /
 * Inter, `--brand-ink`) — no color literals, no new design (DEC-027).
 */

/** What the picker yields to the parent for the SSE request. */
export interface SopSelection {
  /** Plain-text body, or a base64-encoded PDF (see `format`). */
  document: string;
  /** How the gateway should read `document`. */
  format: "text" | "pdf_base64";
  /** The chosen file's name (surfaced on the review screen + audit). */
  filename: string;
}

export interface SopUploadProps {
  /** The current selection, or `null` when none attached. */
  value: SopSelection | null;
  /** Replace the selection (a new file), or clear it (`null`). */
  onChange(next: SopSelection | null): void;
  /** Disabled while a generation is in flight. */
  disabled?: boolean;
}

/** Accepted extensions → MIME hints. PDF goes base64; the rest are text. */
const ACCEPT = ".txt,.md,.markdown,.text,.pdf";
/** Cap mirrors the gateway's `SOP_BODY_CAP_BYTES` (5 MB) so we reject locally
 *  with a friendly message before the upload. */
const MAX_BYTES = 5 * 1024 * 1024;

export function SopUpload({ value, onChange, disabled }: SopUploadProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);

  const pick = useCallback(() => {
    if (disabled || reading) return;
    inputRef.current?.click();
  }, [disabled, reading]);

  const onFile = useCallback(
    async (file: File | undefined) => {
      setError(null);
      if (!file) return;
      if (file.size > MAX_BYTES) {
        setError("That document is over 5 MB. Please attach a smaller SOP.");
        return;
      }
      setReading(true);
      try {
        const isPdf =
          file.type === "application/pdf" ||
          file.name.toLowerCase().endsWith(".pdf");
        if (isPdf) {
          const b64 = await readAsBase64(file);
          onChange({ document: b64, format: "pdf_base64", filename: file.name });
        } else {
          const text = await file.text();
          if (text.trim().length === 0) {
            setError("That file looks empty. Please attach a document with text.");
            return;
          }
          onChange({ document: text, format: "text", filename: file.name });
        }
      } catch {
        setError("Couldn't read that file. Please try a .txt, .md, or .pdf.");
      } finally {
        setReading(false);
      }
    },
    [onChange],
  );

  const clear = useCallback(() => {
    setError(null);
    onChange(null);
    if (inputRef.current) inputRef.current.value = "";
  }, [onChange]);

  return (
    <div className="sop-upload" data-testid="genesis-sop-upload">
      <style jsx>{`
        .sop-upload {
          margin: 14px auto 0;
          max-width: 560px;
          font-family: var(--font-sans);
        }
        .sop-row {
          display: flex;
          align-items: center;
          gap: 10px;
          flex-wrap: wrap;
        }
        .sop-btn {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 9px 16px;
          border-radius: 999px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 35%, transparent);
          background: color-mix(in srgb, var(--brand-primary) 6%, #fff);
          color: var(--brand-ink);
          font-size: 14px;
          font-weight: 600;
          cursor: pointer;
          transition: border-color 0.15s ease, background 0.15s ease;
        }
        .sop-btn:hover:not(:disabled) {
          border-color: var(--brand-primary);
          background: color-mix(in srgb, var(--brand-primary) 12%, #fff);
        }
        .sop-btn:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .sop-hint {
          color: color-mix(in srgb, var(--brand-ink) 60%, transparent);
          font-size: 12.5px;
          line-height: 1.4;
        }
        .sop-chip {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 8px 12px 8px 14px;
          border-radius: 999px;
          /* The locked purple→PINK gradient as a soft tint behind the chip. */
          background: linear-gradient(
            135deg,
            color-mix(in srgb, var(--brand-primary) 14%, #fff),
            color-mix(in srgb, var(--brand-pink) 14%, #fff)
          );
          border: 1.5px solid color-mix(in srgb, var(--brand-pink) 30%, transparent);
          color: var(--brand-ink);
          font-size: 13.5px;
          font-weight: 600;
          max-width: 100%;
        }
        .sop-chip-name {
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          max-width: 320px;
        }
        .sop-chip-clear {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 22px;
          height: 22px;
          border-radius: 999px;
          border: none;
          background: color-mix(in srgb, var(--brand-ink) 8%, transparent);
          color: var(--brand-ink);
          cursor: pointer;
        }
        .sop-chip-clear:hover {
          background: color-mix(in srgb, var(--brand-pink) 22%, transparent);
        }
        .sop-error {
          margin-top: 8px;
          color: var(--brand-coral, #ff6b6b);
          font-size: 12.5px;
          font-weight: 600;
        }
        .sop-icon {
          color: var(--brand-primary);
          flex-shrink: 0;
        }
      `}</style>

      <input
        ref={inputRef}
        type="file"
        accept={ACCEPT}
        hidden
        data-testid="genesis-sop-file-input"
        onChange={(e) => void onFile(e.target.files?.[0])}
      />

      {value ? (
        <div className="sop-row">
          <span className="sop-chip" data-testid="genesis-sop-chip">
            <FileText size={15} className="sop-icon" aria-hidden />
            <span className="sop-chip-name" title={value.filename}>
              {value.filename}
            </span>
            <button
              type="button"
              className="sop-chip-clear"
              onClick={clear}
              disabled={disabled}
              aria-label="Remove attached SOP"
              data-testid="genesis-sop-remove"
            >
              <X size={13} aria-hidden />
            </button>
          </span>
          <span className="sop-hint">
            The teammate will be quizzed on this before it can be hired.
          </span>
        </div>
      ) : (
        <div className="sop-row">
          <button
            type="button"
            className="sop-btn"
            onClick={pick}
            disabled={disabled || reading}
            data-testid="genesis-sop-attach"
          >
            <Paperclip size={15} className="sop-icon" aria-hidden />
            {reading ? "Reading…" : "Attach an SOP (optional)"}
          </button>
          <span className="sop-hint">
            One playbook (.txt, .md, or .pdf). The teammate must demonstrate it
            understood it before hire.
          </span>
        </div>
      )}

      {error ? (
        <div className="sop-error" role="alert" data-testid="genesis-sop-error">
          {error}
        </div>
      ) : null}
    </div>
  );
}

/** Read a File as a base64 string (the PDF path). Strips the data-URL prefix. */
async function readAsBase64(file: File): Promise<string> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}
