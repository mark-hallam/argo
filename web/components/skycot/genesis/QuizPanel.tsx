"use client";

import { useCallback, useEffect, useState } from "react";
import { BadgeCheck, Loader2, MessageCircleQuestion, Pencil, Send } from "lucide-react";
import { askQuiz, fetchSuggestedQuestions, type QuizCitation } from "@/lib/api/genesis-quiz";
import { MemoryEditModal } from "./MemoryEditModal";

/**
 * "Quiz me" panel — the comprehension gate before Hire (Phase 2; memory
 * `genesis-is-ai-teammate-onboarding-studio`). Renders after the Genesis preview
 * card. The operator asks the to-be-hired teammate questions; it answers by
 * retrieving over the uploaded documents (real backend, no theater — when
 * nothing matches it says so plainly). A wrong assumption can be corrected via
 * the memory-edit modal (re-uploads a corrected doc) and re-quizzed.
 *
 * The Hire button stays disabled until the operator explicitly clicks "I'm
 * satisfied with their answers" — this panel owns that confirmation and reports
 * it up via `onSatisfied`.
 */

export interface QuizPanelProps {
  verticalId: string;
  /** Persona name, for the satisfied-state copy. */
  name: string;
  /** Role description, used only to seed grounded suggested questions. */
  roleDescription: string;
  /** Fired when the operator confirms they're satisfied — enables Hire. */
  onSatisfied(): void;
  /** Whether the operator has already confirmed satisfaction. */
  satisfied: boolean;
}

/** One question/answer exchange in the session history. */
interface Exchange {
  question: string;
  answered: boolean;
  answer: string;
  citations: QuizCitation[];
}

export function QuizPanel({
  verticalId,
  name,
  roleDescription,
  onSatisfied,
  satisfied,
}: QuizPanelProps) {
  const [suggested, setSuggested] = useState<string[]>([]);
  const [question, setQuestion] = useState("");
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<Exchange[]>([]);
  const [editing, setEditing] = useState(false);

  const loadSuggestions = useCallback(() => {
    fetchSuggestedQuestions(verticalId, roleDescription)
      .then(setSuggested)
      .catch(() => setSuggested([]));
  }, [verticalId, roleDescription]);

  useEffect(() => {
    loadSuggestions();
  }, [loadSuggestions]);

  const ask = useCallback(
    async (q: string) => {
      const trimmed = q.trim();
      if (!trimmed || asking) return;
      setAsking(true);
      setError(null);
      try {
        const resp = await askQuiz(verticalId, trimmed, roleDescription);
        setHistory((prev) => [
          {
            question: trimmed,
            answered: resp.answered,
            answer: resp.answer,
            citations: resp.citations,
          },
          ...prev,
        ]);
        setQuestion("");
      } catch {
        setError("Couldn't reach the sandbox teammate. Try again.");
      } finally {
        setAsking(false);
      }
    },
    [verticalId, roleDescription, asking],
  );

  return (
    <>
      <style jsx>{`
        .gen-quiz {
          max-width: 880px;
          margin: 24px auto 0;
          padding: 0 24px;
        }
        .gen-quiz-card {
          background: rgba(255, 255, 255, 0.98);
          border: 1px solid #ece9f5;
          border-radius: 22px;
          box-shadow: 0 20px 48px -28px rgba(30, 27, 75, 0.2);
          padding: 28px 32px 26px;
        }
        .gen-quiz-eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: var(--font-mono);
          font-size: 10.5px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 700;
          color: #6d28d9;
          margin-bottom: 6px;
        }
        .gen-quiz-title {
          font-family: var(--font-audiowide), "Inter", sans-serif;
          font-size: 22px;
          font-weight: 400;
          color: #1e1b4b;
          margin: 0;
        }
        .gen-quiz-sub {
          margin-top: 6px;
          font-size: 13.5px;
          color: #52525b;
          line-height: 1.55;
        }
        .gen-quiz-chips {
          margin-top: 16px;
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
        }
        .gen-quiz-chip {
          padding: 7px 12px;
          background: #fafaff;
          border: 1px solid #e6e1f5;
          border-radius: 999px;
          font-size: 12.5px;
          color: #4c1d95;
          cursor: pointer;
          text-align: left;
        }
        .gen-quiz-chip:hover {
          background: #f3effe;
        }
        .gen-quiz-input-row {
          margin-top: 16px;
          display: flex;
          gap: 10px;
          align-items: stretch;
        }
        .gen-quiz-input {
          flex: 1;
          min-height: 46px;
          padding: 0 14px;
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          color: #1e1b4b;
          border: 1px solid #ddd6f3;
          border-radius: 12px;
          outline: none;
        }
        .gen-quiz-input:focus {
          border-color: #8b5cf6;
        }
        .gen-quiz-ask {
          min-height: 46px;
          padding: 0 20px;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          font-weight: 700;
          color: white;
          background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
          border: none;
          border-radius: 12px;
          cursor: pointer;
        }
        .gen-quiz-ask:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .gen-quiz-error {
          margin-top: 10px;
          font-size: 13px;
          color: #b91c1c;
        }
        .gen-quiz-history {
          margin-top: 22px;
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .gen-quiz-x-q {
          font-size: 14px;
          font-weight: 700;
          color: #1e1b4b;
        }
        .gen-quiz-x-a {
          margin-top: 6px;
          font-size: 13.5px;
          color: #3f3f46;
          line-height: 1.55;
          white-space: pre-wrap;
        }
        .gen-quiz-x-a.unknown {
          color: #92400e;
          font-style: italic;
        }
        .gen-quiz-cites {
          margin-top: 8px;
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .gen-quiz-cite {
          font-size: 12px;
          color: #52525b;
          padding: 8px 10px;
          background: #fafaff;
          border-left: 3px solid #8b5cf6;
          border-radius: 0 8px 8px 0;
        }
        .gen-quiz-cite-src {
          font-family: var(--font-mono);
          font-size: 10.5px;
          letter-spacing: 0.04em;
          color: #6d28d9;
          display: block;
          margin-bottom: 2px;
        }
        .gen-quiz-footer {
          margin-top: 24px;
          padding-top: 20px;
          border-top: 1px solid #ece9f5;
          display: flex;
          gap: 12px;
          flex-wrap: wrap;
          align-items: center;
        }
        .gen-quiz-edit {
          min-height: 42px;
          padding: 0 16px;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 13.5px;
          font-weight: 600;
          color: #4c1d95;
          background: white;
          border: 1px solid #ddd6f3;
          border-radius: 11px;
          cursor: pointer;
        }
        .gen-quiz-satisfied {
          min-height: 42px;
          padding: 0 18px;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 13.5px;
          font-weight: 700;
          color: white;
          background: #047857;
          border: none;
          border-radius: 11px;
          cursor: pointer;
        }
        .gen-quiz-satisfied.done {
          background: rgba(16, 185, 129, 0.12);
          color: #047857;
          cursor: default;
        }
      `}</style>
      <div className="gen-quiz" data-testid="genesis-quiz-panel">
        <div className="gen-quiz-card">
          <div className="gen-quiz-eyebrow">
            <MessageCircleQuestion size={12} aria-hidden /> Comprehension check
          </div>
          <h3 className="gen-quiz-title">Quiz {name} before you hire</h3>
          <p className="gen-quiz-sub">
            Ask anything from the documents you uploaded. {name} answers from what
            they actually read — and says so if a document doesn&apos;t cover it.
            Wrong assumption? Fix their memory and ask again.
          </p>

          {suggested.length > 0 && (
            <div className="gen-quiz-chips" data-testid="genesis-quiz-suggestions">
              {suggested.map((q) => (
                <button
                  key={q}
                  type="button"
                  className="gen-quiz-chip"
                  onClick={() => ask(q)}
                  disabled={asking}
                >
                  {q}
                </button>
              ))}
            </div>
          )}

          <div className="gen-quiz-input-row">
            <input
              className="gen-quiz-input"
              type="text"
              placeholder="e.g. How would you handle a refund request over $500?"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") ask(question);
              }}
              data-testid="genesis-quiz-input"
              disabled={asking}
            />
            <button
              type="button"
              className="gen-quiz-ask"
              onClick={() => ask(question)}
              disabled={asking || !question.trim()}
              data-testid="genesis-quiz-ask"
            >
              {asking ? (
                <Loader2 size={16} className="gen-spin" aria-hidden />
              ) : (
                <Send size={15} aria-hidden />
              )}
              Ask
            </button>
          </div>
          {error && <p className="gen-quiz-error">{error}</p>}

          {history.length > 0 && (
            <div className="gen-quiz-history" data-testid="genesis-quiz-history">
              {history.map((x, i) => (
                <div key={`${x.question}-${i}`} className="gen-quiz-exchange">
                  <div className="gen-quiz-x-q">Q: {x.question}</div>
                  <div className={`gen-quiz-x-a${x.answered ? "" : " unknown"}`}>
                    {x.answer}
                  </div>
                  {x.citations.length > 0 && (
                    <div className="gen-quiz-cites">
                      {x.citations.map((c, ci) => (
                        <div key={ci} className="gen-quiz-cite">
                          <span className="gen-quiz-cite-src">{c.source_label}</span>
                          {c.excerpt}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          <div className="gen-quiz-footer">
            <button
              type="button"
              className="gen-quiz-edit"
              onClick={() => setEditing(true)}
              data-testid="genesis-quiz-edit-memory"
            >
              <Pencil size={14} aria-hidden /> Fix a memory
            </button>
            {satisfied ? (
              <span className="gen-quiz-satisfied done" data-testid="genesis-quiz-satisfied-done">
                <BadgeCheck size={15} aria-hidden /> Ready to hire
              </span>
            ) : (
              <button
                type="button"
                className="gen-quiz-satisfied"
                onClick={onSatisfied}
                data-testid="genesis-quiz-satisfied"
              >
                <BadgeCheck size={15} aria-hidden /> I&apos;m satisfied with their answers
              </button>
            )}
          </div>
        </div>
      </div>

      {editing && (
        <MemoryEditModal
          verticalId={verticalId}
          onClose={() => setEditing(false)}
          onSaved={() => {
            setEditing(false);
            loadSuggestions();
          }}
        />
      )}
    </>
  );
}
