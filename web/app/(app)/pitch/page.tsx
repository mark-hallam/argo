"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Loader2, Mic, MicOff, Radio, Volume2 } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { askPitch } from "@/lib/api/pitch";
import { ApiError } from "@/lib/api/types";
import { useVoiceTurn, voiceCaptureSupported } from "@/lib/api/use-voice-turn";
import { useRealtimeVoice, realtimeVoiceSupported } from "@/lib/api/use-realtime-voice";
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

// Group the flat chronological history into turns (an operator question and
// the agent reply that follows it), newest turn first, so the latest exchange
// renders directly under the input box instead of scrolling away to the bottom
// of the page. Messages within a turn keep their natural order (question above
// answer).
function groupIntoTurns(history: Message[]): Message[][] {
  const turns: Message[][] = [];
  for (const m of history) {
    if (m.role === "operator" || turns.length === 0) {
      turns.push([m]);
    } else {
      turns[turns.length - 1].push(m);
    }
  }
  turns.reverse();
  return turns;
}

export default function PitchPage() {
  const [question, setQuestion] = useState("");
  const [history, setHistory] = useState<Message[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputCardRef = useRef<HTMLDivElement | null>(null);

  // Newest exchange renders directly under the input (see `groupIntoTurns`).
  // On each new turn, bring the input + newest answer to the top of the
  // viewport so the latest exchange is where the eye already is — not scrolled
  // away to the bottom. Skipped on first load so the hero stays visible until
  // the first question.
  useEffect(() => {
    if (history.length > 0) {
      inputCardRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    }
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
  // Live, two-way streaming voice (WebRTC). Rehearsal channel so the operator
  // hears the full truth-bounded answer set, same as the typed rehearsal.
  const liveVoice = useRealtimeVoice({ context: "Rehearsal" });

  // `voiceCaptureSupported()` reads `window`/`MediaRecorder`, which only
  // exist on the client. Evaluating it during the first render pass would make
  // the server emit no mic button while the first client paint emits one — a
  // real hydration mismatch (server renders `pitch-submit` first in the action
  // row, client renders `pitch-talk`). Gate it behind a post-mount flag so the
  // FIRST client render matches the server (no mic button); the mic affordance
  // appears once `useEffect` commits after hydration.
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    setMounted(true);
  }, []);
  const micSupported = mounted && voiceCaptureSupported();
  const liveSupported = mounted && realtimeVoiceSupported();

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
  const liveButtonLabel =
    liveVoice.phase === "connecting"
      ? "Connecting…"
      : liveVoice.isLive
        ? "End live call"
        : "Go Live";

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
            <div className="pitch-input-card" ref={inputCardRef}>
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
                  {liveSupported ? (
                    <button
                      className="pitch-talk is-live"
                      type="button"
                      onClick={() =>
                        liveVoice.isLive || liveVoice.phase === "connecting"
                          ? liveVoice.stop()
                          : void liveVoice.start()
                      }
                      disabled={pending}
                      aria-pressed={liveVoice.isLive}
                      aria-label={liveButtonLabel}
                      title={liveButtonLabel}
                      data-testid="pitch-go-live"
                      style={
                        liveVoice.isLive
                          ? { background: "#dc2626", color: "#fff", borderColor: "#dc2626" }
                          : undefined
                      }
                    >
                      <Radio
                        size={14}
                        className={liveVoice.phase === "connecting" ? "pitch-spin" : undefined}
                      />
                      <span>{liveButtonLabel}</span>
                    </button>
                  ) : null}
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
            {liveVoice.error ? (
              <div className="pitch-error-panel" role="alert" data-testid="pitch-live-error">
                <AlertTriangle size={16} />
                <span>{liveVoice.error}</span>
              </div>
            ) : null}
            {liveVoice.phase === "connecting" || liveVoice.isLive ? (
              <section
                className="pitch-live-panel"
                data-testid="pitch-live-panel"
                aria-live="polite"
                style={{
                  border: "1px solid var(--border)",
                  borderRadius: "var(--radius-md, 12px)",
                  background: "var(--card)",
                  padding: "14px 16px",
                  marginBottom: 16,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
                  <span
                    aria-hidden
                    style={{
                      width: 10,
                      height: 10,
                      borderRadius: "50%",
                      background: liveVoice.isLive ? "#dc2626" : "#f59e0b",
                      boxShadow: liveVoice.isLive ? "0 0 0 4px rgba(220,38,38,.18)" : "none",
                      flexShrink: 0,
                    }}
                  />
                  <strong style={{ fontSize: 13, letterSpacing: ".02em" }}>
                    {liveVoice.isLive ? "LIVE · streaming voice" : "Connecting to Portia…"}
                  </strong>
                  <span style={{ fontSize: 11, color: "var(--muted-foreground)" }}>
                    model-bounded · no post-answer net
                  </span>
                  <button
                    type="button"
                    onClick={() => liveVoice.stop()}
                    data-testid="pitch-live-end"
                    style={{
                      marginLeft: "auto",
                      fontSize: 12,
                      padding: "4px 12px",
                      borderRadius: 8,
                      border: "1px solid var(--border)",
                      background: "transparent",
                      cursor: "pointer",
                    }}
                  >
                    End
                  </button>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {liveVoice.transcripts.map((t, i) => (
                    <p key={i} style={{ margin: 0, fontSize: 14, lineHeight: 1.5 }}>
                      <span
                        style={{
                          fontWeight: 700,
                          color: t.role === "user" ? "var(--foreground)" : "var(--primary)",
                        }}
                      >
                        {t.role === "user" ? "You" : "Portia"}:
                      </span>{" "}
                      {t.text}
                    </p>
                  ))}
                  {liveVoice.liveCaption ? (
                    <p style={{ margin: 0, fontSize: 14, lineHeight: 1.5, opacity: 0.7 }}>
                      <span style={{ fontWeight: 700, color: "var(--primary)" }}>Portia:</span>{" "}
                      {liveVoice.liveCaption}…
                    </p>
                  ) : null}
                  {liveVoice.isLive &&
                  liveVoice.transcripts.length === 0 &&
                  !liveVoice.liveCaption ? (
                    <p style={{ margin: 0, fontSize: 13, color: "var(--muted-foreground)" }}>
                      Speak now — ask her anything an investor would. Try the valuation question.
                    </p>
                  ) : null}
                </div>
              </section>
            ) : null}
            <div className="pitch-history" data-testid="pitch-history">
              {history.length === 0 && !pending ? (
                <div className="pitch-empty" data-testid="pitch-empty">
                  Ask a question to start the rehearsal.
                </div>
              ) : null}
              {groupIntoTurns(history).map((turn, ti) => (
                <div className="pitch-turn" key={turn[0]?.id ?? ti}>
                  {turn.map((m) => (
                    <MessageBubble key={m.id} message={m} />
                  ))}
                  {ti === 0 && pending ? (
                    <div className="pitch-msg is-agent" data-testid="pitch-pending">
                      <p className="pitch-msg-text">Thinking…</p>
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </>
  );
}
