"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Mic, ShieldAlert, ShieldCheck, ShieldQuestion } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { askPitch, type PitchAskResponse, type PitchDisposition } from "@/lib/api/pitch";
import { ApiError } from "@/lib/api/types";
import { sourcePill, tokensLabel } from "./pitch-source-chip";

/**
 * `/pitch` — operator-facing rehearsal surface for the Pitch Agent.
 *
 * Calls `POST /api/v1/pitch/ask` for each question, renders the
 * answer + truth-boundary disposition badge alongside the canonical
 * entry id when the matcher hits. No mock fallback — every question
 * goes to the gateway. Same code path the 111-case pitch-eval suite
 * proves green.
 *
 * Single-shot Q&A only: no multi-turn memory, no voice. Voice is
 * P-03's `argo-voice-io` crate and lives behind a follow-up brief.
 */

type Role = "operator" | "agent";

interface OperatorMessage {
  id: string;
  role: "operator";
  text: string;
}

interface AgentMessage {
  id: string;
  role: "agent";
  text: string;
  meta?: PitchAskResponse;
}

type Message = OperatorMessage | AgentMessage;

const PLACEHOLDER =
  "Ask anything an investor might ask. Try: What's the tech stack? How much does it cost?";

function newId(role: Role): string {
  const rand =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  return `${role}-${rand}`;
}

function dispositionPill(disposition: PitchDisposition) {
  if (disposition === "Block") {
    return { Icon: ShieldAlert, label: "Blocked", className: "pitch-pill is-block" };
  }
  if (disposition === "Escalate") {
    return { Icon: ShieldQuestion, label: "Escalated", className: "pitch-pill is-escalate" };
  }
  return { Icon: ShieldCheck, label: "Allowed", className: "pitch-pill is-allow" };
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

  return (
    <>
      <style jsx global>{`
        .pitch-page-bg {
          position: relative;
          min-height: calc(100vh - 64px);
          background: #fafaff;
          overflow: hidden;
        }
        .pitch-page-bg::before {
          content: "";
          position: absolute;
          top: -180px;
          right: -120px;
          width: 540px;
          height: 540px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.18) 0%, transparent 70%);
          pointer-events: none;
        }
        .pitch-page-bg::after {
          content: "";
          position: absolute;
          bottom: -200px;
          left: -120px;
          width: 600px;
          height: 600px;
          background: radial-gradient(circle, rgba(236, 72, 153, 0.14) 0%, transparent 70%);
          pointer-events: none;
        }
        .pitch-page-content {
          position: relative;
          z-index: 1;
          padding-bottom: 72px;
        }
        .pitch-shell {
          max-width: 720px;
          margin: 28px auto 0;
          padding: 0 16px;
        }
        .pitch-input-card {
          display: flex;
          flex-direction: column;
          gap: 12px;
          padding: 18px;
          background: white;
          border: 1px solid rgba(139, 92, 246, 0.18);
          border-radius: 16px;
          box-shadow: 0 12px 36px -22px rgba(76, 29, 149, 0.35);
        }
        .pitch-input {
          width: 100%;
          min-height: 96px;
          resize: vertical;
          padding: 14px 16px;
          border: 1px solid rgba(15, 23, 42, 0.12);
          border-radius: 12px;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          line-height: 1.5;
          color: #1e1b4b;
          background: #fdfcff;
          outline: none;
        }
        .pitch-input:focus {
          border-color: #8b5cf6;
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.18);
        }
        .pitch-actions {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
        }
        .pitch-hint {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: rgba(30, 27, 75, 0.55);
        }
        .pitch-submit {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 10px 18px;
          border-radius: 12px;
          border: none;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          letter-spacing: 0.01em;
          cursor: pointer;
          box-shadow: 0 8px 22px -12px rgba(139, 92, 246, 0.65);
        }
        .pitch-submit:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .pitch-history {
          margin-top: 24px;
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .pitch-empty {
          margin: 20px 0;
          padding: 22px;
          border-radius: 14px;
          background: rgba(255, 255, 255, 0.7);
          border: 1px dashed rgba(139, 92, 246, 0.32);
          color: rgba(30, 27, 75, 0.7);
          text-align: center;
          font-family: "Inter", sans-serif;
          font-size: 14px;
        }
        .pitch-msg {
          padding: 14px 18px;
          border-radius: 14px;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          line-height: 1.5;
          color: #1e1b4b;
        }
        .pitch-msg.is-operator {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.95), rgba(236, 72, 153, 0.92));
          color: white;
          align-self: flex-end;
          max-width: 80%;
          box-shadow: 0 10px 28px -18px rgba(139, 92, 246, 0.7);
        }
        .pitch-msg.is-agent {
          background: white;
          border: 1px solid rgba(15, 23, 42, 0.08);
          align-self: flex-start;
          max-width: 92%;
          box-shadow: 0 8px 24px -20px rgba(15, 23, 42, 0.3);
        }
        .pitch-msg-text {
          margin: 0;
          white-space: pre-wrap;
        }
        .pitch-meta {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          margin-top: 10px;
        }
        .pitch-pill {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 4px 10px;
          border-radius: 999px;
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.1em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .pitch-pill svg {
          width: 12px;
          height: 12px;
        }
        .pitch-pill.is-allow {
          background: rgba(34, 197, 94, 0.12);
          color: #166534;
        }
        .pitch-pill.is-escalate {
          background: rgba(245, 158, 11, 0.14);
          color: #92400e;
        }
        .pitch-pill.is-block {
          background: rgba(239, 68, 68, 0.14);
          color: #991b1b;
        }
        .pitch-pill.is-entry {
          background: rgba(139, 92, 246, 0.12);
          color: #5b21b6;
          letter-spacing: 0.05em;
          text-transform: none;
        }
        .pitch-pill.is-latency {
          background: rgba(15, 23, 42, 0.05);
          color: rgba(30, 27, 75, 0.7);
          text-transform: none;
          letter-spacing: 0.04em;
        }
        .pitch-pill.is-source-canonical {
          background: rgba(14, 165, 233, 0.12);
          color: #075985;
          text-transform: none;
          letter-spacing: 0.04em;
        }
        .pitch-pill.is-source-llm {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.18), rgba(236, 72, 153, 0.14));
          color: #5b21b6;
          text-transform: none;
          letter-spacing: 0.04em;
        }
        .pitch-pill.is-source-llm-blocked {
          background: rgba(239, 68, 68, 0.1);
          color: #7f1d1d;
          text-transform: none;
          letter-spacing: 0.04em;
        }
        .pitch-pill.is-tokens {
          background: rgba(15, 23, 42, 0.04);
          color: rgba(30, 27, 75, 0.6);
          text-transform: none;
          letter-spacing: 0.04em;
          font-variant-numeric: tabular-nums;
        }
        .pitch-pill.is-model {
          background: rgba(76, 29, 149, 0.08);
          color: #4c1d95;
          text-transform: none;
          letter-spacing: 0.02em;
        }
        .pitch-swap-note {
          margin-top: 6px;
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.04em;
          color: rgba(127, 29, 29, 0.85);
        }
        .pitch-draft-block {
          margin-top: 10px;
          padding: 10px 12px;
          border-radius: 10px;
          background: rgba(239, 68, 68, 0.05);
          border: 1px dashed rgba(239, 68, 68, 0.35);
          font-family: var(--font-mono);
          font-size: 12px;
          line-height: 1.45;
          color: rgba(127, 29, 29, 0.85);
          white-space: pre-wrap;
        }
        .pitch-draft-label {
          display: block;
          font-family: var(--font-mono);
          font-size: 10px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: rgba(127, 29, 29, 0.7);
          margin-bottom: 4px;
        }
        .pitch-error-panel {
          margin-top: 18px;
          padding: 14px 16px;
          border: 1px solid #fecaca;
          background: rgba(254, 226, 226, 0.6);
          border-radius: 12px;
          color: #7f1d1d;
          display: flex;
          gap: 10px;
          align-items: flex-start;
          font-family: "Inter", sans-serif;
          font-size: 13px;
        }
        @media (max-width: 720px) {
          .pitch-shell {
            margin: 18px auto 0;
          }
          .pitch-input {
            min-height: 80px;
          }
        }
      `}</style>
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
                <button
                  className="pitch-submit"
                  type="button"
                  onClick={() => void handleSubmit()}
                  disabled={pending || question.trim().length === 0}
                  data-testid="pitch-submit"
                >
                  <Mic size={14} />
                  {pending ? "Asking…" : "Ask"}
                </button>
              </div>
            </div>
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

function MessageBubble({ message }: { message: Message }) {
  if (message.role === "operator") {
    return (
      <div className="pitch-msg is-operator" data-testid="pitch-msg-operator">
        <p className="pitch-msg-text">{message.text}</p>
      </div>
    );
  }
  const meta = message.meta;
  const pill = meta ? dispositionPill(meta.disposition) : null;
  const source = meta ? sourcePill(meta.source) : null;
  const tokens = meta ? tokensLabel(meta.tokens) : null;
  const isSwapped = meta?.source === "llm_blocked_canonical_refused";
  return (
    <div className="pitch-msg is-agent" data-testid="pitch-msg-agent">
      <p className="pitch-msg-text">{message.text}</p>
      {isSwapped ? (
        <p className="pitch-swap-note" data-testid="pitch-swap-note">
          (canonical refusal substituted for the model&apos;s draft)
        </p>
      ) : null}
      {meta ? (
        <div className="pitch-meta">
          {pill ? (
            <span className={pill.className} data-testid="pitch-pill-disposition">
              <pill.Icon /> {pill.label}
            </span>
          ) : null}
          {source ? (
            <span
              className={source.className}
              data-testid="pitch-pill-source"
              title={
                meta?.model
                  ? `${source.label} · ${meta.model}`
                  : source.label
              }
            >
              <source.Icon /> {source.label}
            </span>
          ) : null}
          {meta.entry_id ? (
            <span className="pitch-pill is-entry" data-testid="pitch-pill-entry">
              {meta.entry_id}
            </span>
          ) : null}
          {meta.model ? (
            <span className="pitch-pill is-model" data-testid="pitch-pill-model">
              {meta.model}
            </span>
          ) : null}
          {tokens ? (
            <span className="pitch-pill is-tokens" data-testid="pitch-pill-tokens">
              {tokens}
            </span>
          ) : null}
          <span className="pitch-pill is-latency" data-testid="pitch-pill-latency">
            {meta.latency_ms}ms · {meta.channel}
          </span>
        </div>
      ) : null}
      {meta?.draft && isSwapped ? (
        <div className="pitch-draft-block" data-testid="pitch-draft">
          <span className="pitch-draft-label">Model draft (audited, not shipped)</span>
          {meta.draft}
        </div>
      ) : null}
    </div>
  );
}
