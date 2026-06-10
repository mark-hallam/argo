"use client";

import { useCallback, useRef, useState, type FormEvent } from "react";
import { Mic, Sparkles, Square, Wand2 } from "lucide-react";
import { useDictation } from "@/lib/api/use-dictation";

/**
 * Genesis page hero input — large brand-loud textarea + "Generate
 * worker" CTA. Sits below the BrandLoudHero banner; renders the
 * locked SKYCOT brand-loud register (gradient CTA pill, mono eyebrow,
 * subtle purple focus ring).
 *
 * Owned by the parent `/genesis` page: it controls the textarea value,
 * the busy state, and what happens on submit (start the SSE stream).
 * This component is presentational + form-mechanics only.
 */

const PLACEHOLDER =
  "Describe the role in plain English. e.g. 'A customer success lead who triages tier-1 tickets, drafts response templates, and escalates refunds over $500'.";

/** Mirrors the Rust handler's `MIN_ROLE_LEN`. Keep in sync. */
const MIN_ROLE_LEN = 15;

export interface HeroInputProps {
  value: string;
  onChange(next: string): void;
  onSubmit(): void;
  isBusy: boolean;
}

export function HeroInput({ value, onChange, onSubmit, isBusy }: HeroInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [touched, setTouched] = useState(false);

  // Mic dictation: each transcribed clip is appended to the controlled value
  // via the existing `onChange` — `page.tsx` stays the sole state owner. A ref
  // keeps the append based on the LATEST value (the hook callback is stable).
  const valueRef = useRef(value);
  valueRef.current = value;
  const appendTranscript = useCallback(
    (text: string) => {
      const prev = valueRef.current.trimEnd();
      onChange(prev.length > 0 ? `${prev} ${text}` : text);
    },
    [onChange],
  );
  const dictation = useDictation({ onChunk: appendTranscript });

  const trimmed = value.trim();
  const tooShort = trimmed.length < MIN_ROLE_LEN;
  const showError = touched && tooShort;

  const handleSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();
      setTouched(true);
      if (tooShort || isBusy) {
        textareaRef.current?.focus();
        return;
      }
      onSubmit();
    },
    [tooShort, isBusy, onSubmit],
  );

  return (
    <>
      <style jsx>{`
        .gen-hero-form {
          display: flex;
          flex-direction: column;
          gap: 18px;
          max-width: 880px;
          margin: 32px auto 0;
          padding: 0 24px;
        }
        .gen-hero-label {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.18em;
          font-weight: 700;
          color: #6d28d9;
          text-transform: uppercase;
          display: inline-flex;
          align-items: center;
          gap: 8px;
        }
        .gen-hero-textarea {
          width: 100%;
          min-height: 156px;
          padding: 22px 24px;
          font-family: "Inter", sans-serif;
          font-size: 17px;
          line-height: 1.5;
          color: var(--foreground);
          background: var(--card);
          border: 1.5px solid var(--border);
          border-radius: 16px;
          resize: vertical;
          outline: none;
          box-shadow: 0 8px 24px -16px rgba(30, 27, 75, 0.18);
          transition: border-color 0.15s, box-shadow 0.15s;
        }
        .gen-hero-textarea::placeholder {
          color: var(--muted-foreground);
          font-weight: 400;
        }
        .gen-hero-textarea:focus {
          border-color: #8b5cf6;
          box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.12),
            0 8px 24px -16px rgba(30, 27, 75, 0.18);
        }
        .gen-hero-textarea.is-error {
          border-color: #b91c1c;
        }
        .gen-hero-cta-row {
          display: flex;
          align-items: center;
          gap: 18px;
          flex-wrap: wrap;
        }
        .gen-hero-cta {
          min-height: 48px;
          padding: 0 28px;
          display: inline-flex;
          align-items: center;
          gap: 10px;
          font-family: "Inter", sans-serif;
          font-size: 15.5px;
          font-weight: 700;
          letter-spacing: -0.005em;
          color: white;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          border: none;
          border-radius: 14px;
          cursor: pointer;
          box-shadow: 0 10px 28px -10px rgba(139, 92, 246, 0.55),
            inset 0 1px 0 rgba(255, 255, 255, 0.25);
          transition: transform 0.15s, box-shadow 0.15s, opacity 0.15s;
        }
        .gen-hero-cta:hover:not(:disabled) {
          transform: translateY(-2px);
        }
        .gen-hero-cta:disabled {
          opacity: 0.55;
          cursor: not-allowed;
          box-shadow: none;
        }
        .gen-hero-hint {
          font-size: 13px;
          color: #52525b;
        }
        .gen-hero-error {
          font-size: 13px;
          color: #b91c1c;
        }
        .gen-hero-mic {
          min-height: 48px;
          padding: 0 20px;
          display: inline-flex;
          align-items: center;
          gap: 9px;
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          font-weight: 600;
          color: #6d28d9;
          background: var(--card);
          border: 1.5px solid var(--border);
          border-radius: 14px;
          cursor: pointer;
          transition: border-color 0.15s, color 0.15s, opacity 0.15s;
        }
        .gen-hero-mic:hover:not(:disabled) {
          border-color: #8b5cf6;
        }
        .gen-hero-mic:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .gen-hero-mic.is-recording {
          color: #b91c1c;
          border-color: #fca5a5;
          background: rgba(254, 226, 226, 0.55);
        }
        .gen-hero-mic-dot {
          width: 9px;
          height: 9px;
          border-radius: 50%;
          background: #ef4444;
          animation: gen-hero-pulse 1.1s ease-in-out infinite;
        }
        @keyframes gen-hero-pulse {
          0%,
          100% {
            opacity: 1;
            transform: scale(1);
          }
          50% {
            opacity: 0.45;
            transform: scale(0.7);
          }
        }
      `}</style>
      <form
        className="gen-hero-form"
        onSubmit={handleSubmit}
        data-testid="genesis-hero-form"
      >
        <label className="gen-hero-label" htmlFor="genesis-role-input">
          <Sparkles size={14} aria-hidden />
          Describe your AI teammate
        </label>
        <textarea
          ref={textareaRef}
          id="genesis-role-input"
          className={`gen-hero-textarea${showError ? " is-error" : ""}`}
          placeholder={PLACEHOLDER}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={isBusy}
          aria-invalid={showError}
          aria-describedby={showError ? "genesis-role-error" : "genesis-role-hint"}
        />
        <div className="gen-hero-cta-row">
          <button
            type="submit"
            className="gen-hero-cta"
            disabled={isBusy || tooShort}
            data-testid="genesis-generate-button"
          >
            <Wand2 size={18} aria-hidden />
            {isBusy ? "Generating…" : "Generate worker"}
          </button>
          <button
            type="button"
            className={`gen-hero-mic${dictation.isRecording ? " is-recording" : ""}`}
            onClick={dictation.toggle}
            disabled={isBusy}
            aria-pressed={dictation.isRecording}
            aria-label={
              dictation.isRecording ? "Stop dictation" : "Describe the role by voice"
            }
            data-testid="genesis-mic-button"
          >
            {dictation.isRecording ? (
              <>
                <span className="gen-hero-mic-dot" aria-hidden />
                <Square size={15} aria-hidden />
                Listening…
              </>
            ) : (
              <>
                <Mic size={17} aria-hidden />
                Dictate
              </>
            )}
          </button>
          {showError ? (
            <p id="genesis-role-error" className="gen-hero-error">
              Please describe the role in at least {MIN_ROLE_LEN} characters.
            </p>
          ) : (
            <p id="genesis-role-hint" className="gen-hero-hint">
              We&apos;ll draft a real worker — persona, charter, and skills —
              and walk you through reviewing it.
            </p>
          )}
        </div>
        {dictation.error ? (
          <p className="gen-hero-error" role="status" data-testid="genesis-mic-error">
            {dictation.error}
          </p>
        ) : null}
      </form>
    </>
  );
}
