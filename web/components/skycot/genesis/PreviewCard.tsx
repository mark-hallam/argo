"use client";

import { ArrowRight, BadgeCheck, Loader2 } from "lucide-react";
import type { GenesisVerticalReadyEvent } from "@/lib/api/genesis";

/**
 * Preview card rendered on the `vertical_ready` SSE event. Shows the
 * generated persona name + role + a small set of sample-action and
 * sample-duty rows derived from the Genesis manifest summary, plus the
 * tail of the operator's role description as the charter excerpt.
 *
 * The "Hire <name>" CTA fires the parent's `onHire` (the parent owns
 * the actual `POST /api/v1/workers` call so the loading / error /
 * navigation state lives in one place).
 */

const SAMPLE_DUTIES = [
  "Pick up inbound messages on the morning shift",
  "Escalate sensitive actions back to the operator before they go live",
];

export interface PreviewCardProps {
  ready: GenesisVerticalReadyEvent;
  roleDescription: string;
  onHire(): void;
  isHiring: boolean;
}

function deriveSampleActions(ready: GenesisVerticalReadyEvent): string[] {
  const archetypeLabel = humanizeSlug(ready.manifest.archetype);
  return [
    `Triage inbound work and surface anything ${archetypeLabel} should own.`,
    `Draft a response or proposal grounded in the operator-supplied charter.`,
    `Hold sensitive actions for explicit operator approval before executing.`,
  ];
}

function humanizeSlug(slug: string): string {
  return slug.replace(/[-_]+/g, " ");
}

function charterExcerpt(roleDescription: string): string {
  const trimmed = roleDescription.trim();
  if (trimmed.length <= 220) return trimmed;
  return `${trimmed.slice(0, 217).trim()}…`;
}

export function PreviewCard({
  ready,
  roleDescription,
  onHire,
  isHiring,
}: PreviewCardProps) {
  const actions = deriveSampleActions(ready);
  const name = ready.manifest.name || "Operator";
  const archetypeLabel = humanizeSlug(ready.manifest.archetype);

  return (
    <>
      <style jsx>{`
        .gen-preview {
          max-width: 880px;
          margin: 36px auto 0;
          padding: 0 24px;
        }
        .gen-preview-card {
          background: rgba(255, 255, 255, 0.98);
          border: 1px solid #ece9f5;
          border-radius: 22px;
          box-shadow: 0 20px 48px -24px rgba(30, 27, 75, 0.22);
          padding: 32px 32px 28px;
          position: relative;
          overflow: hidden;
        }
        .gen-preview-card::before {
          content: "";
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          height: 4px;
          background: linear-gradient(90deg, #8b5cf6 0%, #ec4899 100%);
        }
        .gen-preview-header {
          display: flex;
          align-items: flex-start;
          justify-content: space-between;
          gap: 24px;
          flex-wrap: wrap;
        }
        .gen-preview-badge {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: var(--font-mono);
          font-size: 10.5px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          color: #047857;
          background: rgba(16, 185, 129, 0.1);
          padding: 4px 10px;
          border-radius: 999px;
          margin-bottom: 10px;
        }
        .gen-preview-name {
          font-family: var(--font-audiowide), "Inter", sans-serif;
          font-size: 30px;
          font-weight: 400;
          letter-spacing: 0.005em;
          color: #1e1b4b;
          margin: 0;
          line-height: 1.1;
        }
        .gen-preview-role {
          margin-top: 6px;
          font-size: 14px;
          color: #52525b;
          text-transform: capitalize;
        }
        .gen-preview-tagline {
          margin-top: 18px;
          font-size: 14.5px;
          color: #3f3f46;
          line-height: 1.6;
        }
        .gen-preview-grid {
          margin-top: 24px;
          display: grid;
          grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
          gap: 24px;
        }
        @media (max-width: 720px) {
          .gen-preview-grid {
            grid-template-columns: minmax(0, 1fr);
          }
        }
        .gen-preview-col h3 {
          font-family: var(--font-mono);
          font-size: 10.5px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          color: #6d28d9;
          margin: 0 0 10px;
        }
        .gen-preview-list {
          list-style: none;
          padding: 0;
          margin: 0;
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .gen-preview-list li {
          padding: 10px 12px;
          background: #fafaff;
          border: 1px solid #ece9f5;
          border-radius: 10px;
          font-size: 13.5px;
          color: #1e1b4b;
          line-height: 1.45;
        }
        .gen-preview-cta-row {
          margin-top: 28px;
          display: flex;
          align-items: center;
          gap: 14px;
          flex-wrap: wrap;
        }
        .gen-preview-cta {
          min-height: 48px;
          padding: 0 28px;
          display: inline-flex;
          align-items: center;
          gap: 10px;
          font-family: "Inter", sans-serif;
          font-size: 15.5px;
          font-weight: 700;
          color: white;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          border: none;
          border-radius: 14px;
          cursor: pointer;
          box-shadow: 0 10px 28px -10px rgba(139, 92, 246, 0.55);
        }
        .gen-preview-cta:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .gen-preview-meta {
          font-family: var(--font-mono);
          font-size: 11.5px;
          color: #52525b;
          letter-spacing: 0.04em;
        }
        .gen-spin {
          animation: gen-spin-preview 1s linear infinite;
        }
        @keyframes gen-spin-preview {
          to {
            transform: rotate(360deg);
          }
        }
      `}</style>
      <div className="gen-preview" data-testid="genesis-preview-card">
        <div className="gen-preview-card">
          <div className="gen-preview-header">
            <div>
              <div className="gen-preview-badge">
                <BadgeCheck size={11} aria-hidden /> Draft ready
              </div>
              <h2 className="gen-preview-name">{name}</h2>
              <p className="gen-preview-role">{archetypeLabel}</p>
            </div>
          </div>
          <p className="gen-preview-tagline">{charterExcerpt(roleDescription)}</p>
          <div className="gen-preview-grid">
            <div className="gen-preview-col">
              <h3>Sample actions</h3>
              <ul className="gen-preview-list" data-testid="genesis-sample-actions">
                {actions.map((a) => (
                  <li key={a}>{a}</li>
                ))}
              </ul>
            </div>
            <div className="gen-preview-col">
              <h3>Sample duties</h3>
              <ul className="gen-preview-list" data-testid="genesis-sample-duties">
                {SAMPLE_DUTIES.map((d) => (
                  <li key={d}>{d}</li>
                ))}
              </ul>
            </div>
          </div>
          <div className="gen-preview-cta-row">
            <button
              type="button"
              className="gen-preview-cta"
              onClick={onHire}
              disabled={isHiring}
              data-testid="genesis-hire-button"
            >
              {isHiring ? (
                <>
                  <Loader2 size={18} className="gen-spin" aria-hidden /> Hiring…
                </>
              ) : (
                <>
                  Hire {name} <ArrowRight size={16} aria-hidden />
                </>
              )}
            </button>
            <span className="gen-preview-meta" data-testid="genesis-meta">
              {ready.manifest.actions_declared} actions · {ready.eval_seed_count}{" "}
              eval seeds · vertical {ready.vertical_id}
            </span>
          </div>
        </div>
      </div>
    </>
  );
}
