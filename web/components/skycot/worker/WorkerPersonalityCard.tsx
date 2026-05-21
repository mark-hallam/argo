"use client";

import { CheckCircle2, AlertTriangle, Heart } from "lucide-react";
import { useWorker } from "@/lib/hooks/use-worker";

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
 * V1 seeds traits from a static personality map keyed by worker
 * name. The character/charter system on the gateway will surface
 * these in a follow-up brief; the data shape here matches the
 * future API contract.
 */

interface PersonalityTraits {
  traits: string[];
  greatAt: string[];
  escalateWhen: string[];
  signature: string[];
}

const PERSONALITIES: Record<string, PersonalityTraits> = {
  Mariah: {
    traits: ["Patient", "Detail-oriented", "Picture-perfect listings", "Polite"],
    greatAt: [
      "Spotting price gaps against the live market",
      "Cropping + ordering photos for best buyer scroll",
      "Writing tight 60-word listing copy with real specifics",
      "Knowing when a listing needs a category change",
    ],
    escalateWhen: [
      "Price drop below the seller's floor",
      "Photos look heavily edited or stock",
      "Buyer asks a question outside the listing scope",
      "Multiple competing offers on the same item",
    ],
    signature: [
      "Always confirms the seller before posting publicly",
      "Asks for a single source photo if you give multiples",
      "Refuses to use stock imagery — always your photos",
    ],
  },
  Priya: {
    traits: ["Thorough", "Respectful", "Fast", "Calibrated"],
    greatAt: [
      "First-pass skill-match scoring against the JD",
      "Spotting tenure red flags + cultural mismatch",
      "Writing rejection emails that don't burn bridges",
      "Booking interview slots that respect both calendars",
    ],
    escalateWhen: [
      "Score is in the 50-65 band (neither yes nor no)",
      "Candidate has worked at a direct competitor",
      "JD has been edited in the last 24h",
      "Hiring manager hasn't replied to the last 3 candidates",
    ],
    signature: [
      "Always sends the rejection within 48h of decline",
      "Books a buffer week before a senior interview",
      "Asks for clarification before discounting a strong yes",
    ],
  },
  Jamal: {
    traits: ["Calm under pressure", "Empathic", "Quick", "Honest"],
    greatAt: [
      "Classifying tickets by severity in <2 minutes",
      "Pulling the right KB article for self-deflection",
      "Writing follow-ups that land in your customer's voice",
      "Spotting the same issue across 5+ tickets",
    ],
    escalateWhen: [
      "Customer mentions churn or 'cancel' in any reply",
      "Same customer opens 3+ tickets in 24h",
      "Issue involves billing or a refund over $200",
      "Customer is on a VIP or enterprise tier",
    ],
    signature: [
      "Always names the human owner when escalating",
      "Sends a 'still investigating' note at the 4h mark",
      "Never closes a ticket without a one-line outcome",
    ],
  },
};

const DEFAULT_PERSONALITY: PersonalityTraits = {
  traits: ["Reliable", "Curious", "Helpful"],
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
  signature: ["Always confirms before a public or irreversible action"],
};

export function WorkerPersonalityCard({ workerId }: { workerId: string }) {
  const workerQuery = useWorker(workerId);
  const worker = workerQuery.data;
  const name = worker?.name ?? "this teammate";
  const personality = (worker && PERSONALITIES[worker.name]) ?? DEFAULT_PERSONALITY;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .wpc-card {
          background: white;
          border: 1px solid var(--border, #ece9f5);
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
          background: linear-gradient(90deg, #8b5cf6, #ec4899, #ff6b6b);
        }
        .wpc-section-label {
          display: flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--muted-foreground, #52525b);
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
        .wpc-col h4 {
          margin: 0 0 10px;
          font-size: 13.5px;
          color: var(--foreground, #1e1b4b);
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
          color: var(--muted-foreground, #3f3f46);
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
          border-top: 1px solid var(--border, #ece9f5);
        }
      `}</style>
      <div className="wpc-card">
        <div className="wpc-section-label">
          <Heart size={11} /> HOW {name.toUpperCase()} WORKS
        </div>
        <div className="wpc-chips">
          {personality.traits.map((t) => (
            <span key={t} className="wpc-chip">
              {t}
            </span>
          ))}
        </div>

        <div className="wpc-cols">
          <div className="wpc-col">
            <h4>
              <CheckCircle2 size={13} style={{ color: "#14b8a6" }} /> Great at
            </h4>
            <ul className="wpc-list">
              {personality.greatAt.map((g, i) => (
                <li key={i}>{g}</li>
              ))}
            </ul>
          </div>
          <div className="wpc-col">
            <h4>
              <AlertTriangle size={13} style={{ color: "#f59e0b" }} /> Escalate when
            </h4>
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
            {personality.signature.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ul>
        </div>
      </div>
    </>
  );
}
