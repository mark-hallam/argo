"use client";

/**
 * /coach — Cora, the founder coach (live two-way voice).
 *
 * Cora is a distinct teammate from Portia (the pitch agent): she coaches the
 * founder, and can run a founder-story interview. Both run over the *same*
 * truth-bounded live-voice path as the pitch (WebRTC → OpenAI Realtime, the
 * ephemeral token minted server-side with Cora's persona instructions baked
 * in — the real API key never reaches the browser).
 *
 * Two modes, picked while idle:
 *   - coach     — honest, direct coaching (instructed: a coach, not a cheerleader).
 *   - interview — Cora interviews Mark to draw out the founder story.
 *
 * Truth boundary here is LLM-primary: a streamed voice answer can't run
 * SKYCOT's deterministic post-check, so the live label says "model-bounded",
 * never the green deterministic "Allowed" pill (NO THEATER — DEC-007/029).
 */

import { useEffect, useState } from "react";
import { useRealtimeVoice, realtimeVoiceSupported } from "@/lib/api/use-realtime-voice";
import type { RealtimeMode } from "@/lib/api/voice";

const MODES: ReadonlyArray<{ id: RealtimeMode; label: string; blurb: string }> = [
  {
    id: "coach",
    label: "Coaching",
    blurb: "Cora coaches you — honest and direct, never a cheerleader.",
  },
  {
    id: "interview",
    label: "Founder interview",
    blurb: "Cora interviews you to draw out your founder story.",
  },
];

export default function CoachPage() {
  const [mode, setMode] = useState<RealtimeMode>("coach");
  const [supported, setSupported] = useState(false);
  const live = useRealtimeVoice({ mode });

  // SSR-safe: resolve browser capability after mount to avoid a hydration mismatch.
  useEffect(() => {
    setSupported(realtimeVoiceSupported());
  }, []);

  const busy = live.phase === "connecting" || live.phase === "live";
  const active = MODES.find((m) => m.id === mode) ?? MODES[0];

  return (
    <main className="coach">
      <header className="head">
        <p className="eyebrow">SKYCOT · founder coach</p>
        <h1 className="word">Cora</h1>
        <p className="tag">{active.blurb}</p>
      </header>

      <section className="modes" aria-label="Conversation mode">
        {MODES.map((m) => (
          <button
            key={m.id}
            type="button"
            className={`mode ${m.id === mode ? "on" : ""}`}
            aria-pressed={m.id === mode}
            disabled={busy}
            onClick={() => setMode(m.id)}
          >
            {m.label}
          </button>
        ))}
      </section>

      <section className="panel">
        {!supported ? (
          <p className="note">
            This browser can&apos;t do live voice yet — it needs microphone + WebRTC
            access. Open SKYCOT in Chrome with mic permission allowed.
          </p>
        ) : (
          <>
            <div className="controls">
              {busy ? (
                <button type="button" className="cta stop" onClick={live.stop}>
                  End conversation
                </button>
              ) : (
                <button type="button" className="cta go" onClick={() => void live.start()}>
                  Talk to Cora
                </button>
              )}
              <span className={`status ${live.phase}`}>
                {live.phase === "idle" && "Ready"}
                {live.phase === "connecting" && "Connecting…"}
                {live.phase === "live" && "● LIVE · model-bounded"}
                {live.phase === "error" && "Error"}
              </span>
            </div>

            {live.error && <p className="err">{live.error}</p>}

            {(live.transcripts.length > 0 || live.liveCaption) && (
              <ol className="convo">
                {live.transcripts.map((t, i) => (
                  <li key={i} className={`line ${t.role}`}>
                    <span className="who">{t.role === "user" ? "You" : "Cora"}</span>
                    <span className="said">{t.text}</span>
                  </li>
                ))}
                {live.liveCaption && (
                  <li className="line agent live">
                    <span className="who">Cora</span>
                    <span className="said">{live.liveCaption}</span>
                  </li>
                )}
              </ol>
            )}

            <p className="boundary">
              Live answers are <strong>model-bounded</strong>: Cora is instructed to
              coach honestly and stay within what&apos;s true, but a streamed voice
              answer can&apos;t run SKYCOT&apos;s deterministic truth-check — so this
              isn&apos;t the green &ldquo;Allowed&rdquo; pill.
            </p>
          </>
        )}
      </section>

      <style jsx>{`
        .coach {
          max-width: 720px;
          margin: 0 auto;
          padding: 48px 24px 80px;
          font-family: "Inter", system-ui, sans-serif;
          color: #1e1b4b;
        }
        .head {
          text-align: center;
          margin-bottom: 32px;
        }
        .eyebrow {
          font-size: 12px;
          letter-spacing: 0.18em;
          text-transform: uppercase;
          font-weight: 600;
          margin: 0 0 8px;
          background: linear-gradient(90deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .word {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 56px;
          line-height: 1;
          margin: 0 0 12px;
          background: linear-gradient(90deg, #8b5cf6, #ec4899);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .tag {
          font-size: 17px;
          color: #4b5563;
          margin: 0;
        }
        .modes {
          display: flex;
          gap: 8px;
          justify-content: center;
          margin-bottom: 28px;
        }
        .mode {
          border: 1px solid #e5e7eb;
          background: #fff;
          color: #4b5563;
          padding: 10px 18px;
          border-radius: 999px;
          font-size: 14px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.15s;
        }
        .mode:hover:not(:disabled) {
          border-color: #c4b5fd;
        }
        .mode.on {
          color: #fff;
          border-color: transparent;
          background: linear-gradient(90deg, #8b5cf6, #ec4899);
        }
        .mode:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .panel {
          background: #fff;
          border: 1px solid #eee;
          border-radius: 20px;
          padding: 28px;
          box-shadow: 0 8px 40px rgba(139, 92, 246, 0.08);
        }
        .controls {
          display: flex;
          align-items: center;
          gap: 16px;
        }
        .cta {
          border: none;
          border-radius: 999px;
          padding: 14px 28px;
          font-size: 16px;
          font-weight: 700;
          cursor: pointer;
          color: #fff;
        }
        .cta.go {
          background: linear-gradient(90deg, #8b5cf6, #ec4899);
        }
        .cta.stop {
          background: #1e1b4b;
        }
        .status {
          font-size: 13px;
          font-weight: 600;
          color: #6b7280;
        }
        .status.live {
          color: #ec4899;
        }
        .status.error {
          color: #dc2626;
        }
        .err {
          margin: 16px 0 0;
          padding: 12px 16px;
          border-radius: 12px;
          background: #fef2f2;
          color: #b91c1c;
          font-size: 14px;
        }
        .convo {
          list-style: none;
          padding: 0;
          margin: 24px 0 0;
          display: grid;
          gap: 12px;
        }
        .line {
          display: grid;
          grid-template-columns: 56px 1fr;
          gap: 12px;
          font-size: 15px;
        }
        .line .who {
          font-weight: 700;
          font-size: 12px;
          color: #8b5cf6;
          text-transform: uppercase;
        }
        .line.user .who {
          color: #6b7280;
        }
        .line .said {
          color: #1e1b4b;
        }
        .line.live .said {
          color: #6b7280;
          font-style: italic;
        }
        .boundary {
          margin: 24px 0 0;
          font-size: 12.5px;
          line-height: 1.5;
          color: #6b7280;
          border-top: 1px solid #f3f4f6;
          padding-top: 16px;
        }
        .note {
          margin: 0;
          font-size: 15px;
          color: #4b5563;
          line-height: 1.6;
        }
      `}</style>
    </main>
  );
}
