"use client";

import { MessagesSquare, PencilLine } from "lucide-react";

/**
 * "Describe it" ↔ "Talk it through" entry toggle on `/genesis` (UF Phase-A ·
 * A3, DEC-039). ADDITIVE — both modes land on the identical review → hire path;
 * this only chooses how the operator describes the role.
 *
 * Presentational only; consumes ONLY the locked brand tokens from `globals.css`
 * (purple→PINK gradient, Inter) — no color literals, no new design (DEC-027).
 */

export type GenesisMode = "describe" | "talk";

export interface GenesisModeToggleProps {
  mode: GenesisMode;
  onChange(mode: GenesisMode): void;
  disabled?: boolean;
}

export function GenesisModeToggle({
  mode,
  onChange,
  disabled,
}: GenesisModeToggleProps) {
  return (
    <div className="gmt" role="tablist" aria-label="How to describe the role" data-testid="genesis-mode-toggle">
      <style jsx>{`
        .gmt {
          margin: 18px auto 0;
          max-width: 560px;
          display: flex;
          gap: 6px;
          padding: 5px;
          border-radius: 999px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 22%, transparent);
          background: color-mix(in srgb, var(--brand-primary) 4%, #fff);
        }
        .gmt-btn {
          flex: 1 1 0;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          padding: 9px 16px;
          border-radius: 999px;
          border: none;
          background: transparent;
          font-family: var(--font-sans);
          font-size: 14px;
          font-weight: 600;
          color: color-mix(in srgb, var(--brand-ink) 70%, transparent);
          cursor: pointer;
          transition: background 0.15s ease, color 0.15s ease;
        }
        .gmt-btn:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .gmt-btn.is-active {
          color: #fff;
          background: linear-gradient(135deg, var(--brand-primary), var(--brand-pink));
        }
        .gmt-btn:not(.is-active):hover:not(:disabled) {
          background: color-mix(in srgb, var(--brand-primary) 8%, transparent);
        }
      `}</style>
      <button
        type="button"
        role="tab"
        aria-selected={mode === "describe"}
        className={`gmt-btn${mode === "describe" ? " is-active" : ""}`}
        onClick={() => onChange("describe")}
        disabled={disabled}
        data-testid="genesis-mode-describe"
      >
        <PencilLine size={15} aria-hidden />
        Describe it
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === "talk"}
        className={`gmt-btn${mode === "talk" ? " is-active" : ""}`}
        onClick={() => onChange("talk")}
        disabled={disabled}
        data-testid="genesis-mode-talk"
      >
        <MessagesSquare size={15} aria-hidden />
        Talk it through
      </button>
    </div>
  );
}
