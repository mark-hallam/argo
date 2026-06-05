"use client";

import { useCallback, useState } from "react";
import { CalendarClock, Plus, Target, X } from "lucide-react";
import type { SuccessCriterion } from "@/lib/api/genesis";

/**
 * Genesis intake capture panel — cadence / exceptions / success-criteria
 * (UF Phase-A · A1).
 *
 * The Phase-2 intake captures guardrails / systems / docs / voice /
 * comprehension, but nothing records WHEN a teammate runs, WHAT exceptions
 * apply, or HOW success is measured. This panel captures those three as typed
 * slots that thread into the SSE request and persist on the `CapabilitySpec`
 * (surfaced back on the `vertical_ready` review).
 *
 * Presentational + state-mechanics only: the parent `/genesis` page owns the
 * resulting `CadenceExceptionsSelection`. Consumes ONLY the locked brand tokens
 * from `globals.css` (purple→PINK gradient, Inter, `--brand-ink`) — no color
 * literals, no new design (DEC-027). Follows the `SopUpload` panel pattern; it
 * does NOT fork the panel tree.
 */

/** The KPI `kind` wire tokens (mirrors `argo_vertical::KpiKind`). */
const KIND_OPTIONS: ReadonlyArray<{ value: string; label: string }> = [
  { value: "percentage", label: "Percentage (%)" },
  { value: "duration_ms", label: "Duration (ms)" },
  { value: "count", label: "Count" },
  { value: "ratio", label: "Ratio (0–1)" },
  { value: "score", label: "Score" },
];

/** The KPI `comparator` wire tokens (mirrors `argo_vertical::KpiComparator`). */
const COMPARATOR_OPTIONS: ReadonlyArray<{ value: string; label: string }> = [
  { value: "gte", label: "≥ at least" },
  { value: "lte", label: "≤ at most" },
  { value: "eq", label: "= exactly" },
];

/** What the panel yields to the parent for the SSE request. Field shapes mirror
 *  the new `CapabilitySpec` slots; `cadence` is empty-string when unset. */
export interface CadenceExceptionsSelection {
  /** Free-text cadence/schedule (e.g. `"every weekday at 9am"`). */
  cadence: string;
  /** Carve-outs the teammate must NOT act on / must hand off. */
  exceptions: string[];
  /** Typed, measurable success criteria. */
  success_criteria: SuccessCriterion[];
}

export interface CadenceExceptionsCaptureProps {
  /** The current selection. */
  value: CadenceExceptionsSelection;
  /** Replace the selection. */
  onChange(next: CadenceExceptionsSelection): void;
  /** Disabled while a generation is in flight. */
  disabled?: boolean;
}

/** The empty selection a parent can seed state with. */
export const EMPTY_CADENCE_EXCEPTIONS: CadenceExceptionsSelection = {
  cadence: "",
  exceptions: [],
  success_criteria: [],
};

export function CadenceExceptionsCapture({
  value,
  onChange,
  disabled,
}: CadenceExceptionsCaptureProps) {
  const [exceptionDraft, setExceptionDraft] = useState("");

  const setCadence = useCallback(
    (cadence: string) => onChange({ ...value, cadence }),
    [onChange, value],
  );

  const addException = useCallback(() => {
    const trimmed = exceptionDraft.trim();
    if (!trimmed) return;
    onChange({ ...value, exceptions: [...value.exceptions, trimmed] });
    setExceptionDraft("");
  }, [exceptionDraft, onChange, value]);

  const removeException = useCallback(
    (idx: number) =>
      onChange({
        ...value,
        exceptions: value.exceptions.filter((_, i) => i !== idx),
      }),
    [onChange, value],
  );

  const addCriterion = useCallback(() => {
    const next: SuccessCriterion = {
      name: "",
      kind: "count",
      target: 0,
      comparator: "gte",
    };
    onChange({ ...value, success_criteria: [...value.success_criteria, next] });
  }, [onChange, value]);

  const updateCriterion = useCallback(
    (idx: number, patch: Partial<SuccessCriterion>) =>
      onChange({
        ...value,
        success_criteria: value.success_criteria.map((c, i) =>
          i === idx ? { ...c, ...patch } : c,
        ),
      }),
    [onChange, value],
  );

  const removeCriterion = useCallback(
    (idx: number) =>
      onChange({
        ...value,
        success_criteria: value.success_criteria.filter((_, i) => i !== idx),
      }),
    [onChange, value],
  );

  return (
    <div className="cec" data-testid="genesis-cadence-exceptions">
      <style jsx>{`
        .cec {
          margin: 14px auto 0;
          max-width: 560px;
          font-family: var(--font-sans);
          color: var(--brand-ink);
          display: flex;
          flex-direction: column;
          gap: 18px;
          text-align: left;
        }
        .cec-section-label {
          display: inline-flex;
          align-items: center;
          gap: 7px;
          font-size: 13.5px;
          font-weight: 700;
        }
        .cec-icon {
          color: var(--brand-primary);
          flex-shrink: 0;
        }
        .cec-hint {
          color: color-mix(in srgb, var(--brand-ink) 60%, transparent);
          font-size: 12.5px;
          line-height: 1.4;
          margin: 3px 0 8px;
        }
        .cec-input,
        .cec-select {
          font-family: inherit;
          font-size: 14px;
          color: var(--brand-ink);
          padding: 9px 12px;
          border-radius: 10px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 28%, transparent);
          background: color-mix(in srgb, var(--brand-primary) 4%, #fff);
        }
        .cec-input:focus,
        .cec-select:focus {
          outline: none;
          border-color: var(--brand-primary);
        }
        .cec-input:disabled,
        .cec-select:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .cec-cadence-input {
          width: 100%;
        }
        .cec-row {
          display: flex;
          align-items: center;
          gap: 8px;
          flex-wrap: wrap;
        }
        .cec-grow {
          flex: 1 1 auto;
          min-width: 0;
        }
        .cec-btn {
          display: inline-flex;
          align-items: center;
          gap: 6px;
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
        .cec-btn:hover:not(:disabled) {
          border-color: var(--brand-primary);
          background: color-mix(in srgb, var(--brand-primary) 12%, #fff);
        }
        .cec-btn:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .cec-chip {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 7px 10px 7px 13px;
          border-radius: 999px;
          background: linear-gradient(
            135deg,
            color-mix(in srgb, var(--brand-primary) 12%, #fff),
            color-mix(in srgb, var(--brand-pink) 12%, #fff)
          );
          border: 1.5px solid color-mix(in srgb, var(--brand-pink) 28%, transparent);
          font-size: 13px;
          font-weight: 600;
          max-width: 100%;
        }
        .cec-chip-text {
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          max-width: 420px;
        }
        .cec-chip-clear {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 20px;
          height: 20px;
          border-radius: 999px;
          border: none;
          background: color-mix(in srgb, var(--brand-ink) 8%, transparent);
          color: var(--brand-ink);
          cursor: pointer;
          flex-shrink: 0;
        }
        .cec-chip-clear:hover {
          background: color-mix(in srgb, var(--brand-pink) 22%, transparent);
        }
        .cec-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-bottom: 8px;
        }
        .cec-criterion {
          display: flex;
          align-items: center;
          gap: 8px;
          flex-wrap: wrap;
          padding: 10px;
          border-radius: 12px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 18%, transparent);
          margin-bottom: 8px;
        }
        .cec-target {
          width: 92px;
        }
      `}</style>

      {/* Cadence — WHEN it runs. */}
      <div>
        <span className="cec-section-label">
          <CalendarClock size={15} className="cec-icon" aria-hidden />
          When does this teammate run?
        </span>
        <p className="cec-hint">
          A cadence or trigger — e.g. “every weekday at 9am”, or “on each inbound
          ticket”. Optional.
        </p>
        <input
          type="text"
          className="cec-input cec-cadence-input"
          placeholder="every weekday at 9am"
          value={value.cadence}
          disabled={disabled}
          data-testid="genesis-cadence-input"
          onChange={(e) => setCadence(e.target.value)}
        />
      </div>

      {/* Exceptions — WHAT carve-outs apply. */}
      <div>
        <span className="cec-section-label">
          <X size={15} className="cec-icon" aria-hidden />
          What should it never do?
        </span>
        <p className="cec-hint">
          Carve-outs the teammate must hand off, not act on — e.g. “never touch
          legal escalations”. Optional.
        </p>
        {value.exceptions.length > 0 ? (
          <div className="cec-chips" data-testid="genesis-exception-chips">
            {value.exceptions.map((ex, idx) => (
              <span className="cec-chip" key={`${ex}-${idx}`}>
                <span className="cec-chip-text" title={ex}>
                  {ex}
                </span>
                <button
                  type="button"
                  className="cec-chip-clear"
                  onClick={() => removeException(idx)}
                  disabled={disabled}
                  aria-label={`Remove exception: ${ex}`}
                  data-testid={`genesis-exception-remove-${idx}`}
                >
                  <X size={12} aria-hidden />
                </button>
              </span>
            ))}
          </div>
        ) : null}
        <div className="cec-row">
          <input
            type="text"
            className="cec-input cec-grow"
            placeholder="never touch legal escalations"
            value={exceptionDraft}
            disabled={disabled}
            data-testid="genesis-exception-input"
            onChange={(e) => setExceptionDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addException();
              }
            }}
          />
          <button
            type="button"
            className="cec-btn"
            onClick={addException}
            disabled={disabled || exceptionDraft.trim().length === 0}
            data-testid="genesis-exception-add"
          >
            <Plus size={14} aria-hidden />
            Add
          </button>
        </div>
      </div>

      {/* Success criteria — HOW success is measured. */}
      <div>
        <span className="cec-section-label">
          <Target size={15} className="cec-icon" aria-hidden />
          How will you measure success?
        </span>
        <p className="cec-hint">
          Typed, measurable goals — a metric, a target, and a direction. These
          become the teammate’s success criteria. Optional.
        </p>
        {value.success_criteria.map((c, idx) => (
          <div className="cec-criterion" key={idx} data-testid={`genesis-criterion-${idx}`}>
            <input
              type="text"
              className="cec-input cec-grow"
              placeholder="on-time dispatch rate"
              value={c.name}
              disabled={disabled}
              data-testid={`genesis-criterion-name-${idx}`}
              onChange={(e) => updateCriterion(idx, { name: e.target.value })}
            />
            <select
              className="cec-select"
              value={c.comparator}
              disabled={disabled}
              aria-label="Comparator"
              data-testid={`genesis-criterion-comparator-${idx}`}
              onChange={(e) => updateCriterion(idx, { comparator: e.target.value })}
            >
              {COMPARATOR_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <input
              type="number"
              className="cec-input cec-target"
              value={Number.isFinite(c.target) ? c.target : 0}
              disabled={disabled}
              aria-label="Target"
              data-testid={`genesis-criterion-target-${idx}`}
              onChange={(e) => updateCriterion(idx, { target: Number(e.target.value) })}
            />
            <select
              className="cec-select"
              value={c.kind}
              disabled={disabled}
              aria-label="Metric kind"
              data-testid={`genesis-criterion-kind-${idx}`}
              onChange={(e) => updateCriterion(idx, { kind: e.target.value })}
            >
              {KIND_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="cec-chip-clear"
              onClick={() => removeCriterion(idx)}
              disabled={disabled}
              aria-label={`Remove criterion ${idx + 1}`}
              data-testid={`genesis-criterion-remove-${idx}`}
            >
              <X size={12} aria-hidden />
            </button>
          </div>
        ))}
        <button
          type="button"
          className="cec-btn"
          onClick={addCriterion}
          disabled={disabled}
          data-testid="genesis-criterion-add"
        >
          <Plus size={14} aria-hidden />
          Add a success criterion
        </button>
      </div>
    </div>
  );
}
