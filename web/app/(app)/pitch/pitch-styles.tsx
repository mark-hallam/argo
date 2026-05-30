"use client";

/**
 * Scoped global styles for the `/pitch` rehearsal surface.
 *
 * Extracted from `page.tsx` (which was over the file-size budget) into its
 * own client component. `styled-jsx`'s `global` block works in any client
 * component, so this keeps the locked brand styling intact — cool-paper
 * background, purple→pink gradient, Inter body, mono pills — while the page
 * file stays focused on behaviour. Rendered once near the top of the page.
 */
export function PitchStyles() {
  return (
    <style jsx global>{`
      .pitch-page-bg {
        position: relative;
        min-height: calc(100vh - 64px);
        background: #fafaff;
        overflow: hidden;
      }
      .pitch-page-bg::before {
        content: "";
        position: absolute;
        top: -180px;
        right: -120px;
        width: 540px;
        height: 540px;
        background: radial-gradient(circle, rgba(139, 92, 246, 0.18) 0%, transparent 70%);
        pointer-events: none;
      }
      .pitch-page-bg::after {
        content: "";
        position: absolute;
        bottom: -200px;
        left: -120px;
        width: 600px;
        height: 600px;
        background: radial-gradient(circle, rgba(236, 72, 153, 0.14) 0%, transparent 70%);
        pointer-events: none;
      }
      .pitch-page-content {
        position: relative;
        z-index: 1;
        padding-bottom: 72px;
      }
      .pitch-shell {
        max-width: 720px;
        margin: 28px auto 0;
        padding: 0 16px;
      }
      .pitch-input-card {
        display: flex;
        flex-direction: column;
        gap: 12px;
        padding: 18px;
        background: white;
        border: 1px solid rgba(139, 92, 246, 0.18);
        border-radius: 16px;
        box-shadow: 0 12px 36px -22px rgba(76, 29, 149, 0.35);
      }
      .pitch-input {
        width: 100%;
        min-height: 96px;
        resize: vertical;
        padding: 14px 16px;
        border: 1px solid rgba(15, 23, 42, 0.12);
        border-radius: 12px;
        font-family: "Inter", sans-serif;
        font-size: 15px;
        line-height: 1.5;
        color: #1e1b4b;
        background: #fdfcff;
        outline: none;
      }
      .pitch-input:focus {
        border-color: #8b5cf6;
        box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.18);
      }
      .pitch-actions {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
      }
      .pitch-hint {
        font-family: var(--font-mono);
        font-size: 11px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: rgba(30, 27, 75, 0.55);
      }
      .pitch-submit {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 10px 18px;
        border-radius: 12px;
        border: none;
        background: linear-gradient(135deg, #8b5cf6, #ec4899);
        color: white;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        font-weight: 600;
        letter-spacing: 0.01em;
        cursor: pointer;
        box-shadow: 0 8px 22px -12px rgba(139, 92, 246, 0.65);
      }
      .pitch-submit:disabled {
        opacity: 0.55;
        cursor: not-allowed;
      }
      .pitch-action-buttons {
        display: flex;
        align-items: center;
        gap: 10px;
      }
      .pitch-talk {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 10px 16px;
        border-radius: 12px;
        border: 1px solid rgba(139, 92, 246, 0.4);
        background: white;
        color: #5b21b6;
        font-family: "Inter", sans-serif;
        font-size: 14px;
        font-weight: 600;
        letter-spacing: 0.01em;
        cursor: pointer;
        transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
      }
      .pitch-talk:disabled {
        cursor: not-allowed;
      }
      .pitch-talk.is-recording {
        background: linear-gradient(135deg, #ef4444, #ec4899);
        border-color: transparent;
        color: white;
        box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.18);
        animation: pitch-pulse 1.4s ease-in-out infinite;
      }
      .pitch-talk.is-thinking,
      .pitch-talk.is-speaking {
        background: linear-gradient(135deg, rgba(139, 92, 246, 0.16), rgba(236, 72, 153, 0.14));
        border-color: transparent;
        color: #5b21b6;
      }
      @keyframes pitch-pulse {
        0%,
        100% {
          box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.18);
        }
        50% {
          box-shadow: 0 0 0 8px rgba(239, 68, 68, 0.08);
        }
      }
      .pitch-spin {
        animation: pitch-spin 0.9s linear infinite;
      }
      @keyframes pitch-spin {
        to {
          transform: rotate(360deg);
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .pitch-talk.is-recording {
          animation: none;
        }
        .pitch-spin {
          animation: none;
        }
      }
      .pitch-history {
        margin-top: 24px;
        display: flex;
        flex-direction: column;
        gap: 16px;
      }
      .pitch-empty {
        margin: 20px 0;
        padding: 22px;
        border-radius: 14px;
        background: rgba(255, 255, 255, 0.7);
        border: 1px dashed rgba(139, 92, 246, 0.32);
        color: rgba(30, 27, 75, 0.7);
        text-align: center;
        font-family: "Inter", sans-serif;
        font-size: 14px;
      }
      .pitch-msg {
        padding: 14px 18px;
        border-radius: 14px;
        font-family: "Inter", sans-serif;
        font-size: 15px;
        line-height: 1.5;
        color: #1e1b4b;
      }
      .pitch-msg.is-operator {
        background: linear-gradient(135deg, rgba(139, 92, 246, 0.95), rgba(236, 72, 153, 0.92));
        color: white;
        align-self: flex-end;
        max-width: 80%;
        box-shadow: 0 10px 28px -18px rgba(139, 92, 246, 0.7);
      }
      .pitch-msg.is-agent {
        background: white;
        border: 1px solid rgba(15, 23, 42, 0.08);
        align-self: flex-start;
        max-width: 92%;
        box-shadow: 0 8px 24px -20px rgba(15, 23, 42, 0.3);
      }
      .pitch-msg-text {
        margin: 0;
        white-space: pre-wrap;
      }
      .pitch-meta {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        margin-top: 10px;
      }
      .pitch-pill {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 4px 10px;
        border-radius: 999px;
        font-family: var(--font-mono);
        font-size: 11px;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        font-weight: 600;
      }
      .pitch-pill svg {
        width: 12px;
        height: 12px;
      }
      .pitch-pill.is-allow {
        background: rgba(34, 197, 94, 0.12);
        color: #166534;
      }
      .pitch-pill.is-escalate {
        background: rgba(245, 158, 11, 0.14);
        color: #92400e;
      }
      .pitch-pill.is-block {
        background: rgba(239, 68, 68, 0.14);
        color: #991b1b;
      }
      .pitch-pill.is-entry {
        background: rgba(139, 92, 246, 0.12);
        color: #5b21b6;
        letter-spacing: 0.05em;
        text-transform: none;
      }
      .pitch-pill.is-latency {
        background: rgba(15, 23, 42, 0.05);
        color: rgba(30, 27, 75, 0.7);
        text-transform: none;
        letter-spacing: 0.04em;
      }
      .pitch-pill.is-source-canonical {
        background: rgba(14, 165, 233, 0.12);
        color: #075985;
        text-transform: none;
        letter-spacing: 0.04em;
      }
      .pitch-pill.is-source-llm {
        background: linear-gradient(135deg, rgba(139, 92, 246, 0.18), rgba(236, 72, 153, 0.14));
        color: #5b21b6;
        text-transform: none;
        letter-spacing: 0.04em;
      }
      .pitch-pill.is-source-llm-blocked {
        background: rgba(239, 68, 68, 0.1);
        color: #7f1d1d;
        text-transform: none;
        letter-spacing: 0.04em;
      }
      .pitch-pill.is-tokens {
        background: rgba(15, 23, 42, 0.04);
        color: rgba(30, 27, 75, 0.6);
        text-transform: none;
        letter-spacing: 0.04em;
        font-variant-numeric: tabular-nums;
      }
      .pitch-pill.is-model {
        background: rgba(76, 29, 149, 0.08);
        color: #4c1d95;
        text-transform: none;
        letter-spacing: 0.02em;
      }
      .pitch-swap-note {
        margin-top: 6px;
        font-family: var(--font-mono);
        font-size: 11px;
        letter-spacing: 0.04em;
        color: rgba(127, 29, 29, 0.85);
      }
      .pitch-draft-block {
        margin-top: 10px;
        padding: 10px 12px;
        border-radius: 10px;
        background: rgba(239, 68, 68, 0.05);
        border: 1px dashed rgba(239, 68, 68, 0.35);
        font-family: var(--font-mono);
        font-size: 12px;
        line-height: 1.45;
        color: rgba(127, 29, 29, 0.85);
        white-space: pre-wrap;
      }
      .pitch-draft-label {
        display: block;
        font-family: var(--font-mono);
        font-size: 10px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: rgba(127, 29, 29, 0.7);
        margin-bottom: 4px;
      }
      .pitch-error-panel {
        margin-top: 18px;
        padding: 14px 16px;
        border: 1px solid #fecaca;
        background: rgba(254, 226, 226, 0.6);
        border-radius: 12px;
        color: #7f1d1d;
        display: flex;
        gap: 10px;
        align-items: flex-start;
        font-family: "Inter", sans-serif;
        font-size: 13px;
      }
      @media (max-width: 720px) {
        .pitch-shell {
          margin: 18px auto 0;
        }
        .pitch-input {
          min-height: 80px;
        }
      }
    `}</style>
  );
}
