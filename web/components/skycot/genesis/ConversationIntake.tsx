"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { AlertTriangle, ArrowUp, CheckCircle2, Loader2 } from "lucide-react";
import { ConversationCapturedPanel } from "@/components/skycot/genesis/ConversationCapturedPanel";
import {
  EMPTY_CAPTURED,
  postConversationTurn,
  type ConversationCaptured,
  type ConversationStatus,
  type ConversationTurn,
} from "@/lib/api/genesis-conversation";
import type { SuccessCriterion } from "@/lib/api/genesis";

/**
 * "Talk it through" conversational intake surface for `/genesis` (UF Phase-A ·
 * A3, DEC-039). Drives a multi-turn back-and-forth against the live turn
 * endpoint `POST /api/v1/genesis/conversation` (engine shipped in #2724),
 * rendering the running transcript and a live [[ConversationCapturedPanel]].
 *
 * On `ready` it surfaces a confirm action; on confirm (`confirm: true` →
 * `done`) it hands the GATHERED SLOTS up via `onComplete` so the parent runs
 * the EXISTING one-shot `streamGenesis` → review → hire path. There is NO
 * parallel client-side materializer here — this component only gathers.
 *
 * Brand tokens only (DEC-027): purple→PINK gradient, Inter, `--brand-ink`.
 */

/** What the conversation hands to the parent's one-shot flow on confirm. */
export interface ConversationGatheredSlots {
  /** Composed free-text role description (charter + responsibilities + persona)
   *  fed to `streamGenesis` exactly like the "Describe it" textarea. */
  roleDescription: string;
  cadence: string;
  exceptions: string[];
  successCriteria: SuccessCriterion[];
  /** Draft-only access needs (DEC-048 Access slot). */
  accessIntegrations: string[];
  /** Escalation override (DEC-048 Escalation slot; null keeps the default). */
  escalationFailuresThreshold: number | null;
  escalationTo: string;
}

export interface ConversationIntakeProps {
  /** Per-browser tenant id (shared with the one-shot flow). */
  tenantId: string;
  /** Fired once the conversation is confirmed (`status: "done"`). */
  onComplete(gathered: ConversationGatheredSlots): void;
  /** Disabled while the parent's downstream stream/hire is in flight. */
  disabled?: boolean;
}

/** Compose the gathered slots from the captured block. Exported for tests. */
export function gatherSlotsFromCaptured(
  c: ConversationCaptured,
): ConversationGatheredSlots {
  const parts: string[] = [];
  if (c.charter.trim().length > 0) parts.push(c.charter.trim());
  if (c.responsibilities.length > 0) {
    parts.push(
      `Responsibilities:\n${c.responsibilities.map((r) => `- ${r}`).join("\n")}`,
    );
  }
  if (c.persona.trim().length > 0) parts.push(`Persona: ${c.persona.trim()}`);
  return {
    roleDescription: parts.join("\n\n"),
    cadence: c.cadence,
    exceptions: c.exceptions,
    successCriteria: c.success_criteria,
    accessIntegrations: c.access_integrations,
    escalationFailuresThreshold: c.escalation_failures_threshold,
    escalationTo: c.escalation_to,
  };
}

function newConversationId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `conv-${crypto.randomUUID().replace(/-/g, "").slice(0, 20)}`;
  }
  return `conv-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export function ConversationIntake({
  tenantId,
  onComplete,
  disabled,
}: ConversationIntakeProps) {
  const conversationIdRef = useRef<string>("");
  if (conversationIdRef.current === "") {
    conversationIdRef.current = newConversationId();
  }
  const startedRef = useRef(false);

  const [transcript, setTranscript] = useState<ConversationTurn[]>([]);
  const [captured, setCaptured] = useState<ConversationCaptured>({
    ...EMPTY_CAPTURED,
  });
  const [status, setStatus] = useState<ConversationStatus>("asking");
  const [canConfirm, setCanConfirm] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sendTurn = useCallback(
    async (opts: { message?: string; confirm?: boolean }) => {
      if (busy) return;
      setBusy(true);
      setError(null);
      // Optimistically show the operator's line; the response transcript (when
      // present) is authoritative and replaces this.
      if (opts.message) {
        setTranscript((prev) => [
          ...prev,
          { speaker: "operator", text: opts.message as string },
        ]);
      }
      try {
        const res = await postConversationTurn({
          conversationId: conversationIdRef.current,
          message: opts.message,
          confirm: opts.confirm,
          tenantId,
        });
        setCaptured(res.captured);
        setStatus(res.status);
        setCanConfirm(res.can_confirm === true || res.status === "ready");
        if (res.transcript.length > 0) {
          setTranscript(res.transcript);
        } else if (res.assistant_message) {
          setTranscript((prev) => [
            ...prev,
            { speaker: "assistant", text: res.assistant_message as string },
          ]);
        }
        if (res.status === "done") {
          onComplete(gatherSlotsFromCaptured(res.captured));
        }
      } catch (err) {
        setError(
          err instanceof Error && err.message
            ? err.message
            : "Couldn't reach the conversation. Please retry.",
        );
      } finally {
        setBusy(false);
      }
    },
    [busy, onComplete, tenantId],
  );

  // Opening turn: no message — the engine asks the first question.
  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    void sendTurn({});
    // sendTurn is stable enough for the one-shot kick; intentionally fired once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSend = useCallback(() => {
    const text = draft.trim();
    if (text.length === 0 || busy || disabled) return;
    setDraft("");
    void sendTurn({ message: text });
  }, [draft, busy, disabled, sendTurn]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  const handleConfirm = useCallback(() => {
    if (busy || disabled) return;
    void sendTurn({ confirm: true });
  }, [busy, disabled, sendTurn]);

  const inputsDisabled = busy || disabled || status === "done";

  return (
    <div className="ci" data-testid="genesis-conversation">
      <style jsx>{`
        .ci {
          margin: 18px auto 0;
          max-width: 920px;
          display: grid;
          grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
          gap: 24px;
          font-family: var(--font-sans);
          color: var(--brand-ink);
          text-align: left;
        }
        @media (max-width: 760px) {
          .ci {
            grid-template-columns: 1fr;
          }
        }
        .ci-chat {
          display: flex;
          flex-direction: column;
          gap: 14px;
          min-width: 0;
        }
        .ci-transcript {
          display: flex;
          flex-direction: column;
          gap: 10px;
        }
        .ci-bubble {
          max-width: 88%;
          padding: 11px 15px;
          border-radius: 16px;
          font-size: 14px;
          line-height: 1.5;
          white-space: pre-wrap;
        }
        .ci-bubble.assistant {
          align-self: flex-start;
          background: color-mix(in srgb, var(--brand-primary) 8%, #fff);
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 18%, transparent);
          border-bottom-left-radius: 5px;
        }
        .ci-bubble.operator {
          align-self: flex-end;
          color: #fff;
          background: linear-gradient(135deg, var(--brand-primary), var(--brand-pink));
          border-bottom-right-radius: 5px;
        }
        .ci-thinking {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          align-self: flex-start;
          font-size: 13px;
          color: color-mix(in srgb, var(--brand-ink) 60%, transparent);
        }
        .ci-spin {
          animation: ci-spin 0.9s linear infinite;
        }
        @keyframes ci-spin {
          to {
            transform: rotate(360deg);
          }
        }
        .ci-error {
          display: flex;
          align-items: flex-start;
          gap: 10px;
          padding: 12px 14px;
          border-radius: 12px;
          border: 1px solid #fecaca;
          background: rgba(254, 226, 226, 0.55);
          color: #7f1d1d;
          font-size: 13px;
        }
        .ci-composer {
          display: flex;
          align-items: flex-end;
          gap: 8px;
        }
        .ci-input {
          flex: 1 1 auto;
          min-width: 0;
          resize: none;
          font-family: inherit;
          font-size: 14px;
          color: var(--brand-ink);
          padding: 11px 13px;
          border-radius: 12px;
          border: 1.5px solid color-mix(in srgb, var(--brand-primary) 28%, transparent);
          background: color-mix(in srgb, var(--brand-primary) 4%, #fff);
        }
        .ci-input:focus {
          outline: none;
          border-color: var(--brand-primary);
        }
        .ci-input:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .ci-send {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 42px;
          height: 42px;
          border-radius: 12px;
          border: none;
          color: #fff;
          background: linear-gradient(135deg, var(--brand-primary), var(--brand-pink));
          cursor: pointer;
          flex-shrink: 0;
        }
        .ci-send:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .ci-confirm-card {
          margin-top: 4px;
          padding: 14px 16px;
          border-radius: 14px;
          border: 1.5px solid color-mix(in srgb, var(--brand-pink) 32%, transparent);
          background: linear-gradient(
            135deg,
            color-mix(in srgb, var(--brand-primary) 7%, #fff),
            color-mix(in srgb, var(--brand-pink) 7%, #fff)
          );
          display: flex;
          flex-direction: column;
          gap: 10px;
        }
        .ci-confirm-title {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          font-weight: 700;
        }
        .ci-confirm-icon {
          color: var(--brand-primary);
        }
        .ci-confirm-hint {
          font-size: 13px;
          line-height: 1.5;
          color: color-mix(in srgb, var(--brand-ink) 70%, transparent);
        }
        .ci-confirm-btn {
          align-self: flex-start;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 10px 18px;
          border-radius: 999px;
          border: none;
          color: #fff;
          font-size: 14px;
          font-weight: 600;
          background: linear-gradient(135deg, var(--brand-primary), var(--brand-pink));
          cursor: pointer;
        }
        .ci-confirm-btn:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .ci-captured {
          min-width: 0;
        }
      `}</style>

      <div className="ci-chat">
        <div
          className="ci-transcript"
          data-testid="genesis-conversation-transcript"
          aria-live="polite"
        >
          {transcript.map((turn, idx) => (
            <div
              key={`${turn.speaker}-${idx}`}
              className={`ci-bubble ${turn.speaker}`}
              data-speaker={turn.speaker}
              data-testid={`genesis-conversation-bubble-${idx}`}
            >
              {turn.text}
            </div>
          ))}
          {busy ? (
            <span className="ci-thinking" data-testid="genesis-conversation-thinking">
              <Loader2 size={14} className="ci-spin" aria-hidden />
              Thinking&hellip;
            </span>
          ) : null}
        </div>

        {error ? (
          <div className="ci-error" role="alert" data-testid="genesis-conversation-error">
            <AlertTriangle size={16} aria-hidden style={{ marginTop: 1 }} />
            <span>{error}</span>
          </div>
        ) : null}

        {status === "asking" && canConfirm && !busy ? (
          <div className="ci-confirm-card" data-testid="genesis-conversation-early-confirm">
            <p className="ci-confirm-hint">
              The essentials are captured — the remaining questions are optional.
            </p>
            <button
              type="button"
              className="ci-confirm-btn"
              onClick={handleConfirm}
              disabled={inputsDisabled}
              data-testid="genesis-conversation-confirm-early"
            >
              <CheckCircle2 size={15} aria-hidden />
              Skip the rest &amp; build
            </button>
          </div>
        ) : null}

        {status === "ready" ? (
          <div className="ci-confirm-card" data-testid="genesis-conversation-ready">
            <span className="ci-confirm-title">
              <CheckCircle2 size={16} className="ci-confirm-icon" aria-hidden />
              Ready to build this teammate
            </span>
            <p className="ci-confirm-hint">
              Review what we&rsquo;ve captured on the right. Confirm to build the
              teammate — same review and hire as &ldquo;Describe it&rdquo;.
            </p>
            <button
              type="button"
              className="ci-confirm-btn"
              onClick={handleConfirm}
              disabled={inputsDisabled}
              data-testid="genesis-conversation-confirm"
            >
              <CheckCircle2 size={15} aria-hidden />
              Confirm &amp; build
            </button>
          </div>
        ) : null}

        {status !== "done" ? (
          <div className="ci-composer">
            <textarea
              className="ci-input"
              rows={2}
              placeholder="Type your answer…"
              value={draft}
              disabled={inputsDisabled}
              data-testid="genesis-conversation-input"
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={handleKeyDown}
            />
            <button
              type="button"
              className="ci-send"
              onClick={handleSend}
              disabled={inputsDisabled || draft.trim().length === 0}
              aria-label="Send"
              data-testid="genesis-conversation-send"
            >
              <ArrowUp size={18} aria-hidden />
            </button>
          </div>
        ) : null}
      </div>

      <div className="ci-captured">
        <ConversationCapturedPanel captured={captured} />
      </div>
    </div>
  );
}
