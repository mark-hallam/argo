"use client";

/**
 * `PastVoiceSessionsPanel` — history of live-voice conversations for one
 * persona's mode (Portia's `pitch`, Cora's `coach` / `interview`).
 *
 * Reads `GET /api/v1/voice/sessions` (via {@link listVoiceSessions}), which
 * groups the tenant's `voice.transcript_turn` audit rows by `session_id`.
 * Each row is a hash-chained, tamper-evident audit record — so this panel is
 * the moat applied to conversation: *every word your AI teammate said, to you
 * or a customer, logged and provable.*
 *
 * Collapsed rows show time · turn count · preview; clicking one expands the
 * full both-sides transcript, reusing the live `/coach` `.convo` turn styling.
 * Mounted below the live panel on `/pitch` (pitch) and `/coach` (coach).
 */

import { useCallback, useEffect, useState } from "react";
import { listVoiceSessions, type RealtimeMode, type VoiceSession } from "@/lib/api/voice";

export interface PastVoiceSessionsPanelProps {
  /** Only sessions recorded in this mode are listed. */
  mode: RealtimeMode;
  /** Display name for the teammate's turns (e.g. `"Portia"`, `"Cora"`). */
  agentName?: string;
  /** Bump to force a reload — e.g. when a live call ends. */
  refreshSignal?: number;
}

/** Compact "2 min ago" / "3 h ago" / date, from an epoch-ms timestamp. */
export function relativeTime(ms: number): string {
  if (!ms || ms <= 0) return "";
  const diff = Math.max(0, Date.now() - ms);
  const min = Math.floor(diff / 60000);
  if (min < 1) return "just now";
  if (min < 60) return `${min} min ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr} h ago`;
  const days = Math.floor(hr / 24);
  if (days < 7) return `${days} d ago`;
  return new Date(ms).toLocaleDateString();
}

export function PastVoiceSessionsPanel({
  mode,
  agentName = "Agent",
  refreshSignal = 0,
}: PastVoiceSessionsPanelProps) {
  const [sessions, setSessions] = useState<VoiceSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSessions(await listVoiceSessions(mode));
    } catch {
      setError("Couldn't load past conversations.");
    } finally {
      setLoading(false);
    }
  }, [mode]);

  useEffect(() => {
    void load();
  }, [load, refreshSignal]);

  return (
    <section className="past" aria-label="Past voice conversations">
      <header className="phead">
        <h2 className="ptitle">Past conversations</h2>
        <button type="button" className="refresh" onClick={() => void load()} disabled={loading}>
          {loading ? "Loading…" : "Refresh"}
        </button>
      </header>

      {error && <p className="pnote err">{error}</p>}

      {!error && !loading && sessions.length === 0 && (
        <p className="pnote">
          No past conversations yet. Every live call with {agentName} is transcribed —
          both sides — and written to the tamper-evident audit trail. They&apos;ll appear
          here so you can replay exactly what was said.
        </p>
      )}

      {sessions.length > 0 && (
        <ul className="slist">
          {sessions.map((s) => {
            const open = openId === s.session_id;
            return (
              <li key={s.session_id} className={`srow ${open ? "open" : ""}`}>
                <button
                  type="button"
                  className="ssum"
                  aria-expanded={open}
                  onClick={() => setOpenId(open ? null : s.session_id)}
                >
                  <span className="meta">
                    <span className="when">{relativeTime(s.ended_at)}</span>
                    <span className="count">
                      {s.turn_count} turn{s.turn_count === 1 ? "" : "s"}
                    </span>
                  </span>
                  <span className="preview">{s.preview || "(no transcript captured)"}</span>
                  <span className="chev" aria-hidden>
                    {open ? "▾" : "▸"}
                  </span>
                </button>
                {open && (
                  <ol className="convo">
                    {s.turns.map((t, i) => (
                      <li key={i} className={`line ${t.role}`}>
                        <span className="who">{t.role === "user" ? "You" : agentName}</span>
                        <span className="said">{t.text}</span>
                      </li>
                    ))}
                  </ol>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <style jsx>{`
        .past {
          background: var(--card);
          border: 1px solid var(--border);
          border-radius: 20px;
          padding: 24px 28px;
          box-shadow: 0 8px 40px rgba(139, 92, 246, 0.08);
          margin-top: 20px;
          font-family: "Inter", system-ui, sans-serif;
          color: var(--foreground);
        }
        .phead {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
        }
        .ptitle {
          font-size: 16px;
          font-weight: 700;
          margin: 0;
          color: var(--foreground);
        }
        .refresh {
          border: 1px solid var(--border);
          background: var(--card);
          color: var(--muted-foreground);
          padding: 6px 14px;
          border-radius: 999px;
          font-size: 12px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.15s;
        }
        .refresh:hover:not(:disabled) {
          border-color: #c4b5fd;
          color: #8b5cf6;
        }
        .refresh:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .pnote {
          margin: 16px 0 0;
          font-size: 14px;
          line-height: 1.6;
          color: var(--muted-foreground);
        }
        .pnote.err {
          color: #b91c1c;
        }
        .slist {
          list-style: none;
          padding: 0;
          margin: 16px 0 0;
          display: grid;
          gap: 10px;
        }
        .srow {
          border: 1px solid var(--border);
          border-radius: 14px;
          overflow: hidden;
          transition: border-color 0.15s;
        }
        .srow.open {
          border-color: #e9d5ff;
        }
        .ssum {
          width: 100%;
          display: grid;
          grid-template-columns: auto 1fr auto;
          align-items: center;
          gap: 14px;
          padding: 14px 16px;
          background: var(--card);
          border: none;
          cursor: pointer;
          text-align: left;
        }
        .ssum:hover {
          background: var(--muted);
        }
        .meta {
          display: flex;
          flex-direction: column;
          gap: 2px;
          min-width: 64px;
        }
        .when {
          font-size: 12px;
          font-weight: 700;
          color: #8b5cf6;
          white-space: nowrap;
        }
        .count {
          font-size: 11px;
          color: var(--muted-foreground);
          white-space: nowrap;
        }
        .preview {
          font-size: 14px;
          color: var(--muted-foreground);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .chev {
          font-size: 12px;
          color: #c4b5fd;
        }
        .convo {
          list-style: none;
          padding: 4px 16px 18px;
          margin: 0;
          display: grid;
          gap: 12px;
          border-top: 1px solid var(--border);
          background: var(--muted);
        }
        .line {
          display: grid;
          grid-template-columns: 56px 1fr;
          gap: 12px;
          font-size: 14.5px;
          padding-top: 12px;
        }
        .line .who {
          font-weight: 700;
          font-size: 12px;
          color: #8b5cf6;
          text-transform: uppercase;
        }
        .line.user .who {
          color: var(--muted-foreground);
        }
        .line .said {
          color: var(--foreground);
          line-height: 1.5;
        }
      `}</style>
    </section>
  );
}

export default PastVoiceSessionsPanel;
