"use client";

import { useCallback, useState } from "react";
import {
  AlertTriangle,
  Inbox,
  Send,
  ShieldCheck,
  ShieldQuestion,
  Sparkles,
} from "lucide-react";
import Link from "next/link";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import {
  draftFollowup,
  type GtmDisposition,
  type GtmDraftResponse,
  type GtmTier,
} from "@/lib/api/gtm";
import { ApiError } from "@/lib/api/types";

/**
 * `/gtm` — operator-facing console for Grant, the GTM (go-to-market)
 * Engineer. Paste a waitlist signup; Grant drafts a brand-safe inbound
 * follow-up via `POST /api/v1/gtm/draft-followup`, truth-bounded, and
 * files it to the approval queue. No mock fallback — every draft goes
 * to the gateway (same LLM + truth-boundary path Portia uses).
 *
 * A *responsibility*, not a task: Grant owns inbound growth, drafts on
 * demand, escalates when a draft would breach approved facts, and never
 * auto-sends — the operator approves in `/approvals`.
 */

const NAME_PLACEHOLDER = "Dana Reed";
const EMAIL_PLACEHOLDER = "dana@brightloop.io";
const CONTEXT_PLACEHOLDER =
  "Head of Ops at a 40-person logistics startup. Found us via a post about AI teammates taking on responsibilities, not tasks. Wants to cut manual status reporting.";

function dispositionPill(disposition: GtmDisposition) {
  if (disposition === "Allow") {
    return { Icon: ShieldCheck, label: "Brand-safe", className: "gtm-pill is-allow" };
  }
  return { Icon: ShieldQuestion, label: "Held for review", className: "gtm-pill is-escalate" };
}

function tierClass(tier: GtmTier): string {
  if (tier === "Hot") return "gtm-tier is-hot";
  if (tier === "Warm") return "gtm-tier is-warm";
  return "gtm-tier is-cold";
}

export default function GtmPage() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [context, setContext] = useState("");
  const [result, setResult] = useState<GtmDraftResponse | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = useCallback(async () => {
    const trimmedName = name.trim();
    if (!trimmedName || pending) return;
    setPending(true);
    setError(null);
    setResult(null);
    try {
      const res = await draftFollowup({
        name: trimmedName,
        email: email.trim(),
        context: context.trim() || undefined,
      });
      setResult(res);
    } catch (err) {
      const message =
        err instanceof ApiError && err.message
          ? err.message
          : err instanceof Error && err.message
            ? err.message
            : "Couldn't reach Grant.";
      setError(message);
    } finally {
      setPending(false);
    }
  }, [name, email, context, pending]);

  const pill = result ? dispositionPill(result.disposition) : null;
  const tokens = result?.tokens;
  const tokensLabel =
    tokens && (tokens.input != null || tokens.output != null)
      ? `${tokens.input != null ? `in ${tokens.input}` : ""}${
          tokens.input != null && tokens.output != null ? " · " : ""
        }${tokens.output != null ? `out ${tokens.output}` : ""}`
      : null;

  return (
    <>
      <style jsx global>{`
        .gtm-page-bg {
          position: relative;
          min-height: calc(100vh - 64px);
          background: var(--background);
          overflow: hidden;
        }
        .gtm-page-bg::before {
          content: "";
          position: absolute;
          top: -180px;
          right: -120px;
          width: 540px;
          height: 540px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.18) 0%, transparent 70%);
          pointer-events: none;
        }
        .gtm-page-bg::after {
          content: "";
          position: absolute;
          bottom: -200px;
          left: -120px;
          width: 600px;
          height: 600px;
          background: radial-gradient(circle, rgba(8, 145, 178, 0.12) 0%, transparent 70%);
          pointer-events: none;
        }
        .gtm-page-content {
          position: relative;
          z-index: 1;
          padding-bottom: 72px;
        }
        .gtm-shell {
          max-width: 720px;
          margin: 28px auto 0;
          padding: 0 16px;
        }
        .gtm-input-card {
          display: flex;
          flex-direction: column;
          gap: 12px;
          padding: 18px;
          background: var(--card);
          border: 1px solid rgba(139, 92, 246, 0.18);
          border-radius: 16px;
          box-shadow: 0 12px 36px -22px rgba(76, 29, 149, 0.35);
        }
        .gtm-field-label {
          font-family: var(--font-mono);
          font-size: 11px;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          color: var(--muted-foreground);
          margin-bottom: 4px;
          display: block;
        }
        .gtm-field {
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
        }
        .gtm-field:focus {
          border-color: #8b5cf6;
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.18);
        }
        textarea.gtm-field {
          min-height: 84px;
          resize: vertical;
        }
        .gtm-row {
          display: flex;
          gap: 12px;
        }
        .gtm-row > div {
          flex: 1;
        }
        .gtm-actions {
          display: flex;
          align-items: center;
          justify-content: flex-end;
          gap: 12px;
        }
        .gtm-submit {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 10px 18px;
          border-radius: 12px;
          border: none;
          background: linear-gradient(135deg, #8b5cf6, #0891b2);
          color: white;
          font-family: "Inter", sans-serif;
          font-size: 14px;
          font-weight: 600;
          cursor: pointer;
          box-shadow: 0 8px 22px -12px rgba(139, 92, 246, 0.65);
        }
        .gtm-submit:disabled {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .gtm-result {
          margin-top: 24px;
          padding: 18px;
          background: var(--card);
          border: 1px solid var(--border);
          border-radius: 16px;
          box-shadow: 0 10px 30px -22px rgba(15, 23, 42, 0.4);
        }
        .gtm-result-head {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: 8px;
          margin-bottom: 12px;
        }
        .gtm-qual {
          margin-bottom: 14px;
          padding-bottom: 14px;
          border-bottom: 1px solid var(--border);
        }
        .gtm-qual-head {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
          gap: 10px;
        }
        .gtm-tier {
          display: inline-flex;
          align-items: center;
          padding: 4px 12px;
          border-radius: 999px;
          font-family: var(--font-mono);
          font-size: 11px;
          font-weight: 700;
          letter-spacing: 0.1em;
          text-transform: uppercase;
        }
        .gtm-tier.is-hot {
          background: rgba(239, 68, 68, 0.12);
          color: #b91c1c;
        }
        .gtm-tier.is-warm {
          background: rgba(245, 158, 11, 0.16);
          color: #92400e;
        }
        .gtm-tier.is-cold {
          background: rgba(100, 116, 139, 0.14);
          color: #475569;
        }
        .gtm-qual-rationale {
          font-family: "Inter", sans-serif;
          font-size: 13px;
          color: var(--foreground);
        }
        .gtm-signals {
          display: flex;
          flex-wrap: wrap;
          gap: 6px;
          margin-top: 10px;
        }
        .gtm-signal {
          display: inline-flex;
          align-items: center;
          padding: 3px 9px;
          border-radius: 8px;
          background: rgba(139, 92, 246, 0.08);
          color: #5b21b6;
          font-family: "Inter", sans-serif;
          font-size: 11px;
          font-weight: 500;
        }
        .gtm-draft {
          margin: 0;
          white-space: pre-wrap;
          font-family: "Inter", sans-serif;
          font-size: 15px;
          line-height: 1.6;
          color: var(--foreground);
        }
        .gtm-filed {
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
        .gtm-filed:hover {
          background: rgba(139, 92, 246, 0.16);
        }
        .gtm-filed svg {
          width: 15px;
          height: 15px;
        }
        .gtm-pill {
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
        .gtm-pill svg {
          width: 12px;
          height: 12px;
        }
        .gtm-pill.is-allow {
          background: rgba(34, 197, 94, 0.12);
          color: #166534;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .gtm-pill.is-escalate {
          background: rgba(245, 158, 11, 0.14);
          color: #92400e;
          text-transform: uppercase;
          letter-spacing: 0.1em;
        }
        .gtm-pill.is-source {
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.18), rgba(8, 145, 178, 0.12));
          color: #5b21b6;
        }
        .gtm-pill.is-model {
          background: rgba(76, 29, 149, 0.08);
          color: #4c1d95;
        }
        .gtm-pill.is-tokens {
          background: rgba(15, 23, 42, 0.04);
          color: var(--muted-foreground);
          font-variant-numeric: tabular-nums;
        }
        .gtm-error-panel {
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
          .gtm-row {
            flex-direction: column;
          }
        }
      `}</style>
      <div className="gtm-page-bg">
        <div className="gtm-page-content">
          <BrandLoudHero
            eyebrowDotColor="#67e8f9"
            eyebrowLabel="GTM AGENT — GRANT"
            pulseDot
            title="Watch Grant turn a signup into a follow-up."
            titleAccent="follow-up"
            sub="Paste a waitlist signup. Grant drafts a brand-safe outreach note — truth-bounded, filed to your approval queue, never auto-sent."
            subMaxWidth={540}
          />
          <section className="gtm-shell" aria-label="GTM Agent console">
            <div className="gtm-input-card">
              <div className="gtm-row">
                <div>
                  <label className="gtm-field-label" htmlFor="gtm-name">
                    Name
                  </label>
                  <input
                    id="gtm-name"
                    className="gtm-field"
                    placeholder={NAME_PLACEHOLDER}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    disabled={pending}
                    data-testid="gtm-name"
                  />
                </div>
                <div>
                  <label className="gtm-field-label" htmlFor="gtm-email">
                    Email
                  </label>
                  <input
                    id="gtm-email"
                    className="gtm-field"
                    placeholder={EMAIL_PLACEHOLDER}
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    disabled={pending}
                    data-testid="gtm-email"
                  />
                </div>
              </div>
              <div>
                <label className="gtm-field-label" htmlFor="gtm-context">
                  Context (optional)
                </label>
                <textarea
                  id="gtm-context"
                  className="gtm-field"
                  placeholder={CONTEXT_PLACEHOLDER}
                  value={context}
                  onChange={(e) => setContext(e.target.value)}
                  disabled={pending}
                  data-testid="gtm-context"
                />
              </div>
              <div className="gtm-actions">
                <button
                  className="gtm-submit"
                  type="button"
                  onClick={() => void handleSubmit()}
                  disabled={pending || name.trim().length === 0}
                  data-testid="gtm-submit"
                >
                  <Send size={14} />
                  {pending ? "Drafting…" : "Draft follow-up"}
                </button>
              </div>
            </div>
            {error ? (
              <div className="gtm-error-panel" role="alert" data-testid="gtm-error">
                <AlertTriangle size={16} />
                <span>{error}</span>
              </div>
            ) : null}
            {result ? (
              <div className="gtm-result" data-testid="gtm-result">
                {result.qualification ? (
                  <div className="gtm-qual" data-testid="gtm-qualification">
                    <div className="gtm-qual-head">
                      <span
                        className={tierClass(result.qualification.tier)}
                        data-testid="gtm-tier"
                      >
                        {result.qualification.tier} lead
                      </span>
                      <span className="gtm-qual-rationale">
                        {result.qualification.rationale}
                      </span>
                    </div>
                    {result.qualification.signals.length > 0 ? (
                      <div className="gtm-signals">
                        {result.qualification.signals.map((s) => (
                          <span className="gtm-signal" key={s}>
                            {s}
                          </span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                ) : null}
                <div className="gtm-result-head">
                  {pill ? (
                    <span className={pill.className} data-testid="gtm-pill-disposition">
                      <pill.Icon /> {pill.label}
                    </span>
                  ) : null}
                  <span className="gtm-pill is-source" data-testid="gtm-pill-source">
                    <Sparkles /> {result.source ?? "llm"}
                  </span>
                  {result.model ? (
                    <span className="gtm-pill is-model" data-testid="gtm-pill-model">
                      {result.model}
                    </span>
                  ) : null}
                  {tokensLabel ? (
                    <span className="gtm-pill is-tokens" data-testid="gtm-pill-tokens">
                      {tokensLabel}
                    </span>
                  ) : null}
                </div>
                <p className="gtm-draft" data-testid="gtm-draft">
                  {result.draft}
                </p>
                {result.approval_id ? (
                  <Link className="gtm-filed" href="/approvals" data-testid="gtm-filed">
                    <Inbox /> Filed to approvals — review before it sends
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
