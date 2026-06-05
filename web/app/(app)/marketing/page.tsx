"use client";

import { useCallback, useMemo, useState } from "react";
import {
  AlertTriangle,
  Inbox,
  Linkedin,
  PenLine,
  Send,
  ShieldCheck,
  ShieldQuestion,
  Sparkles,
  Twitter,
} from "lucide-react";
import Link from "next/link";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import {
  draftResponse,
  type MarketingChannel,
  type MarketingDisposition,
  type MarketingDraftResponse,
  type MarketingMode,
} from "@/lib/api/marketing";
import { ApiError } from "@/lib/api/types";

/**
 * `/marketing` — operator-facing console for Marcus, the marketing
 * teammate (DEC-028, the third hero teammate after Portia + Grant).
 * Paste a mention / URL (reply mode) or give a topic (article mode),
 * pick a channel (X short / LinkedIn long-form), and Marcus drafts a
 * truth-bounded piece in Mark's voice via
 * `POST /api/v1/marketing/draft-response` — filed to the approval queue,
 * NEVER posted. No mock fallback — every draft goes to the gateway (same
 * LLM + truth-boundary path Portia + Grant use).
 *
 * A *responsibility*, not a task: Marcus owns top-of-funnel marketing +
 * social, drafts on demand, holds a draft that would breach approved
 * facts, and never auto-posts — the operator hand-posts the approved
 * draft (there is no posting integration in v0).
 */

const REPLY_PLACEHOLDER =
  "Paste the post / mention / URL you want Marcus to reply to. e.g. a tweet: \"AI agents are just fancy autocomplete — they can't actually own anything.\"";
const ARTICLE_PLACEHOLDER =
  "Give Marcus a topic. e.g. \"Why ideas are still the human moat in the building loop\" — optionally add framing on the next line.";

function dispositionPill(disposition: MarketingDisposition) {
  if (disposition === "Allow") {
    return { Icon: ShieldCheck, label: "Brand-safe", className: "mk-pill is-allow" };
  }
  return { Icon: ShieldQuestion, label: "Held for review", className: "mk-pill is-escalate" };
}

export default function MarketingPage() {
  const [mode, setMode] = useState<MarketingMode>("reply");
  const [channel, setChannel] = useState<MarketingChannel>("x");
  const [text, setText] = useState("");
  const [result, setResult] = useState<MarketingDraftResponse | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const placeholder = mode === "reply" ? REPLY_PLACEHOLDER : ARTICLE_PLACEHOLDER;
  const fieldLabel = mode === "reply" ? "Mention / post / URL" : "Topic";

  const handleSubmit = useCallback(async () => {
    const trimmed = text.trim();
    if (!trimmed || pending) return;
    setPending(true);
    setError(null);
    setResult(null);
    try {
      const res = await draftResponse({
        mode,
        channel,
        source: mode === "reply" ? trimmed : undefined,
        topic: mode === "article" ? trimmed : undefined,
      });
      setResult(res);
    } catch (err) {
      const message =
        err instanceof ApiError && err.message
          ? err.message
          : err instanceof Error && err.message
            ? err.message
            : "Couldn't reach Marcus.";
      setError(message);
    } finally {
      setPending(false);
    }
  }, [mode, channel, text, pending]);

  const pill = result ? dispositionPill(result.disposition) : null;
  const tokens = result?.tokens;
  const tokensLabel = useMemo(() => {
    if (!tokens || (tokens.input == null && tokens.output == null)) return null;
    const inPart = tokens.input != null ? `in ${tokens.input}` : "";
    const outPart = tokens.output != null ? `out ${tokens.output}` : "";
    const sep = tokens.input != null && tokens.output != null ? " · " : "";
    return `${inPart}${sep}${outPart}`;
  }, [tokens]);

  return (
    <>
      <style jsx global>{`
        .mk-page-bg {
          position: relative;
          min-height: calc(100vh - 64px);
          background: var(--background);
          overflow: hidden;
        }
        .mk-page-bg::before {
          content: "";
          position: absolute;
          top: -180px;
          right: -120px;
          width: 540px;
          height: 540px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.18) 0%, transparent 70%);
          pointer-events: none;
        }
        .mk-page-bg::after {
          content: "";
          position: absolute;
          bottom: -200px;
          left: -120px;
          width: 600px;
          height: 600px;
          background: radial-gradient(circle, rgba(236, 72, 153, 0.12) 0%, transparent 70%);
          pointer-events: none;
        }
        .mk-page-content {
          position: relative;
          z-index: 1;
          padding-bottom: 72px;
        }
        .mk-shell {
          max-width: 720px;
          margin: 28px auto 0;
          padding: 0 16px;
        }
        .mk-input-card {
          display: flex;
          flex-direction: column;
          gap: 14px;
          padding: 18px;
          background: var(--card);
          border: 1px solid rgba(139, 92, 246, 0.18);
          border-radius: 16px;
          box-shadow: 0 12px 36px -22px rgba(76, 29, 149, 0.35);
        }
        .mk-toggle-row {
          display: flex;
          flex-wrap: wrap;
          gap: 16px;
        }
        .mk-toggle-group {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .mk-toggle-label {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: var(--muted-foreground);
        }
        .mk-segmented {
          display: inline-flex;
          padding: 3px;
          background: rgba(139, 92, 246, 0.08);
          border-radius: 12px;
          gap: 3px;
        }
        .mk-seg {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 7px 14px;
          border: none;
          background: transparent;
          border-radius: 9px;
          font-family: "Inter", sans-serif;
          font-size: 13px;
          font-weight: 600;
          color: var(--muted-foreground);
          cursor: pointer;
        }
        .mk-seg svg {
          width: 14px;
          height: 14px;
        }
        .mk-seg.is-active {
          background: var(--card);
          color: #5b21b6;
          box-shadow: 0 4px 12px -6px rgba(76, 29, 149, 0.4);
        }
        .mk-seg:disabled {
          cursor: not-allowed;
          opacity: 0.6;
        }
        .mk-field-label {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: var(--muted-foreground);
          margin-bottom: 4px;
          display: block;
        }
        .mk-field {
          width: 100%;
          padding: 12px 14px;
          border: 1px solid var(--border);
          border-radius: 12px;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          line-height: 1.5;
          color: var(--foreground);
          background: var(--card);
          outline: none;
          min-height: 110px;
          resize: vertical;
        }
        .mk-field:focus {
          border-color: #8b5cf6;
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.18);
        }
        .mk-actions {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
        }
        .mk-note {
          font-family: "Inter", sans-serif;
          font-size: 12px;
          color: var(--muted-foreground);
        }
        .mk-submit {
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
          cursor: pointer;
          box-shadow: 0 8px 22px -12px rgba(139, 92, 246, 0.65);
        }
        .mk-submit:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .mk-result {
          margin-top: 24px;
          padding: 18px;
          background: var(--card);
          border: 1px solid var(--border);
          border-radius: 16px;
          box-shadow: 0 10px 30px -22px rgba(15, 23, 42, 0.4);
        }
        .mk-result-head {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: 8px;
          margin-bottom: 12px;
        }
        .mk-draft {
          margin: 0;
          white-space: pre-wrap;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          line-height: 1.6;
          color: var(--foreground);
        }
        .mk-filed {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          margin-top: 16px;
          padding: 8px 14px;
          border-radius: 12px;
          background: rgba(139, 92, 246, 0.08);
          color: #5b21b6;
          font-family: "Inter", sans-serif;
          font-size: 13px;
          font-weight: 600;
          text-decoration: none;
        }
        .mk-filed:hover {
          background: rgba(139, 92, 246, 0.16);
        }
        .mk-filed svg {
          width: 15px;
          height: 15px;
        }
        .mk-pill {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 4px 10px;
          border-radius: 999px;
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.06em;
          font-weight: 600;
        }
        .mk-pill svg {
          width: 12px;
          height: 12px;
        }
        .mk-pill.is-allow {
          background: rgba(34, 197, 94, 0.12);
          color: #166534;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .mk-pill.is-escalate {
          background: rgba(245, 158, 11, 0.14);
          color: #92400e;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .mk-pill.is-channel {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.18), rgba(236, 72, 153, 0.12));
          color: #5b21b6;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .mk-pill.is-source {
          background: rgba(139, 92, 246, 0.1);
          color: #5b21b6;
        }
        .mk-pill.is-model {
          background: rgba(76, 29, 149, 0.08);
          color: #4c1d95;
        }
        .mk-pill.is-tokens {
          background: rgba(15, 23, 42, 0.04);
          color: var(--muted-foreground);
          font-variant-numeric: tabular-nums;
        }
        .mk-error-panel {
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
        @media (max-width: 600px) {
          .mk-actions {
            flex-direction: column;
            align-items: stretch;
          }
        }
      `}</style>
      <div className="mk-page-bg">
        <div className="mk-page-content">
          <BrandLoudHero
            eyebrowDotColor="#f9a8d4"
            eyebrowLabel="MARKETING — MARCUS"
            pulseDot
            title="Watch Marcus draft your public voice."
            titleAccent="public voice"
            sub="Paste a mention or give a topic. Marcus drafts a reply or an article in Mark's voice — truth-bounded, filed to your approval queue, never auto-posted. You post the approved draft."
            subMaxWidth={560}
          />
          <section className="mk-shell" aria-label="Marketing console">
            <div className="mk-input-card">
              <div className="mk-toggle-row">
                <div className="mk-toggle-group">
                  <span className="mk-toggle-label">Mode</span>
                  <div className="mk-segmented" role="group" aria-label="Draft mode">
                    <button
                      type="button"
                      className={`mk-seg${mode === "reply" ? " is-active" : ""}`}
                      onClick={() => setMode("reply")}
                      disabled={pending}
                      aria-pressed={mode === "reply"}
                      data-testid="mk-mode-reply"
                    >
                      <Send /> Reply
                    </button>
                    <button
                      type="button"
                      className={`mk-seg${mode === "article" ? " is-active" : ""}`}
                      onClick={() => setMode("article")}
                      disabled={pending}
                      aria-pressed={mode === "article"}
                      data-testid="mk-mode-article"
                    >
                      <PenLine /> Article
                    </button>
                  </div>
                </div>
                <div className="mk-toggle-group">
                  <span className="mk-toggle-label">Channel</span>
                  <div className="mk-segmented" role="group" aria-label="Channel">
                    <button
                      type="button"
                      className={`mk-seg${channel === "x" ? " is-active" : ""}`}
                      onClick={() => setChannel("x")}
                      disabled={pending}
                      aria-pressed={channel === "x"}
                      data-testid="mk-channel-x"
                    >
                      <Twitter /> X
                    </button>
                    <button
                      type="button"
                      className={`mk-seg${channel === "linkedin" ? " is-active" : ""}`}
                      onClick={() => setChannel("linkedin")}
                      disabled={pending}
                      aria-pressed={channel === "linkedin"}
                      data-testid="mk-channel-linkedin"
                    >
                      <Linkedin /> LinkedIn
                    </button>
                  </div>
                </div>
              </div>
              <div>
                <label className="mk-field-label" htmlFor="mk-text">
                  {fieldLabel}
                </label>
                <textarea
                  id="mk-text"
                  className="mk-field"
                  placeholder={placeholder}
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  disabled={pending}
                  data-testid="mk-text"
                />
              </div>
              <div className="mk-actions">
                <span className="mk-note">
                  Draft-only — Marcus never posts. You hand-post the approved draft.
                </span>
                <button
                  className="mk-submit"
                  type="button"
                  onClick={() => void handleSubmit()}
                  disabled={pending || text.trim().length === 0}
                  data-testid="mk-submit"
                >
                  <Sparkles size={14} />
                  {pending ? "Drafting…" : "Draft it"}
                </button>
              </div>
            </div>
            {error ? (
              <div className="mk-error-panel" role="alert" data-testid="mk-error">
                <AlertTriangle size={16} />
                <span>{error}</span>
              </div>
            ) : null}
            {result ? (
              <div className="mk-result" data-testid="mk-result">
                <div className="mk-result-head">
                  {pill ? (
                    <span className={pill.className} data-testid="mk-pill-disposition">
                      <pill.Icon /> {pill.label}
                    </span>
                  ) : null}
                  <span className="mk-pill is-channel" data-testid="mk-pill-channel">
                    {result.channel === "linkedin" ? "LinkedIn" : "X"} · {result.mode}
                  </span>
                  <span className="mk-pill is-source" data-testid="mk-pill-source">
                    <Sparkles /> {result.source ?? "llm"}
                  </span>
                  {result.model ? (
                    <span className="mk-pill is-model" data-testid="mk-pill-model">
                      {result.model}
                    </span>
                  ) : null}
                  {tokensLabel ? (
                    <span className="mk-pill is-tokens" data-testid="mk-pill-tokens">
                      {tokensLabel}
                    </span>
                  ) : null}
                </div>
                <p className="mk-draft" data-testid="mk-draft">
                  {result.draft}
                </p>
                {result.approval_id ? (
                  <Link className="mk-filed" href="/approvals" data-testid="mk-filed">
                    <Inbox /> Filed to approvals — review before you post it
                  </Link>
                ) : null}
              </div>
            ) : null}
          </section>
        </div>
      </div>
    </>
  );
}
