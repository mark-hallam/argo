"use client";

import { FileText, ScrollText, UserCircle2 } from "lucide-react";
import {
  CadenceExceptionsCapture,
  type CadenceExceptionsSelection,
} from "@/components/skycot/genesis/CadenceExceptionsCapture";
import type { ConversationCaptured } from "@/lib/api/genesis-conversation";

/**
 * Live "what we've captured" panel for the conversational intake (UF Phase-A ·
 * A3, DEC-039). Driven by the turn response's `captured` block; updates as the
 * chat fills slots.
 *
 * The three typed intake slots (cadence / exceptions / success-criteria) REUSE
 * the existing [[CadenceExceptionsCapture]] slot rendering in disabled (read-
 * only) mode rather than forking it — same brand tokens, same chips, same
 * criterion rows. Charter / persona / responsibilities are free-text slots the
 * capture panel doesn't model, so they get small read-only sections above it.
 *
 * Presentational only; consumes ONLY locked brand tokens from `globals.css`
 * (DEC-027) — no color literals, no new design.
 */

export interface ConversationCapturedPanelProps {
  captured: ConversationCaptured;
  /**
   * The slot the engine is CURRENTLY gathering (the turn response's `slot`),
   * or `undefined` once the conversation is `ready`/`done` (everything is
   * resolved) or when no turn context is available.
   *
   * Used to gate each field on its RESOLVED slot: a value only paints once the
   * conversation has moved PAST that slot's question. The engine's per-turn
   * `captured` block can briefly carry a value in a not-yet-asked slot (an early
   * rich answer can over-attribute, e.g., a carve-out phrase into `cadence`
   * before the cadence question is asked). Without this gate the panel paints
   * that stray value into the wrong input for a frame until the real answer
   * overwrites it (cosmetic glitch N4, 2026-06-13 shakedown). Gating by ask
   * order means the active/unanswered slot is never trusted until the user has
   * actually answered it and the conversation has advanced.
   */
  activeSlot?: string;
}

/** The deterministic ask order (mirrors `argo_genesis::IntakeSlot::slug`). A
 *  slot is "resolved" once the conversation has advanced strictly past it. */
const SLOT_ORDER: readonly string[] = [
  "charter",
  "persona",
  "responsibilities",
  "cadence",
  "exceptions",
  "success_criteria",
  "access",
  "escalation",
];

/** `true` when `slot` is resolved relative to the currently-gathered
 *  `activeSlot`. With no `activeSlot` (ready/done or no turn context) every
 *  field is resolved. An unrecognized `activeSlot` is treated as "all
 *  resolved" so a new slot can never silently blank the whole panel. */
function isResolved(slot: string, activeSlot?: string): boolean {
  if (!activeSlot) return true;
  const activeIdx = SLOT_ORDER.indexOf(activeSlot);
  if (activeIdx === -1) return true;
  const slotIdx = SLOT_ORDER.indexOf(slot);
  return slotIdx < activeIdx;
}

function hasAnything(c: ConversationCaptured): boolean {
  return (
    c.charter.length > 0 ||
    c.persona.length > 0 ||
    c.responsibilities.length > 0 ||
    c.cadence.length > 0 ||
    c.exceptions.length > 0 ||
    c.success_criteria.length > 0
  );
}

export function ConversationCapturedPanel({
  captured: capturedProp,
  activeSlot,
}: ConversationCapturedPanelProps) {
  // Gate the captured block on resolved slots BEFORE rendering, so a value
  // over-attributed to a not-yet-asked slot never paints into its input.
  const captured: ConversationCaptured = {
    charter: isResolved("charter", activeSlot) ? capturedProp.charter : "",
    persona: isResolved("persona", activeSlot) ? capturedProp.persona : "",
    responsibilities: isResolved("responsibilities", activeSlot)
      ? capturedProp.responsibilities
      : [],
    cadence: isResolved("cadence", activeSlot) ? capturedProp.cadence : "",
    exceptions: isResolved("exceptions", activeSlot)
      ? capturedProp.exceptions
      : [],
    success_criteria: isResolved("success_criteria", activeSlot)
      ? capturedProp.success_criteria
      : [],
    access_integrations: isResolved("access", activeSlot)
      ? capturedProp.access_integrations
      : [],
    escalation_failures_threshold: isResolved("escalation", activeSlot)
      ? capturedProp.escalation_failures_threshold
      : null,
    escalation_to: isResolved("escalation", activeSlot)
      ? capturedProp.escalation_to
      : "",
  };
  const cadenceSlots: CadenceExceptionsSelection = {
    cadence: captured.cadence,
    exceptions: captured.exceptions,
    success_criteria: captured.success_criteria,
  };
  const showTyped =
    captured.cadence.length > 0 ||
    captured.exceptions.length > 0 ||
    captured.success_criteria.length > 0;

  return (
    <aside className="ccp" data-testid="genesis-conversation-captured">
      <style jsx>{`
        .ccp {
          font-family: var(--font-sans);
          color: var(--brand-ink);
          display: flex;
          flex-direction: column;
          gap: 16px;
          text-align: left;
        }
        .ccp-title {
          font-size: 13px;
          font-weight: 700;
          letter-spacing: 0.02em;
          text-transform: uppercase;
          color: color-mix(in srgb, var(--brand-ink) 55%, transparent);
        }
        .ccp-empty {
          font-size: 13px;
          color: color-mix(in srgb, var(--brand-ink) 55%, transparent);
          line-height: 1.5;
        }
        .ccp-label {
          display: inline-flex;
          align-items: center;
          gap: 7px;
          font-size: 13.5px;
          font-weight: 700;
          margin-bottom: 4px;
        }
        .ccp-icon {
          color: var(--brand-primary);
          flex-shrink: 0;
        }
        .ccp-text {
          font-size: 13.5px;
          line-height: 1.5;
          color: color-mix(in srgb, var(--brand-ink) 82%, transparent);
        }
        .ccp-list {
          margin: 4px 0 0;
          padding-left: 18px;
          font-size: 13.5px;
          line-height: 1.5;
          color: color-mix(in srgb, var(--brand-ink) 82%, transparent);
        }
      `}</style>

      <span className="ccp-title">What we&rsquo;ve captured</span>

      {!hasAnything(captured) ? (
        <p className="ccp-empty" data-testid="genesis-captured-empty">
          As you talk it through, the role takes shape here — charter, persona,
          responsibilities, cadence, exceptions, and success criteria.
        </p>
      ) : null}

      {captured.charter.length > 0 ? (
        <div data-testid="genesis-captured-charter">
          <span className="ccp-label">
            <ScrollText size={15} className="ccp-icon" aria-hidden />
            Charter
          </span>
          <p className="ccp-text">{captured.charter}</p>
        </div>
      ) : null}

      {captured.persona.length > 0 ? (
        <div data-testid="genesis-captured-persona">
          <span className="ccp-label">
            <UserCircle2 size={15} className="ccp-icon" aria-hidden />
            Persona
          </span>
          <p className="ccp-text">{captured.persona}</p>
        </div>
      ) : null}

      {captured.responsibilities.length > 0 ? (
        <div data-testid="genesis-captured-responsibilities">
          <span className="ccp-label">
            <FileText size={15} className="ccp-icon" aria-hidden />
            Responsibilities
          </span>
          <ul className="ccp-list">
            {captured.responsibilities.map((r, idx) => (
              <li key={`${r}-${idx}`}>{r}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {showTyped ? (
        // Reuse the locked CadenceExceptionsCapture slot rendering, disabled =
        // read-only. `onChange` is a no-op: this panel mirrors what the engine
        // captured; edits happen via the chat, not here.
        <CadenceExceptionsCapture
          value={cadenceSlots}
          onChange={() => {}}
          disabled
        />
      ) : null}

      {captured.access_integrations.length > 0 ? (
        <div data-testid="genesis-captured-access">
          <span className="ccp-label">
            <FileText size={15} className="ccp-icon" aria-hidden />
            Access needed (draft-only)
          </span>
          <ul className="ccp-list">
            {captured.access_integrations.map((a, idx) => (
              <li key={`${a}-${idx}`}>{a}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {captured.escalation_failures_threshold !== null ||
      captured.escalation_to.length > 0 ? (
        <div data-testid="genesis-captured-escalation">
          <span className="ccp-label">
            <ScrollText size={15} className="ccp-icon" aria-hidden />
            Escalation
          </span>
          <p className="ccp-text">
            {captured.escalation_failures_threshold !== null
              ? `Escalates after ${captured.escalation_failures_threshold} failed ${
                  captured.escalation_failures_threshold === 1 ? "try" : "tries"
                }`
              : "Escalates"}
            {captured.escalation_to.length > 0
              ? ` to ${captured.escalation_to}`
              : ""}
            .
          </p>
        </div>
      ) : null}
    </aside>
  );
}
