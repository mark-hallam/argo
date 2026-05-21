"use client";

import { useState } from "react";
import { ChevronDown, ChevronUp, CheckCircle2, XCircle, Clock, X } from "lucide-react";
import type { RunPhase, RunSummary, StepRunState } from "./useTestRun";

interface StepDescriptor {
  id: string;
  name: string;
  kind: string;
}

interface RunOverlayProps {
  phase: RunPhase;
  steps: StepDescriptor[];
  stepStates: Record<string, StepRunState>;
  summary: RunSummary | null;
  transportError: string | null;
  onDismiss: () => void;
}


function formatMs(ms: number | undefined): string {
  if (ms == null) return "—";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatCost(cost: number | undefined): string {
  if (cost == null) return "—";
  return `$${cost.toFixed(cost < 0.01 ? 4 : 3)}`;
}

export function RunOverlay({
  phase,
  steps,
  stepStates,
  summary,
  transportError,
  onDismiss,
}: RunOverlayProps) {
  const [expanded, setExpanded] = useState<boolean>(true);

  if (phase === "idle") return null;

  const finished = phase === "complete" || phase === "failed";
  const okCount = summary?.ok ?? 0;
  const failCount = summary?.fail ?? 0;
  const isFailure = phase === "failed" || failCount > 0 || !!transportError;

  return (
    <>
      <style jsx global>{`
        .run-banner {
          position: absolute;
          top: 12px;
          left: 12px;
          right: 12px;
          padding: 14px 18px;
          border-radius: 12px;
          display: flex;
          gap: 14px;
          align-items: center;
          font-family: "Inter", sans-serif;
          box-shadow: 0 8px 20px -10px rgba(0, 0, 0, 0.15);
          z-index: 6;
        }
        .run-banner-running {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.1), rgba(236, 72, 153, 0.08));
          border: 1px solid rgba(139, 92, 246, 0.3);
          color: #1e1b4b;
        }
        .run-banner-success {
          background: rgba(16, 185, 129, 0.08);
          border: 1px solid rgba(16, 185, 129, 0.35);
          color: #064e3b;
        }
        .run-banner-failure {
          background: rgba(244, 63, 94, 0.08);
          border: 1px solid rgba(244, 63, 94, 0.35);
          color: #7f1d1d;
        }
        .run-banner-icon {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          display: grid;
          place-items: center;
          color: white;
          flex-shrink: 0;
        }
        .run-banner-icon-running {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
        }
        .run-banner-icon-success {
          background: #10b981;
        }
        .run-banner-icon-failure {
          background: #f43f5e;
        }
        .run-banner-title {
          font-weight: 700;
          font-size: 14px;
          margin: 0;
        }
        .run-banner-sub {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          opacity: 0.75;
          margin-top: 2px;
          letter-spacing: 0.04em;
        }
        .run-banner-close {
          margin-left: auto;
          background: transparent;
          border: none;
          color: inherit;
          opacity: 0.6;
          cursor: pointer;
          padding: 4px;
          border-radius: 6px;
          display: grid;
          place-items: center;
        }
        .run-banner-close:hover { opacity: 1; background: rgba(0,0,0,0.06); }
        .run-tray {
          position: absolute;
          left: 12px;
          right: 12px;
          bottom: 12px;
          background: white;
          border: 1px solid #ece9f5;
          border-radius: 14px;
          box-shadow: 0 16px 40px -16px rgba(30, 27, 75, 0.25);
          font-family: "Inter", sans-serif;
          z-index: 5;
          max-height: 50vh;
          display: flex;
          flex-direction: column;
        }
        .run-tray-head {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 12px 18px;
          border-bottom: 1px solid #ece9f5;
        }
        .run-tray-head-title {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
          color: #1e1b4b;
        }
        .run-tray-counts {
          display: flex;
          gap: 14px;
          margin-left: auto;
          font-size: 12px;
          color: #3f3f46;
        }
        .run-tray-count-num {
          font-family: "JetBrains Mono", monospace;
          font-weight: 700;
          color: #1e1b4b;
          margin-right: 4px;
        }
        .run-tray-toggle {
          background: transparent;
          border: 1px solid #ece9f5;
          color: #3f3f46;
          padding: 4px 8px;
          border-radius: 8px;
          cursor: pointer;
          display: inline-flex;
          align-items: center;
          gap: 4px;
          font-size: 11px;
          font-family: "JetBrains Mono", monospace;
          font-weight: 600;
        }
        .run-tray-toggle:hover { color: #8b5cf6; border-color: #8b5cf6; }
        .run-tray-body {
          overflow-y: auto;
          padding: 6px 0;
        }
        .run-tray-row {
          display: grid;
          grid-template-columns: 28px 1fr 90px 90px 110px;
          align-items: center;
          gap: 12px;
          padding: 10px 18px;
          border-bottom: 1px solid #f4f4f8;
          font-size: 12.5px;
        }
        .run-tray-row:last-child { border-bottom: none; }
        .run-tray-status-icon {
          width: 22px;
          height: 22px;
          border-radius: 6px;
          display: grid;
          place-items: center;
          color: white;
        }
        .run-tray-status-pending { background: #d4d4d8; }
        .run-tray-status-running {
          background: linear-gradient(135deg, #f59e0b, #fbbf24);
          animation: run-pulse 1.1s ease-in-out infinite;
        }
        .run-tray-status-success { background: #10b981; }
        .run-tray-status-error { background: #f43f5e; }
        @keyframes run-pulse {
          0%, 100% { transform: scale(1); opacity: 1; }
          50% { transform: scale(1.12); opacity: 0.82; }
        }
        .run-tray-step-name { font-weight: 600; color: #1e1b4b; }
        .run-tray-step-meta {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #52525b;
          letter-spacing: 0.04em;
          text-align: right;
        }
        .run-tray-response {
          grid-column: 2 / -1;
          margin-top: 4px;
          padding: 8px 10px;
          background: #fafaff;
          border: 1px solid #ece9f5;
          border-radius: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #3f3f46;
          line-height: 1.5;
        }
        .run-tray-error {
          grid-column: 2 / -1;
          margin-top: 4px;
          padding: 8px 10px;
          background: rgba(244, 63, 94, 0.08);
          border: 1px solid rgba(244, 63, 94, 0.3);
          border-radius: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #7f1d1d;
          line-height: 1.5;
        }
      `}</style>

      <div
        className={
          "run-banner " +
          (phase === "running"
            ? "run-banner-running"
            : isFailure
              ? "run-banner-failure"
              : "run-banner-success")
        }
        data-testid="run-summary-banner"
      >
        <div
          className={
            "run-banner-icon " +
            (phase === "running"
              ? "run-banner-icon-running"
              : isFailure
                ? "run-banner-icon-failure"
                : "run-banner-icon-success")
          }
        >
          {phase === "running" ? (
            <Clock size={15} />
          ) : isFailure ? (
            <XCircle size={15} />
          ) : (
            <CheckCircle2 size={15} />
          )}
        </div>
        <div>
          <p className="run-banner-title">
            {phase === "running"
              ? "Test run in progress…"
              : isFailure
                ? transportError
                  ? "Test run could not start."
                  : `Test run failed. ${okCount}/${steps.length} steps succeeded, ${failCount} failed.`
                : `Test run complete. ${okCount}/${steps.length} steps succeeded.`}
          </p>
          {finished && summary ? (
            <p className="run-banner-sub">
              TOTAL {formatMs(summary.durationMs)} · {formatCost(summary.costUsd)}
              {summary.tokens != null ? ` · ${summary.tokens} TOK` : ""}
            </p>
          ) : transportError ? (
            <p className="run-banner-sub">{transportError}</p>
          ) : null}
        </div>
        {finished ? (
          <button
            type="button"
            className="run-banner-close"
            onClick={onDismiss}
            aria-label="Dismiss test run"
          >
            <X size={14} />
          </button>
        ) : null}
      </div>

      <div className="run-tray" data-testid="run-details-tray">
        <div className="run-tray-head">
          <span className="run-tray-head-title">Run details</span>
          <div className="run-tray-counts">
            <span>
              <span className="run-tray-count-num">{okCount}</span>ok
            </span>
            <span>
              <span className="run-tray-count-num">{failCount}</span>fail
            </span>
            <span>
              <span className="run-tray-count-num">
                {summary ? formatMs(summary.durationMs) : "…"}
              </span>
              elapsed
            </span>
          </div>
          <button
            type="button"
            className="run-tray-toggle"
            onClick={() => setExpanded((v) => !v)}
            aria-label={expanded ? "Collapse run details" : "Expand run details"}
          >
            {expanded ? <ChevronDown size={12} /> : <ChevronUp size={12} />}
            {expanded ? "HIDE" : "SHOW"}
          </button>
        </div>
        {expanded ? (
          <div className="run-tray-body">
            {steps.map((step) => {
              const state = stepStates[step.id] ?? { status: "pending" };
              return (
                <div key={step.id} className="run-tray-row">
                  <div
                    className={`run-tray-status-icon run-tray-status-${state.status}`}
                  >
                    {state.status === "success" ? (
                      <CheckCircle2 size={13} />
                    ) : state.status === "error" ? (
                      <XCircle size={13} />
                    ) : state.status === "running" ? (
                      <Clock size={12} />
                    ) : null}
                  </div>
                  <div>
                    <div className="run-tray-step-name">{step.name}</div>
                  </div>
                  <div className="run-tray-step-meta">{formatMs(state.durationMs)}</div>
                  <div className="run-tray-step-meta">
                    {state.tokens != null ? `${state.tokens} TOK` : "—"}
                  </div>
                  <div className="run-tray-step-meta">
                    {state.status.toUpperCase()}
                  </div>
                  {state.response ? (
                    <div className="run-tray-response">{state.response}</div>
                  ) : null}
                  {state.error ? (
                    <div className="run-tray-error">{state.error}</div>
                  ) : null}
                </div>
              );
            })}
          </div>
        ) : null}
      </div>
    </>
  );
}
