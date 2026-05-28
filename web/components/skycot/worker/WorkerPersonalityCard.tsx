"use client";

import { CheckCircle2, AlertTriangle, Heart } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";
import { findSamplePersona, type SamplePersonality } from "@/lib/sample-personas";

/**
 * Worker personality card — C2 personality locked design (2026-05-19).
 *
 * Anchors the "this teammate is a person, not a tool" register at the
 * top of the worker overview tab. Renders:
 *
 *   - Trait chips: short adjectives describing the worker's
 *     personality ("Patient", "Detail-oriented", "Politely
 *     persistent")
 *   - "Great at" list: 3-4 things the worker excels at, in plain
 *     English from the operator's perspective
 *   - "Escalate when" list: 3-4 signals that should trigger human
 *     review, in worker-voice
 *   - Signature behaviors: 2-3 hallmark choices the worker always
 *     makes (e.g. "Always confirms before posting public listings")
 *
 * Traits + behavior copy are sourced from `sample-personas.ts` —
 * the canonical persona spine — so /team and /workers/<id> stay in
 * sync. The character/charter system on the gateway will surface
 * these in a follow-up brief; the data shape here matches the
 * future API contract.
 */

const DEFAULT_TRAITS: string[] = ["Reliable", "Curious", "Helpful"];

const DEFAULT_PERSONALITY: SamplePersonality = {
  greatAt: [
    "Working through the responsibilities you set",
    "Asking when something is ambiguous",
    "Surfacing patterns across recent runs",
  ],
  escalateWhen: [
    "Confidence in the next action is below 80%",
    "A risky side-effect would happen if they proceed",
    "Multiple plausible interpretations exist",
  ],
  signatureBehaviors: [
    "Always confirms before a public or irreversible action",
  ],
};

export function WorkerPersonalityCard({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);
  const worker = workerQuery.data;
  const name = worker?.name ?? "this teammate";
  const persona = findSamplePersona(workerId);
  const traits = persona?.traits ?? DEFAULT_TRAITS;
  const personality = persona?.personality ?? DEFAULT_PERSONALITY;

  return (
    <>
      <style jsx global>{`
        .wpc-card {
          background: white;
          border: 1px solid var(--border);
          border-radius: 14px;
          padding: 22px;
          position: relative;
          overflow: hidden;
        }
        .wpc-card::before {
          content: "";
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          height: 4px;
          background: linear-gradient(90deg, var(--primary), var(--gradient-end), var(--brand-coral));
        }
        .wpc-section-label {
          display: flex;
          align-items: center;
          gap: 6px;
          font-family: var(--font-mono);
          font-size: 10.5px;
          color: var(--muted-foreground);
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 10px;
        }
        .wpc-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;
          margin-bottom: 22px;
        }
        .wpc-chip {
          padding: 4px 10px;
          border-radius: 999px;
          background: rgba(139, 92, 246, 0.08);
          color: #6d28d9;
          font-size: 12.5px;
          font-weight: 600;
        }
        .wpc-cols {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 24px;
        }
        @media (max-width: 700px) {
          .wpc-cols {
            grid-template-columns: 1fr;
          }
        }
        .wpc-col h2 {
          margin: 0 0 10px;
          font-size: 13.5px;
          color: var(--foreground);
          font-weight: 600;
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .wpc-list {
          list-style: none;
          padding: 0;
          margin: 0;
          font-size: 13.5px;
          color: var(--muted-foreground);
          line-height: 1.55;
        }
        .wpc-list li {
          display: flex;
          gap: 8px;
          padding: 5px 0;
        }
        .wpc-list li::before {
          content: "";
          flex-shrink: 0;
          width: 4px;
          height: 4px;
          border-radius: 50%;
          background: #c4b5fd;
          margin-top: 9px;
        }
        .wpc-signature {
          margin-top: 22px;
          padding-top: 18px;
          border-top: 1px solid var(--border);
        }
      `}</style>
      <div className="wpc-card">
        <div className="wpc-section-label">
          <Heart size={11} /> HOW {name.toUpperCase()} WORKS
        </div>
        <div className="wpc-chips">
          {traits.map((t) => (
            <span key={t} className="wpc-chip">
              {t}
            </span>
          ))}
        </div>

        <div className="wpc-cols">
          <div className="wpc-col">
            <h2>
              <CheckCircle2 size={13} style={{ color: "var(--brand-teal)" }} /> Great at
            </h2>
            <ul className="wpc-list">
              {personality.greatAt.map((g, i) => (
                <li key={i}>{g}</li>
              ))}
            </ul>
          </div>
          <div className="wpc-col">
            <h2>
              <AlertTriangle size={13} style={{ color: "var(--warning)" }} /> Escalate when
            </h2>
            <ul className="wpc-list">
              {personality.escalateWhen.map((g, i) => (
                <li key={i}>{g}</li>
              ))}
            </ul>
          </div>
        </div>

        <div className="wpc-signature">
          <div className="wpc-section-label">SIGNATURE BEHAVIORS</div>
          <ul className="wpc-list">
            {personality.signatureBehaviors.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ul>
        </div>
      </div>
    </>
  );
}
