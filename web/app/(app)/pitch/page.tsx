"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Loader2, Mic, MicOff, Volume2 } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { askPitch } from "@/lib/api/pitch";
import { ApiError } from "@/lib/api/types";
import { useVoiceTurn, voiceCaptureSupported } from "@/lib/api/use-voice-turn";
import type { PitchTurnResponse } from "@/lib/api/voice";
import {
  MessageBubble,
  voiceMeta,
  type AgentMessage,
  type Message,
  type OperatorMessage,
} from "./pitch-message-bubble";
import { PitchStyles } from "./pitch-styles";

/**
 * `/pitch` — operator-facing rehearsal surface for the Pitch Agent.
 *
 * Calls `POST /api/v1/pitch/ask` for each typed question and
 * `POST /api/v1/voice/pitch-turn` for each spoken one — both render the
 * answer + truth-boundary disposition badge alongside the canonical entry
 * id when the matcher hits. No mock fallback — every turn goes to the
 * gateway. Same truth boundary the 111-case pitch-eval suite proves green:
 * an over-claim question defers whether typed or spoken.
 *
 * Voice uses the L1/L2 stack (`argo-voice-io` + the gateway voice routes,
 * DEC-021/DEC-023): mic capture → STT → the reused pitch answer → TTS →
 * playback. Bubble rendering lives in `./pitch-message-bubble`.
 */

type Role = "operator" | "agent";

const PLACEHOLDER =
  "Ask anything an investor might ask. Try: What's the tech stack? How much does it cost?";

function newId(role: Role): string {
  const rand =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  return `${role}-${rand}`;
}

export default function PitchPage() {
  const [question, setQuestion] = useState("");
  const [history, setHistory] = useState<Message[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const historyEndRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    historyEndRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [history.length, pending]);

  const handleSubmit = useCallback(async () => {
    const trimmed = question.trim();
    if (!trimmed || pending) return;
    const operatorMsg: OperatorMessage = {
      id: newId("operator"),
      role: "operator",
      text: trimmed,
    };
    setHistory((prev) => [...prev, operatorMsg]);
    setQuestion("");
    setPending(true);
    setError(null);
    try {
      const res = await askPitch({ question: trimmed, channel: "Rehearsal" });
      const agentMsg: AgentMessage = {
        id: newId("agent"),
        role: "agent",
        text: res.answer,
        meta: res,
      };
      setHistory((prev) => [...prev, agentMsg]);
    } catch (err) {
      const message =
        err instanceof ApiError && err.message
          ? err.message
          : err instanceof Error && err.message
            ? err.message
            : "Couldn't reach the Pitch Agent.";
      setError(message);
    } finally {
      setPending(false);
    }
  }, [question, pending]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit],
  );

  // Spoken turn: append the heard question + Portia's spoken answer to the
  // same history the typed flow uses, so they render identically. The
  // truth-boundary block is reshaped into the `meta` the agent bubble
  // already knows how to render (disposition + source pills).
  const handleVoiceTurn = useCallback((turn: PitchTurnResponse) => {
    const operatorMsg: OperatorMessage = {
      id: newId("operator"),
      role: "operator",
      text: turn.transcript,
    };
    const agentMsg: AgentMessage = {
      id: newId("agent"),
      role: "agent",
      text: turn.answer_text,
      meta: voiceMeta(turn),
    };
    setHistory((prev) => [...prev, operatorMsg, agentMsg]);
  }, []);

  const voice = useVoiceTurn({ context: "WebEmbed", onTurn: handleVoiceTurn });
  const micSupported = voiceCaptureSupported();
  const voiceLabel =
    voice.phase === "recording"
      ? "Listening… tap to send"
      : voice.phase === "thinking"
        ? "Thinking…"
        : voice.phase === "speaking"
          ? "Portia is speaking…"
          : "Talk to Portia";
  const VoiceIcon =
    voice.phase === "recording"
      ? MicOff
      : voice.phase === "thinking"
        ? Loader2
        : voice.phase === "speaking"
          ? Volume2
          : Mic;

  return (
    <>
      <PitchStyles />
      <div className="pitch-page-bg">
        <div className="pitch-page-content">
          <BrandLoudHero
            eyebrowDotColor="#fde68a"
            eyebrowLabel="PITCH AGENT — REHEARSAL"
            pulseDot
            title="Watch what happens when I try to pitch SKYCOT."
            titleAccent="SKYCOT"
            sub="Rehearse with the Pitch Agent. Truth-bounded, evidence-led, brand-safe."
            subMaxWidth={520}
          />
          <section className="pitch-shell" aria-label="Pitch Agent chat">
            <div className="pitch-input-card">
              <textarea
                className="pitch-input"
                aria-label="Ask the Pitch Agent"
                placeholder={PLACEHOLDER}
                value={question}
                onChange={(e) => setQuestion(e.target.value)}
                onKeyDown={onKeyDown}
                disabled={pending}
                data-testid="pitch-input"
              />
              <div className="pitch-actions">
                <span className="pitch-hint">Enter to send · Shift+Enter for newline</span>
                <div className="pitch-action-buttons">
                  {micSupported ? (
                    <button
                      className={`pitch-talk is-${voice.phase}`}
                      type="button"
                      onClick={() => voice.toggle()}
                      disabled={pending || voice.phase === "thinking" || voice.phase === "speaking"}
                      aria-pressed={voice.isRecording}
                      aria-label={voiceLabel}
                      title={voiceLabel}
                      data-testid="pitch-talk"
                    >
                      <VoiceIcon
                        size={14}
                        className={voice.phase === "thinking" ? "pitch-spin" : undefined}
                      />
                      <span>{voiceLabel}</span>
                    </button>
                  ) : null}
                  <button
                    className="pitch-submit"
                    type="button"
                    onClick={() => void handleSubmit()}
                    disabled={pending || question.trim().length === 0}
                    data-testid="pitch-submit"
                  >
                    {pending ? "Asking…" : "Ask"}
                  </button>
                </div>
              </div>
            </div>
            {voice.error ? (
              <div className="pitch-error-panel" role="alert" data-testid="pitch-voice-error">
                <AlertTriangle size={16} />
                <span>{voice.error}</span>
              </div>
            ) : null}
            {error ? (
              <div className="pitch-error-panel" role="alert" data-testid="pitch-error">
                <AlertTriangle size={16} />
                <span>{error}</span>
              </div>
            ) : null}
            <div className="pitch-history" data-testid="pitch-history">
              {history.length === 0 && !pending ? (
                <div className="pitch-empty" data-testid="pitch-empty">
                  Ask a question to start the rehearsal.
                </div>
              ) : null}
              {history.map((m) => (
                <MessageBubble key={m.id} message={m} />
              ))}
              {pending ? (
                <div className="pitch-msg is-agent" data-testid="pitch-pending">
                  <p className="pitch-msg-text">Thinking…</p>
                </div>
              ) : null}
              <div ref={historyEndRef} />
            </div>
          </section>
        </div>
      </div>
    </>
  );
}
