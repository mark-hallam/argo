"use client";

import { CheckCircle2, XCircle } from "lucide-react";

import type { TestRunHistoryEntry } from "@/lib/hooks/use-test-run-history";

interface TestRunHistoryPanelProps {
  entries: TestRunHistoryEntry[];
  /**
   * Re-opens the RunOverlay against a previously-completed run. The
   * page wires this to a `replayEntry` slot it passes into RunOverlay
   * so the same overlay component renders the historical run.
   */
  onView: (entry: TestRunHistoryEntry) => void;
}

function formatMs(ms: number | undefined): string {
  if (ms == null) return "—";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatRelative(epochMs: number): string {
  const delta = Math.max(0, Date.now() - epochMs);
  const sec = Math.floor(delta / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}

function formatAbsolute(epochMs: number): string {
  try {
    return new Date(epochMs).toLocaleString();
  } catch {
    return new Date(epochMs).toISOString();
  }
}

export function TestRunHistoryPanel({
  entries,
  onView,
}: TestRunHistoryPanelProps) {
  return (
    <section
      className="run-history-panel"
      aria-label="Recent test runs"
      data-testid="run-history-panel"
    >
      <style jsx>{`
        .run-history-panel {
          background: white;
          border: 1px solid #ece9f5;
          border-radius: 14px;
          font-family: "Inter", sans-serif;
          padding: 14px 18px 8px;
          box-shadow: 0 4px 14px -10px rgba(30, 27, 75, 0.18);
        }
        .run-history-head {
          display: flex;
          align-items: baseline;
          gap: 10px;
          margin-bottom: 8px;
        }
        .run-history-title {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
          color: #1e1b4b;
        }
        .run-history-count {
          font-family: var(--font-mono);
          font-size: 11px;
          color: #6b6b76;
        }
        .run-history-empty {
          padding: 18px 4px 22px;
          color: #6b6b76;
          font-size: 13px;
          text-align: center;
          line-height: 1.5;
        }
        .run-history-empty strong {
          color: #1e1b4b;
          font-weight: 600;
        }
        .run-history-row {
          display: grid;
          grid-template-columns: 28px 1fr 110px 90px 70px;
          align-items: center;
          gap: 12px;
          padding: 10px 0;
          border-top: 1px solid #f4f4f8;
          font-size: 12.5px;
        }
        .run-history-row:first-of-type {
          border-top: none;
        }
        .run-history-status-icon {
          width: 22px;
          height: 22px;
          border-radius: 6px;
          display: grid;
          place-items: center;
          color: white;
        }
        .run-history-status-complete {
          background: #10b981;
        }
        .run-history-status-failed {
          background: #f43f5e;
        }
        .run-history-summary {
          font-weight: 600;
          color: #1e1b4b;
        }
        .run-history-summary-sub {
          font-family: var(--font-mono);
          font-size: 11px;
          color: #6b6b76;
          margin-top: 2px;
        }
        .run-history-meta {
          font-family: var(--font-mono);
          font-size: 11px;
          color: #52525b;
          letter-spacing: 0.04em;
        }
        .run-history-time {
          color: #3f3f46;
          font-size: 12px;
        }
        .run-history-view {
          background: transparent;
          border: 1px solid #ece9f5;
          color: #3f3f46;
          padding: 4px 10px;
          border-radius: 8px;
          cursor: pointer;
          font-size: 11px;
          font-family: var(--font-mono);
          font-weight: 600;
          letter-spacing: 0.06em;
          text-transform: uppercase;
        }
        .run-history-view:hover {
          color: #8b5cf6;
          border-color: #8b5cf6;
        }
        .run-history-view:focus-visible {
          outline: 2px solid #8b5cf6;
          outline-offset: 2px;
        }
      `}</style>
      <div className="run-history-head">
        <span className="run-history-title">Recent test runs</span>
        {entries.length > 0 ? (
          <span className="run-history-count">
            {entries.length} {entries.length === 1 ? "run" : "runs"}
          </span>
        ) : null}
      </div>
      {entries.length === 0 ? (
        <div className="run-history-empty" data-testid="run-history-empty">
          <strong>No test runs yet.</strong>
          <br />
          Click <strong>Test run</strong> in the toolbar to start one.
        </div>
      ) : (
        entries.map((entry) => {
          const summary = entry.summary;
          const stepCount = entry.steps.length;
          const okCount = summary?.ok ?? 0;
          const failCount = summary?.fail ?? 0;
          const isFailed = entry.status === "failed";
          const headline = entry.transportError
            ? "Run could not start"
            : isFailed
              ? `${okCount}/${stepCount} ok · ${failCount} failed`
              : `${okCount}/${stepCount} steps succeeded`;
          return (
            <div
              key={entry.id}
              className="run-history-row"
              data-testid="run-history-row"
            >
              <div
                className={`run-history-status-icon run-history-status-${entry.status}`}
                aria-label={entry.status}
              >
                {isFailed ? (
                  <XCircle size={13} />
                ) : (
                  <CheckCircle2 size={13} />
                )}
              </div>
              <div>
                <div className="run-history-summary">{headline}</div>
                {entry.transportError ? (
                  <div className="run-history-summary-sub">
                    {entry.transportError}
                  </div>
                ) : null}
              </div>
              <div
                className="run-history-time"
                title={formatAbsolute(entry.endedAt)}
              >
                {formatRelative(entry.endedAt)}
              </div>
              <div className="run-history-meta">
                {formatMs(summary?.durationMs)}
              </div>
              <button
                type="button"
                className="run-history-view"
                onClick={() => onView(entry)}
                aria-label={`View test run from ${formatAbsolute(entry.endedAt)}`}
              >
                View
              </button>
            </div>
          );
        })
      )}
    </section>
  );
}
