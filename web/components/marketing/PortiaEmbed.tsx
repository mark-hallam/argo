"use client";

import { useCallback, useState } from "react";
import { ArrowRight, ShieldCheck, Sparkles } from "lucide-react";

/**
 * "Talk to Portia" public embed — a lightweight taste of the Pitch Agent
 * (Portia) for anonymous landing-page visitors.
 *
 * ⚠️ PUBLIC-SAFETY FLAG — NOT WIRED TO THE LIVE ENDPOINT. Portia is live at
 * `POST /api/v1/pitch/ask` (truth-bounded), but that route is authenticated
 * and was built for the operator-facing `/pitch` rehearsal console. Exposing
 * the raw endpoint to anonymous public traffic needs a gated, rate-limited,
 * abuse-guarded PUBLIC-SAFE path first — otherwise it's an open,
 * cost-bearing LLM proxy. So this embed is intentionally a CURATED PREVIEW:
 * the answers below are real, operator-reviewed canonical Q&A pulled from the
 * pitch knowledge base, shown verbatim. The embed does NOT call the gateway,
 * and it is clearly labeled as a preview so it does not fake a live round-trip
 * (NO-THEATER, DEC-007). The full live rehearsal is one click away at `/pitch`
 * (behind auth).
 *
 * Backend follow-up to make this live for the public:
 *   - A public, unauthenticated, rate-limited `POST /api/v1/pitch/ask/public`
 *     (or a public-token-gated variant) that reuses the same truth-boundary
 *     evaluator but caps cost/abuse. Then swap the canned answers below for a
 *     real `askPitch()` call (the client already exists in
 *     `web/lib/api/pitch.ts`).
 */

interface CannedQa {
  q: string;
  a: string;
  /** The truth-boundary disposition this canonical answer carries. */
  disposition: "Allow" | "Escalate";
}

// Operator-reviewed canonical answers (verbatim taste — not a live call).
const CANNED: CannedQa[] = [
  {
    q: "What makes SKYCOT different from other AI agent platforms?",
    a: "Four things no competitor we surveyed matched: a hash-chained tamper-evident audit trail, a claim-level truth boundary that stops a teammate over-claiming, approval-first control across nine channels, and a hire-a-teammate model with persistent identity and shared skills.",
    disposition: "Allow",
  },
  {
    q: "Is every action really audit-logged?",
    a: "Yes — every action an AI teammate takes is written to a cryptographically hash-chained ledger, so the record is tamper-evident: it can't be silently altered or deleted after the fact. It's an audit trail you can take to a board or a regulator.",
    disposition: "Allow",
  },
  {
    q: "How much does SKYCOT cost?",
    a: "I'm not able to quote pricing — that's something the team confirms directly so you get an accurate number for your use case. I can put you in touch, or you can join the early-access list above.",
    disposition: "Escalate",
  },
];

/**
 * `Escalate` answers are exactly the truth boundary working — Portia
 * declines to state a number she can't back up and defers to a human.
 * Surfacing that on the marketing page is a feature, not a bug.
 */
export function PortiaEmbed() {
  const [activeIdx, setActiveIdx] = useState(0);
  const active = CANNED[activeIdx];

  const pick = useCallback((idx: number) => setActiveIdx(idx), []);

  return (
    <div className="portia-embed" data-testid="portia-embed">
      <div className="portia-head">
        <div className="portia-avatar" aria-hidden="true">
          <Sparkles size={18} />
        </div>
        <div>
          <p className="portia-name">Portia</p>
          <p className="portia-role">Pitch Agent · truth-bounded</p>
        </div>
        <span className="portia-preview-pill" data-testid="portia-preview-pill">
          PREVIEW
        </span>
      </div>

      <div className="portia-window" aria-live="polite">
        <div className="portia-bubble portia-bubble-user">{active.q}</div>
        <div className="portia-bubble portia-bubble-agent">
          <p className="portia-answer">{active.a}</p>
          <span
            className={`portia-disp portia-disp-${active.disposition.toLowerCase()}`}
          >
            <ShieldCheck size={11} />
            {active.disposition === "Allow"
              ? "Truth-boundary: cleared"
              : "Truth-boundary: deferred to a human"}
          </span>
        </div>
      </div>

      <p className="portia-prompt-label">Try one of these:</p>
      <div className="portia-chips">
        {CANNED.map((item, idx) => (
          <button
            key={item.q}
            type="button"
            className={`portia-chip ${idx === activeIdx ? "is-active" : ""}`}
            onClick={() => pick(idx)}
            data-testid={`portia-chip-${idx}`}
          >
            {item.q}
          </button>
        ))}
      </div>

      <div className="portia-foot">
        <p className="portia-note" data-testid="portia-safety-note">
          Preview answers are real, operator-reviewed responses — shown verbatim,
          not a live call. A public, rate-limited Portia is a backend follow-up.
        </p>
        <a href="/pitch" className="portia-cta">
          Open the live rehearsal <ArrowRight size={14} />
        </a>
      </div>
    </div>
  );
}
