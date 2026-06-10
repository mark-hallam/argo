"use client";

import { useCallback, useState } from "react";
import { Plus, ShieldCheck, SlidersHorizontal, Lock, X } from "lucide-react";
import { BoundariesPanelStyles } from "./BoundariesPanel.styles";

/**
 * Genesis Phase-2 "Set boundaries" panel — visual access-control + guardrails
 * specification (mission-genesis-access-controls-and-guardrails-spec-ui).
 *
 * A real teammate gets access boundaries (what data it may touch), approval
 * thresholds (what dollar actions need a human), and escalation rules (which
 * questions go to a person). This panel makes that explicit and visual —
 * toggles, a slider, free-text topics — instead of trusting Genesis to infer
 * it all from prose. The selection threads into the SSE request; the gateway
 * synthesizes an enforceable truth-boundary policy from it
 * (`argo_genesis::policy_synth`) and echoes back what it persisted.
 *
 * Two modes:
 *  - EDIT (default): `value` + `onChange` drive the toggles/slider/topics.
 *  - APPLIED (`applied` set): a read-only render of the server's `vertical_ready`
 *    boundaries echo — what was ACTUALLY synthesized + persisted, NOT the local
 *    form (no-theater, per `feedback_no_theater_in_demo`).
 *
 * Consumes ONLY the locked brand tokens from `globals.css` (purple→PINK
 * gradient, Inter, `--brand-ink`) — no color literals, no new design (DEC-027).
 * Follows the `CadenceExceptionsCapture` panel pattern; does NOT fork it.
 */

/** A data category the operator scopes. Wire tokens mirror
 *  `argo_genesis::policy_synth::DataCategory` (snake_case). */
export type DataCategory =
  | "customer_pii"
  | "financial"
  | "contracts"
  | "source_code";

/** Access level for a category. Mirrors `policy_synth::AccessLevel`. */
export type AccessLevel = "read_only" | "read_write";

/** One access-scope toggle. Mirrors `policy_synth::AccessScope`. */
export interface AccessScope {
  category: DataCategory;
  access: AccessLevel;
}

/** The full boundaries spec posted on the genesis request. Mirrors
 *  `argo_genesis::policy_synth::BoundariesSpec` (snake_case wire). */
export interface BoundariesSpec {
  access_scopes: AccessScope[];
  /** Dollar ceiling above which money actions need sign-off; `null` = none. */
  approval_threshold_usd: number | null;
  /** Free-text "must escalate questions about X" topics. */
  escalation_topics: string[];
}

/** The server-truth echo carried on `vertical_ready` (mirrors
 *  `policy_synth::boundaries_echo`). */
export interface BoundariesEcho {
  policy_id: string;
  gated_classes: string[];
  approval_threshold_usd: number | null;
  escalation_topics: string[];
  access_scopes: { category: string; access: string }[];
}

const CATEGORIES: ReadonlyArray<{ value: DataCategory; label: string }> = [
  { value: "customer_pii", label: "Customer PII" },
  { value: "financial", label: "Financial" },
  { value: "contracts", label: "Contracts" },
  { value: "source_code", label: "Source code" },
];

/** The default ceiling shown when the operator first enables the slider. */
const DEFAULT_THRESHOLD_USD = 500;

/** The empty selection a parent seeds state with — every category read-only
 *  (conservative), no dollar ceiling, no topics. */
export const EMPTY_BOUNDARIES: BoundariesSpec = {
  access_scopes: CATEGORIES.map((c) => ({
    category: c.value,
    access: "read_only" as AccessLevel,
  })),
  approval_threshold_usd: null,
  escalation_topics: [],
};

/** Deterministic client-side pre-fill mirroring
 *  `argo_genesis::policy_synth::smart_defaults` — keyword-mapped, no LLM. Lets
 *  the panel suggest sensible boundaries from the role text. */
export function boundariesSmartDefaults(roleText: string): BoundariesSpec {
  const t = roleText.toLowerCase();
  const scopes = (pii: AccessLevel): AccessScope[] => [
    { category: "customer_pii", access: pii },
    { category: "financial", access: "read_only" },
    { category: "contracts", access: "read_only" },
    { category: "source_code", access: "read_only" },
  ];
  if (
    t.includes("customer success") ||
    t.includes("customer support") ||
    t.includes("support")
  ) {
    return {
      access_scopes: scopes("read_write"),
      approval_threshold_usd: 500,
      escalation_topics: ["legal", "compliance"],
    };
  }
  if (t.includes("sales") || t.includes("sdr") || t.includes("account executive")) {
    return {
      access_scopes: scopes("read_write"),
      approval_threshold_usd: 1000,
      escalation_topics: ["legal", "pricing"],
    };
  }
  if (t.includes("marketplace") || t.includes("seller")) {
    return {
      access_scopes: scopes("read_only"),
      approval_threshold_usd: 250,
      escalation_topics: ["legal", "refunds"],
    };
  }
  return {
    access_scopes: scopes("read_only"),
    approval_threshold_usd: null,
    escalation_topics: ["legal"],
  };
}

export interface BoundariesPanelProps {
  /** Current selection (EDIT mode). */
  value?: BoundariesSpec;
  /** Replace the selection (EDIT mode). */
  onChange?(next: BoundariesSpec): void;
  /** Disabled while a generation is in flight. */
  disabled?: boolean;
  /** When set, render the read-only server echo instead of the editor. */
  applied?: BoundariesEcho | null;
}

export function BoundariesPanel({
  value,
  onChange,
  disabled,
  applied,
}: BoundariesPanelProps) {
  const [topicDraft, setTopicDraft] = useState("");

  const spec = value ?? EMPTY_BOUNDARIES;

  const setAccess = useCallback(
    (category: DataCategory, access: AccessLevel) => {
      if (!onChange) return;
      onChange({
        ...spec,
        access_scopes: spec.access_scopes.map((s) =>
          s.category === category ? { ...s, access } : s,
        ),
      });
    },
    [onChange, spec],
  );

  const setThresholdEnabled = useCallback(
    (enabled: boolean) => {
      if (!onChange) return;
      onChange({
        ...spec,
        approval_threshold_usd: enabled ? DEFAULT_THRESHOLD_USD : null,
      });
    },
    [onChange, spec],
  );

  const setThreshold = useCallback(
    (amount: number) => {
      if (!onChange) return;
      onChange({ ...spec, approval_threshold_usd: amount });
    },
    [onChange, spec],
  );

  const addTopic = useCallback(() => {
    if (!onChange) return;
    const trimmed = topicDraft.trim();
    if (!trimmed) return;
    onChange({ ...spec, escalation_topics: [...spec.escalation_topics, trimmed] });
    setTopicDraft("");
  }, [onChange, spec, topicDraft]);

  const removeTopic = useCallback(
    (idx: number) => {
      if (!onChange) return;
      onChange({
        ...spec,
        escalation_topics: spec.escalation_topics.filter((_, i) => i !== idx),
      });
    },
    [onChange, spec],
  );

  if (applied) {
    return <BoundariesApplied echo={applied} />;
  }

  const thresholdEnabled = spec.approval_threshold_usd !== null;

  return (
    <div className="bp" data-testid="genesis-boundaries">
      <BoundariesPanelStyles />
      <span className="bp-title">
        <ShieldCheck size={16} className="bp-icon" aria-hidden />
        Set boundaries
      </span>
      <p className="bp-hint">
        What this teammate may touch, what dollar actions need your sign-off, and
        which questions it must escalate to you. Skycot enforces these at runtime.
      </p>

      {/* Access scope — what the teammate may touch. */}
      <div className="bp-section">
        <span className="bp-section-label">
          <Lock size={14} className="bp-icon" aria-hidden />
          Data access
        </span>
        {CATEGORIES.map((cat) => {
          const current =
            spec.access_scopes.find((s) => s.category === cat.value)?.access ??
            "read_only";
          return (
            <div
              className="bp-access-row"
              key={cat.value}
              data-testid={`genesis-boundary-access-${cat.value}`}
            >
              <span className="bp-access-label">{cat.label}</span>
              <div className="bp-seg">
                {(["read_only", "read_write"] as AccessLevel[]).map((lvl) => (
                  <button
                    key={lvl}
                    type="button"
                    className={`bp-seg-btn${current === lvl ? " is-active" : ""}`}
                    disabled={disabled}
                    aria-pressed={current === lvl}
                    data-testid={`genesis-boundary-access-${cat.value}-${lvl}`}
                    onClick={() => setAccess(cat.value, lvl)}
                  >
                    {lvl === "read_only" ? "Read-only" : "Read-write"}
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>

      {/* Approval threshold — what dollar actions need a human. */}
      <div className="bp-section">
        <span className="bp-section-label">
          <SlidersHorizontal size={14} className="bp-icon" aria-hidden />
          Approval threshold
        </span>
        <label className="bp-check">
          <input
            type="checkbox"
            checked={thresholdEnabled}
            disabled={disabled}
            data-testid="genesis-boundary-threshold-enabled"
            onChange={(e) => setThresholdEnabled(e.target.checked)}
          />
          Require my sign-off for money actions (refunds, sends) above an amount
        </label>
        {thresholdEnabled ? (
          <div className="bp-slider-row">
            <input
              type="range"
              min={0}
              max={5000}
              step={50}
              value={spec.approval_threshold_usd ?? DEFAULT_THRESHOLD_USD}
              disabled={disabled}
              aria-label="Approval threshold (USD)"
              data-testid="genesis-boundary-threshold-slider"
              onChange={(e) => setThreshold(Number(e.target.value))}
            />
            <span className="bp-amount" data-testid="genesis-boundary-threshold-amount">
              ${spec.approval_threshold_usd ?? DEFAULT_THRESHOLD_USD}
            </span>
          </div>
        ) : null}
      </div>

      {/* Escalation topics — which questions go to a person. */}
      <div className="bp-section">
        <span className="bp-section-label">
          <ShieldCheck size={14} className="bp-icon" aria-hidden />
          Always escalate questions about…
        </span>
        {spec.escalation_topics.length > 0 ? (
          <div className="bp-chips" data-testid="genesis-boundary-topic-chips">
            {spec.escalation_topics.map((topic, idx) => (
              <span className="bp-chip" key={`${topic}-${idx}`}>
                <span className="bp-chip-text" title={topic}>
                  {topic}
                </span>
                <button
                  type="button"
                  className="bp-chip-clear"
                  onClick={() => removeTopic(idx)}
                  disabled={disabled}
                  aria-label={`Remove topic: ${topic}`}
                  data-testid={`genesis-boundary-topic-remove-${idx}`}
                >
                  <X size={12} aria-hidden />
                </button>
              </span>
            ))}
          </div>
        ) : null}
        <div className="bp-row">
          <input
            type="text"
            className="bp-input bp-grow"
            placeholder="legal, compliance, pricing…"
            value={topicDraft}
            disabled={disabled}
            data-testid="genesis-boundary-topic-input"
            onChange={(e) => setTopicDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addTopic();
              }
            }}
          />
          <button
            type="button"
            className="bp-btn"
            onClick={addTopic}
            disabled={disabled || topicDraft.trim().length === 0}
            data-testid="genesis-boundary-topic-add"
          >
            <Plus size={14} aria-hidden />
            Add
          </button>
        </div>
      </div>
    </div>
  );
}

/** Read-only render of the server's synthesized boundaries echo. */
function BoundariesApplied({ echo }: { echo: BoundariesEcho }) {
  return (
    <div className="bp" data-testid="genesis-boundaries-applied">
      <BoundariesPanelStyles />
      <span className="bp-title">
        <ShieldCheck size={16} className="bp-icon" aria-hidden />
        Enforced boundaries
      </span>
      <p className="bp-hint">
        Skycot synthesized this truth-boundary policy from what you set and will
        enforce it at runtime. A declared claim in a gated class is held for your
        approval.
      </p>
      <div className="bp-section">
        <span className="bp-section-label">Gated claim classes</span>
        {echo.gated_classes.length > 0 ? (
          <div className="bp-chips">
            {echo.gated_classes.map((cls, idx) => (
              <span
                className="bp-chip"
                key={`${cls}-${idx}`}
                data-testid={`genesis-boundary-applied-class-${idx}`}
              >
                <span className="bp-chip-text">{cls}</span>
              </span>
            ))}
          </div>
        ) : (
          <p className="bp-hint">No gated classes — this teammate has no claim boundaries.</p>
        )}
      </div>
      {echo.approval_threshold_usd !== null ? (
        <div className="bp-section">
          <span className="bp-section-label">Approval threshold</span>
          <span className="bp-amount" data-testid="genesis-boundary-applied-threshold">
            Money actions above ${echo.approval_threshold_usd} need your sign-off
          </span>
        </div>
      ) : null}
    </div>
  );
}
