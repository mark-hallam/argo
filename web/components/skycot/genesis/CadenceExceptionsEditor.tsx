"use client";

import { useCallback, useRef, useState } from "react";
import { SlidersHorizontal } from "lucide-react";
import {
  CadenceExceptionsCapture,
  type CadenceExceptionsSelection,
} from "@/components/skycot/genesis/CadenceExceptionsCapture";
import type { GenesisVerticalReadyEvent } from "@/lib/api/genesis";
import {
  applyIntakeEdits,
  intakeFromContract,
} from "@/lib/api/genesis-hire-intake";

/**
 * Pre-hire "review and edit" seam for the captured cadence / exceptions /
 * success-criteria (DEC-025 #7).
 *
 * The streamed `vertical_ready` review is otherwise read-only — a wrong
 * capture can't be corrected before the hire commits. This panel offers an
 * "Adjust" affordance that pre-fills from the SERVER echo
 * (`ready.role_contract`, single source of truth — no client-side divergence)
 * and lets the operator correct the three intake slots before confirming.
 *
 * Edits are ADVISORY (empty / skipped fields fall back to the server echo) and
 * are NOT part of the governance `contract_digest`, so adjusting them never
 * gates the Confirm button. The merged event is emitted via `onChange` so the
 * page's hire call carries the edited values. Reuses
 * [[CadenceExceptionsCapture]] as the form internals — does not fork a second
 * capture form. Consumes ONLY the locked brand tokens (DEC-027).
 */

export interface CadenceExceptionsEditorProps {
  /** The streamed `vertical_ready` event whose role contract seeds the form. */
  ready: GenesisVerticalReadyEvent;
  /** Emits a NEW event with the operator's edits merged into `role_contract`. */
  onChange(next: GenesisVerticalReadyEvent): void;
  /** Disabled while the hire is in flight. */
  disabled?: boolean;
}

export function CadenceExceptionsEditor({
  ready,
  onChange,
  disabled,
}: CadenceExceptionsEditorProps) {
  // Capture the ORIGINAL server contract once so empty edits always fall back
  // to the server echo, not to a previously-edited value (avoids compounding
  // when the merged event is fed back in on re-render).
  const originalRef = useRef<GenesisVerticalReadyEvent>(ready);
  const [open, setOpen] = useState(false);
  const [edits, setEdits] = useState<CadenceExceptionsSelection>(() =>
    intakeFromContract(ready.role_contract),
  );

  const handleChange = useCallback(
    (next: CadenceExceptionsSelection) => {
      setEdits(next);
      onChange(applyIntakeEdits(originalRef.current, next));
    },
    [onChange],
  );

  // No role contract → nothing to adjust (older gateways). Render nothing.
  if (!ready.role_contract) return null;

  return (
    <div className="cee" data-testid="genesis-cadence-editor">
      <style jsx>{`
        .cee {
          margin: 18px auto 0;
          max-width: 560px;
          font-family: var(--font-sans);
          color: var(--brand-ink);
        }
        .cee-toggle {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 9px 14px;
          border-radius: 999px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 35%, transparent);
          background: color-mix(in srgb, var(--brand-primary) 6%, #fff);
          color: var(--brand-ink);
          font-size: 13.5px;
          font-weight: 600;
          cursor: pointer;
          transition: border-color 0.15s ease, background 0.15s ease;
        }
        .cee-toggle:hover:not(:disabled) {
          border-color: var(--brand-primary);
          background: color-mix(in srgb, var(--brand-primary) 12%, #fff);
        }
        .cee-toggle:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .cee-icon {
          color: var(--brand-primary);
          flex-shrink: 0;
        }
        .cee-hint {
          color: color-mix(in srgb, var(--brand-ink) 60%, transparent);
          font-size: 12.5px;
          line-height: 1.4;
          margin: 8px 0 0;
        }
      `}</style>
      <button
        type="button"
        className="cee-toggle"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen((v) => !v)}
        data-testid="genesis-cadence-editor-toggle"
      >
        <SlidersHorizontal size={15} className="cee-icon" aria-hidden />
        {open ? "Hide adjustments" : "Adjust cadence, exceptions & success"}
      </button>
      {open ? (
        <>
          <p className="cee-hint">
            Correct what Genesis captured before you confirm. Anything you leave
            blank keeps what the teammate was generated with.
          </p>
          <CadenceExceptionsCapture
            value={edits}
            onChange={handleChange}
            disabled={disabled}
          />
        </>
      ) : null}
    </div>
  );
}
