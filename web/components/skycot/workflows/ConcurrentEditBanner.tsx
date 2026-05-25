"use client";
import { AlertTriangle, GitCompare, RotateCcw, X } from "lucide-react";

interface Props {
  /**
   * Current head version the gateway has on disk (the user's view is
   * stale relative to this number). Rendered in the banner copy so the
   * operator knows how far behind they are.
   */
  currentVersion: number;
  /**
   * Triggered when the operator picks "Refresh" — the parent reloads
   * the workflow from the gateway, discarding the user's dirty edits.
   * The component does NOT discard dirty state itself; the parent owns
   * that decision (and may show its own confirmation).
   */
  onRefresh: () => void;
  /**
   * Triggered when the operator picks "Show diff" — for v1 this is a
   * stub that opens the current head's read-only view in a new tab.
   * A real diff UI lands as a v1.x follow-up.
   */
  onShowDiff: () => void;
  /**
   * Dismiss the banner without action (rare — usually the user picks
   * Refresh or Show diff). Clears the conflict store so a subsequent
   * save attempt can surface a fresh conflict cleanly.
   */
  onDismiss: () => void;
}

/**
 * Concurrent-edit conflict banner — surfaces when the gateway rejects
 * a save with `409 Conflict` because the workflow head has advanced
 * past the user's loaded version. Sits below the toolbar (similar to
 * `UnsavedBanner`) so it's visible without obscuring the canvas.
 *
 * Two CTAs match the v1 procurement-grade contract:
 *   - Refresh (loses my changes) — reload from the live head.
 *   - Show diff vs current head — for v1 opens the read-only viewer
 *     of the current head in a new tab; a proper inline diff is
 *     scoped to a v1.x follow-up.
 */
export function ConcurrentEditBanner({
  currentVersion,
  onRefresh,
  onShowDiff,
  onDismiss,
}: Props) {
  return (
    <div
      className="builder-conflict-banner"
      data-testid="concurrent-edit-banner"
      role="alert"
      aria-live="assertive"
    >
      <div className="builder-conflict-banner-message">
        <AlertTriangle size={15} aria-hidden="true" />
        <span>
          Someone else just edited this workflow — current version is v
          {currentVersion}. Refresh to see their changes, or compare against
          your draft.
        </span>
      </div>
      <div className="builder-conflict-banner-actions">
        <button
          type="button"
          className="builder-conflict-banner-btn builder-conflict-banner-diff"
          onClick={onShowDiff}
          data-testid="concurrent-edit-banner-diff"
        >
          <GitCompare size={13} aria-hidden="true" /> Show diff vs current head
        </button>
        <button
          type="button"
          className="builder-conflict-banner-btn builder-conflict-banner-refresh"
          onClick={onRefresh}
          data-testid="concurrent-edit-banner-refresh"
        >
          <RotateCcw size={13} aria-hidden="true" /> Refresh (loses my changes)
        </button>
        <button
          type="button"
          className="builder-conflict-banner-dismiss"
          onClick={onDismiss}
          data-testid="concurrent-edit-banner-dismiss"
          aria-label="Dismiss conflict banner"
        >
          <X size={13} aria-hidden="true" />
        </button>
      </div>
      <style jsx>{`
        .builder-conflict-banner {
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 10px 28px;
          background: linear-gradient(180deg, #fef2f2 0%, #fff1f2 100%);
          border-bottom: 1px solid #fca5a5;
          color: #7f1d1d;
          font-family: "Inter", sans-serif;
          animation: builder-conflict-banner-slide 180ms var(--motion-ease);
        }
        .builder-conflict-banner-message {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 13.5px;
          font-weight: 600;
          flex: 1 1 auto;
        }
        .builder-conflict-banner-actions {
          margin-left: auto;
          display: flex;
          gap: 8px;
          align-items: center;
        }
        .builder-conflict-banner-btn {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 6px 12px;
          border-radius: 8px;
          font-family: "Inter", sans-serif;
          font-size: 12.5px;
          font-weight: 600;
          cursor: pointer;
        }
        .builder-conflict-banner-diff {
          background: white;
          color: #7f1d1d;
          border: 1px solid #fca5a5;
        }
        .builder-conflict-banner-diff:hover {
          background: #fff1f2;
        }
        .builder-conflict-banner-refresh {
          background: linear-gradient(135deg, #dc2626, #b91c1c);
          color: white;
          border: none;
          box-shadow: 0 2px 8px -2px rgba(220, 38, 38, 0.4);
        }
        .builder-conflict-banner-refresh:hover {
          filter: brightness(1.04);
        }
        .builder-conflict-banner-dismiss {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          background: transparent;
          color: #7f1d1d;
          border: none;
          border-radius: 6px;
          padding: 4px;
          cursor: pointer;
        }
        .builder-conflict-banner-dismiss:hover {
          background: rgba(127, 29, 29, 0.08);
        }
        @keyframes builder-conflict-banner-slide {
          from {
            transform: translateY(-100%);
            opacity: 0;
          }
          to {
            transform: translateY(0);
            opacity: 1;
          }
        }
        @media (prefers-reduced-motion: reduce) {
          .builder-conflict-banner {
            animation: none;
          }
        }
      `}</style>
    </div>
  );
}
