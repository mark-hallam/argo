"use client";

import { useCallback, useState, type FormEvent } from "react";
import { ArrowRight, Loader2, Send, ShieldCheck, Sparkles } from "lucide-react";
import { askPitchPublic } from "@/lib/api/pitch";

/**
 * "Talk to Portia" public embed — a LIVE, truth-bounded taste of the Pitch
 * Agent (Portia) for anonymous landing-page visitors.
 *
 * This now makes a REAL call to the public, anonymous, rate-limited
 * `POST /api/v1/pitch/ask/public` (gateway-public-safe-pitch-ask-rate-limited):
 * the gateway forces the strict public truth-boundary channel server-side,
 * grounds the answer in the operator-reviewed canonical-Q&A corpus, and
 * returns either a corpus-grounded answer or a deterministic deferral —
 * never a fabricated claim. So the truth-boundary moat is proven LIVE on the
 * landing page instead of with a hand-written transcript.
 *
 * NO-THEATER (DEC-007): when the live route is unreachable or rate-limited
 * (HTTP 429), the embed falls back GRACEFULLY to an operator-reviewed canned
 * answer and labels it clearly as an offline sample — it never fakes a live
 * round-trip. The full operator rehearsal console is at `/pitch` (behind auth).
 */

type Disposition = "Allow" | "Escalate" | "Block";

interface OfflineQa {
  q: string;
  a: string;
  disposition: "Allow" | "Escalate";
}

/**
 * Operator-reviewed canonical answers — used ONLY as the graceful offline
 * fallback (live route unreachable / rate-limited) and as the suggested
 * starter questions. Never presented as a live answer.
 */
const OFFLINE_QA: OfflineQa[] = [
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

const SUGGESTIONS: readonly string[] = OFFLINE_QA.map((item) => item.q);

const OFFLINE_DEFER =
  "I can't reach my live knowledge base right now — so I won't guess. The team can follow up directly, or you can join the early-access list above.";

interface Exchange {
  q: string;
  a: string;
  disposition: Disposition;
  /**
   * `true` when this answer is the operator-reviewed offline fallback (the
   * live call failed or was rate-limited); `false` when it is a real,
   * truth-bounded gateway answer.
   */
  offline: boolean;
  /** `true` for the initial seed example shown before the visitor asks. */
  seeded?: boolean;
}

function offlineAnswerFor(question: string): Exchange {
  const match = OFFLINE_QA.find((item) => item.q === question);
  if (match) {
    return { q: question, a: match.a, disposition: match.disposition, offline: true };
  }
  return { q: question, a: OFFLINE_DEFER, disposition: "Escalate", offline: true };
}

function dispLabel(disposition: Disposition): string {
  return disposition === "Allow"
    ? "Truth-boundary: cleared"
    : "Truth-boundary: deferred to a human";
}

function dispClass(disposition: Disposition): string {
  // Block (a swapped-in refusal) renders with the same "deferred" treatment
  // as Escalate — both mean "Portia did not state an unbacked claim".
  return disposition === "Allow" ? "portia-disp-allow" : "portia-disp-escalate";
}

/**
 * `Escalate` answers are exactly the truth boundary working — Portia
 * declines to state a number she can't back up and defers to a human.
 * Surfacing that on the marketing page is a feature, not a bug.
 */
export function PortiaEmbed() {
  // Seed with the first canonical answer as a clearly-labeled sample so the
  // window is never empty; the visitor's own questions get LIVE answers.
  const [exchange, setExchange] = useState<Exchange>({
    ...OFFLINE_QA[0],
    offline: true,
    seeded: true,
  });
  const [pendingQ, setPendingQ] = useState<string | null>(null);
  const [input, setInput] = useState("");
  const pending = pendingQ !== null;

  const ask = useCallback(
    async (raw: string) => {
      const question = raw.trim();
      if (!question || pending) return;
      setInput("");
      setPendingQ(question);
      try {
        const res = await askPitchPublic(question);
        setExchange({
          q: question,
          a: res.answer,
          disposition: res.disposition,
          offline: false,
        });
      } catch {
        // 429 / network / non-2xx → graceful, clearly-labeled offline answer.
        setExchange(offlineAnswerFor(question));
      } finally {
        setPendingQ(null);
      }
    },
    [pending],
  );

  const onSubmit = useCallback(
    (e: FormEvent<HTMLFormElement>) => {
      e.preventDefault();
      void ask(input);
    },
    [ask, input],
  );

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
        <span className="portia-live-pill" data-testid="portia-live-pill">
          LIVE
        </span>
      </div>

      <div className="portia-window" aria-live="polite">
        {pending ? (
          <>
            <div className="portia-bubble portia-bubble-user">{pendingQ}</div>
            <div className="portia-bubble portia-bubble-agent" data-testid="portia-pending">
              <span className="portia-thinking">
                <Loader2 size={14} className="portia-spin" /> Portia is checking her knowledge base…
              </span>
            </div>
          </>
        ) : (
          <>
            <div className="portia-bubble portia-bubble-user">{exchange.q}</div>
            <div className="portia-bubble portia-bubble-agent">
              <p className="portia-answer" data-testid="portia-answer">
                {exchange.a}
              </p>
              <span
                className={`portia-disp ${dispClass(exchange.disposition)}`}
                data-testid="portia-disp"
              >
                <ShieldCheck size={11} />
                {dispLabel(exchange.disposition)}
              </span>
              {exchange.offline && (
                <span className="portia-offline-tag" data-testid="portia-offline-tag">
                  {exchange.seeded ? "Sample answer" : "Offline sample"} — ask your own
                  question for a live reply
                </span>
              )}
            </div>
          </>
        )}
      </div>

      <form className="portia-ask" onSubmit={onSubmit}>
        <input
          className="portia-ask-input"
          data-testid="portia-input"
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Ask Portia anything about SKYCOT…"
          maxLength={600}
          disabled={pending}
          aria-label="Ask Portia a question"
        />
        <button
          type="submit"
          className="portia-ask-send"
          data-testid="portia-send"
          disabled={pending || input.trim().length === 0}
          aria-label="Send question to Portia"
        >
          <Send size={15} />
        </button>
      </form>

      <p className="portia-prompt-label">Or try one of these:</p>
      <div className="portia-chips">
        {SUGGESTIONS.map((q, idx) => (
          <button
            key={q}
            type="button"
            className="portia-chip"
            onClick={() => void ask(q)}
            disabled={pending}
            data-testid={`portia-chip-${idx}`}
          >
            {q}
          </button>
        ))}
      </div>

      <div className="portia-foot">
        <p className="portia-note" data-testid="portia-safety-note">
          Portia answers live and truth-bounded: she only states what's in her
          operator-reviewed knowledge base and defers anything she can't back up.
          If the live service is unavailable, you'll see a clearly-labeled sample.
        </p>
        <a href="/pitch" className="portia-cta">
          Open the live rehearsal <ArrowRight size={14} />
        </a>
      </div>
    </div>
  );
}
