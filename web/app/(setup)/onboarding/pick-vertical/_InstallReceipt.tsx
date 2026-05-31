"use client";

import type { CSSProperties } from "react";

export interface InstallVerticalReceipt {
  vertical_id: string;
  skills_added: number;
  memory_schema_attached: boolean;
  approval_gates_added: number;
}

interface Props {
  receipt: InstallVerticalReceipt;
  onPickAgain: () => void;
  onContinue: () => void;
}

const rowStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 12,
  fontSize: 14.5,
  color: "var(--brand-ink)",
};

function tickStyle(filled: boolean): CSSProperties {
  return {
    width: 26,
    height: 26,
    flexShrink: 0,
    display: "grid",
    placeItems: "center",
    borderRadius: 999,
    fontSize: 13,
    fontWeight: 700,
    color: filled ? "var(--brand-gradient-foreground)" : "var(--brand-ink-4)",
    background: filled ? "var(--brand-gradient)" : "transparent",
    border: filled ? "none" : "1.5px solid var(--brand-line-strong)",
  };
}

/**
 * The 3-row "what just got installed" confirmation panel shown on
 * `/onboarding/pick-vertical` after the gateway `install-vertical`
 * call returns. Closes UX-AUDIT-T0119MEL F8: the wizard's sub-copy
 * promised skills + memory schema + approval gates but the receipt
 * only confirmed skills.
 */
export function InstallReceipt({ receipt, onPickAgain, onContinue }: Props) {
  const hasMemory = receipt.memory_schema_attached;
  const gateCount = receipt.approval_gates_added;
  const hasGates = gateCount > 0;
  return (
    <div
      className="ob-stack"
      data-testid="onboarding-vertical-receipt"
      role="status"
      aria-live="polite"
      style={{
        padding: "28px 24px 24px",
        borderRadius: 18,
        border: "1px solid var(--brand-line)",
        background: "color-mix(in oklab, var(--brand-paper) 85%, transparent)",
        boxShadow:
          "0 12px 36px -16px color-mix(in oklab, var(--brand-ink) 12%, transparent)",
      }}
    >
      <h2
        style={{
          fontSize: 20,
          fontWeight: 700,
          color: "var(--brand-ink)",
          letterSpacing: "-0.015em",
          margin: 0,
        }}
      >
        Installed for{" "}
        <strong data-testid="onboarding-vertical-receipt-id">
          {receipt.vertical_id}
        </strong>
        .
      </h2>
      <ul
        style={{
          listStyle: "none",
          padding: 0,
          margin: 0,
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        <li data-testid="onboarding-vertical-receipt-skills" style={rowStyle}>
          <span aria-hidden style={tickStyle(true)}>
            ✓
          </span>
          <span>
            <strong>{receipt.skills_added}</strong>{" "}
            {receipt.skills_added === 1 ? "skill" : "skills"} attached
          </span>
        </li>
        <li data-testid="onboarding-vertical-receipt-memory" style={rowStyle}>
          <span aria-hidden style={tickStyle(hasMemory)}>
            {hasMemory ? "✓" : "—"}
          </span>
          <span>
            {hasMemory
              ? "Memory schema attached"
              : "No memory schema in this pack"}
          </span>
        </li>
        <li data-testid="onboarding-vertical-receipt-gates" style={rowStyle}>
          <span aria-hidden style={tickStyle(hasGates)}>
            {hasGates ? "✓" : "—"}
          </span>
          <span>
            {hasGates ? (
              <>
                Approval gates (<strong>{gateCount}</strong>{" "}
                {gateCount === 1 ? "policy" : "policies"})
              </>
            ) : (
              <>No approval gates in this pack</>
            )}
          </span>
        </li>
      </ul>
      <div className="ob-cta-row is-split">
        <button
          type="button"
          className="ob-cta-back"
          onClick={onPickAgain}
          data-testid="onboarding-vertical-receipt-pick-again"
        >
          ← Pick a different template
        </button>
        <button
          type="button"
          className="ob-cta"
          onClick={onContinue}
          data-testid="onboarding-vertical-receipt-continue"
        >
          Continue →
        </button>
      </div>
    </div>
  );
}
