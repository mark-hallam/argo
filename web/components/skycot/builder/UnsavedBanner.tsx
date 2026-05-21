"use client";
import { AlertCircle, RotateCcw, Save } from "lucide-react";

interface Props {
  dirty: boolean;
  onSave: () => void;
  onDiscard: () => void;
}

/**
 * Unsaved-changes banner for the visual workflow builder.
 *
 * Renders at the top of the canvas region only while
 * `builder.dirty === true`. Slide-down entry animation is guarded by
 * `prefers-reduced-motion: reduce`, so users with the OS setting see
 * the banner appear without movement.
 *
 * Save is wired to the existing save-publish toast stub until
 * `web-workflow-builder-ai-prompt-wired` (#1882) lands the live
 * gateway path. Discard reverts the canvas to the initial seed via
 * `useWorkflowBuilderState.reset()` exposed at #1944.
 */
export function UnsavedBanner({ dirty, onSave, onDiscard }: Props) {
  if (!dirty) return null;

  return (
    <div
      className="builder-unsaved-banner"
      data-testid="unsaved-banner"
      role="status"
      aria-live="polite"
    >
      <div className="builder-unsaved-banner-message">
        <AlertCircle size={15} aria-hidden="true" />
        <span>You have unsaved changes</span>
      </div>
      <div className="builder-unsaved-banner-actions">
        <button
          type="button"
          className="builder-unsaved-banner-btn builder-unsaved-banner-discard"
          onClick={onDiscard}
          data-testid="unsaved-banner-discard"
        >
          <RotateCcw size={13} aria-hidden="true" /> Discard
        </button>
        <button
          type="button"
          className="builder-unsaved-banner-btn builder-unsaved-banner-save"
          onClick={onSave}
          data-testid="unsaved-banner-save"
        >
          <Save size={13} aria-hidden="true" /> Save
        </button>
      </div>
      <style jsx>{`
        .builder-unsaved-banner {
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 10px 28px;
          background: linear-gradient(180deg, #fff7ed 0%, #fffbeb 100%);
          border-bottom: 1px solid #fcd34d;
          color: #78350f;
          font-family: "Inter", sans-serif;
          animation: builder-unsaved-banner-slide 180ms ease-out;
        }
        .builder-unsaved-banner-message {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 13.5px;
          font-weight: 600;
        }
        .builder-unsaved-banner-actions {
          margin-left: auto;
          display: flex;
          gap: 8px;
        }
        .builder-unsaved-banner-btn {
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
        .builder-unsaved-banner-discard {
          background: white;
          color: #78350f;
          border: 1px solid #fcd34d;
        }
        .builder-unsaved-banner-discard:hover {
          background: #fffbeb;
        }
        .builder-unsaved-banner-save {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border: none;
          box-shadow: 0 2px 8px -2px rgba(139, 92, 246, 0.4);
        }
        .builder-unsaved-banner-save:hover {
          filter: brightness(1.04);
        }
        @keyframes builder-unsaved-banner-slide {
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
          .builder-unsaved-banner {
            animation: none;
          }
        }
      `}</style>
    </div>
  );
}
