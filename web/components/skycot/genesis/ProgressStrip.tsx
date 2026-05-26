"use client";

import { Check, Loader2 } from "lucide-react";
import { GENESIS_PHASES, type GenesisPhase } from "@/lib/api/genesis";

/**
 * Live progress strip for /genesis — 5 phase pills that flip from
 * pending → active → done as the SSE stream advances. The phase order
 * + ids mirror the Rust handler's `write_progress(...)` call order in
 * `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs`.
 */

export interface ProgressStripProps {
  /** Highest phase the stream has reached so far. `null` before start. */
  activePhase: GenesisPhase | null;
  /** True when an `error` event fired — freezes the strip and reds it. */
  hasError: boolean;
}

const PHASE_LABEL: Record<GenesisPhase, string> = {
  drafting_persona: "Drafting persona",
  compiling_charter: "Compiling charter",
  generating_skill_bundle: "Generating skill bundle",
  validating_eval_seeds: "Validating eval seeds",
  ready: "Ready",
};

type PhaseState = "pending" | "active" | "done" | "error";

function phaseState(
  phase: GenesisPhase,
  active: GenesisPhase | null,
  hasError: boolean,
): PhaseState {
  if (active === null) return "pending";
  const activeIdx = GENESIS_PHASES.indexOf(active);
  const thisIdx = GENESIS_PHASES.indexOf(phase);
  if (hasError && thisIdx === activeIdx) return "error";
  if (thisIdx < activeIdx) return "done";
  if (thisIdx === activeIdx) {
    // `ready` is terminal — render it as done once we hit it.
    return phase === "ready" ? "done" : "active";
  }
  return "pending";
}

export function ProgressStrip({ activePhase, hasError }: ProgressStripProps) {
  return (
    <>
      <style jsx>{`
        .gen-progress {
          max-width: 880px;
          margin: 28px auto 0;
          padding: 0 24px;
          display: flex;
          gap: 10px;
          flex-wrap: wrap;
        }
        .gen-progress-pill {
          flex: 1 1 160px;
          min-width: 160px;
          padding: 14px 16px;
          border-radius: 14px;
          background: rgba(255, 255, 255, 0.85);
          border: 1px solid #ece9f5;
          display: flex;
          align-items: center;
          gap: 10px;
          transition: border-color 0.2s, background 0.2s, opacity 0.2s;
        }
        .gen-progress-pill.is-pending {
          opacity: 0.55;
        }
        .gen-progress-pill.is-active {
          border-color: #8b5cf6;
          background: rgba(139, 92, 246, 0.06);
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.08);
        }
        .gen-progress-pill.is-done {
          border-color: #c4b5fd;
          background: rgba(139, 92, 246, 0.04);
        }
        .gen-progress-pill.is-error {
          border-color: #b91c1c;
          background: rgba(185, 28, 28, 0.06);
          opacity: 1;
        }
        .gen-progress-icon {
          width: 28px;
          height: 28px;
          border-radius: 999px;
          display: grid;
          place-items: center;
          flex-shrink: 0;
          background: #ece9f5;
          color: #6d28d9;
        }
        .gen-progress-pill.is-active .gen-progress-icon {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
        }
        .gen-progress-pill.is-done .gen-progress-icon {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
        }
        .gen-progress-pill.is-error .gen-progress-icon {
          background: #fee2e2;
          color: #b91c1c;
        }
        .gen-progress-label {
          font-family: "Inter", sans-serif;
          font-size: 13.5px;
          font-weight: 600;
          color: #1e1b4b;
          letter-spacing: -0.005em;
        }
        .gen-progress-pill.is-pending .gen-progress-label {
          color: #52525b;
        }
        .gen-spin {
          animation: gen-spin 1s linear infinite;
        }
        @keyframes gen-spin {
          to {
            transform: rotate(360deg);
          }
        }
        @media (prefers-reduced-motion: reduce) {
          .gen-spin {
            animation: none;
          }
        }
      `}</style>
      <ol
        className="gen-progress"
        data-testid="genesis-progress-strip"
        aria-label="Genesis progress"
      >
        {GENESIS_PHASES.map((phase) => {
          const state = phaseState(phase, activePhase, hasError);
          return (
            <li
              key={phase}
              data-testid={`genesis-phase-${phase}`}
              data-state={state}
              className={`gen-progress-pill is-${state}`}
            >
              <span className="gen-progress-icon" aria-hidden>
                {state === "done" ? (
                  <Check size={16} strokeWidth={3} />
                ) : state === "active" ? (
                  <Loader2 size={16} className="gen-spin" />
                ) : state === "error" ? (
                  <span style={{ fontWeight: 700 }}>!</span>
                ) : (
                  <span style={{ fontSize: 11, fontWeight: 700 }}>
                    {GENESIS_PHASES.indexOf(phase) + 1}
                  </span>
                )}
              </span>
              <span className="gen-progress-label">{PHASE_LABEL[phase]}</span>
            </li>
          );
        })}
      </ol>
    </>
  );
}
