"use client";

import { useCallback, useState } from "react";
import { ArrowRight, CalendarCheck, Loader2, AlertCircle, ShieldCheck } from "lucide-react";
import {
  submitEnterpriseInquiry,
  isValidEmail,
} from "@/lib/api/enterprise-inquiry";
import { ApiError } from "@/lib/api/types";

/**
 * "Book a governance walkthrough" — the sales-led conversion path for the
 * landing hero (web-book-governance-walkthrough-cta).
 *
 * The 2026-06-15 launch research recommends an enterprise/sales-led motion
 * (the trust/governance wedge to the C-suite), but the site's only existing
 * conversion path was the self-serve waitlist. This adds the missing
 * "talk to us / book a call" destination where trust-differentiated enterprise
 * deals are actually won.
 *
 * Flow (operator-decided 2026-06-15: hybrid form → scheduler, graceful-degrade):
 *   1. A short qualifying form POSTs to the EXISTING
 *      `POST /api/v1/enterprise-inquiry` route (reused via
 *      {@link submitEnterpriseInquiry} — no parallel route). The role + the
 *      "what would you hand to an AI teammate?" free-text are folded into the
 *      inquiry `message` so the call has context; the lead is captured even if
 *      no scheduler is wired.
 *   2. On a real 2xx, IF `NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL` is set, a
 *      scheduler embed (Calendly / Cal.com inline) renders so the qualified
 *      prospect self-books a slot.
 *   3. NO-THEATER graceful degrade (DEC-007): if that env var is unset, the
 *      form submission alone IS the inquiry — an honest "we'll be in touch"
 *      state renders and NO "book / pick a time" affordance is shown. We never
 *      promise a calendar that isn't wired (the dead-play-button lesson, #2892).
 *
 * Self-contained inline styling (CSS-variable driven) so it renders correctly
 * in the brand-locked hero without page-specific CSS classes.
 */

type Phase = "idle" | "submitting" | "queued" | "error";

export interface WalkthroughCtaProps {
  /** Origin tag for the inquiry's context line. */
  source?: string;
}

/** Resolved at render so tests can stub the env; unset → graceful degrade. */
function resolveSchedulerUrl(): string | null {
  const raw = process.env.NEXT_PUBLIC_WALKTHROUGH_SCHEDULER_URL;
  const trimmed = typeof raw === "string" ? raw.trim() : "";
  return trimmed.length > 0 ? trimmed : null;
}

export function WalkthroughCta({
  source = "landing-hero",
}: WalkthroughCtaProps) {
  const [email, setEmail] = useState("");
  const [company, setCompany] = useState("");
  const [name, setName] = useState("");
  const [role, setRole] = useState("");
  const [handoff, setHandoff] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const clearErrorOnEdit = useCallback(() => {
    setPhase((p) => (p === "error" ? "idle" : p));
  }, []);

  const submit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (phase === "submitting") return;
      const em = email.trim();
      const c = company.trim();
      const n = name.trim();
      const r = role.trim();
      if (!isValidEmail(em)) {
        setPhase("error");
        setErrorMsg("Enter a valid work email address.");
        return;
      }
      if (!c) {
        setPhase("error");
        setErrorMsg("Enter your company name.");
        return;
      }
      if (!n) {
        setPhase("error");
        setErrorMsg("Enter your name.");
        return;
      }
      setPhase("submitting");
      setErrorMsg(null);
      // Fold the qualifying context into the existing inquiry `message` so the
      // reused endpoint carries everything the sales call needs.
      const contextLines = [
        `Governance walkthrough request (${source}).`,
        r ? `Role: ${r}` : null,
        handoff.trim()
          ? `Would hand to an AI teammate: ${handoff.trim()}`
          : null,
      ].filter(Boolean);
      try {
        await submitEnterpriseInquiry({
          company: c,
          name: n,
          email: em,
          message: contextLines.join("\n"),
        });
        setPhase("queued");
      } catch (err) {
        setPhase("error");
        setErrorMsg(
          err instanceof ApiError && err.message
            ? err.message
            : "Couldn’t send your request. Please try again or email hello@skycot.com.",
        );
      }
    },
    [email, company, name, role, handoff, source, phase],
  );

  if (phase === "queued") {
    const schedulerUrl = resolveSchedulerUrl();
    return (
      <div className="wcta-done" data-testid="walkthrough-success">
        {schedulerUrl ? (
          <div data-testid="walkthrough-scheduler">
            <div className="wcta-done-head">
              <CalendarCheck size={22} className="wcta-done-icon" />
              <div>
                <p className="wcta-done-title">You’re qualified — pick a time.</p>
                <p className="wcta-done-sub">
                  Choose a slot below and we’ll bring the governance walkthrough
                  tailored to <strong>{company.trim()}</strong>.
                </p>
              </div>
            </div>
            <iframe
              className="wcta-embed"
              data-testid="walkthrough-scheduler-embed"
              src={schedulerUrl}
              title="Book a governance walkthrough"
              loading="lazy"
            />
            <p className="wcta-embed-fallback">
              Calendar not loading?{" "}
              <a href={schedulerUrl} target="_blank" rel="noopener noreferrer">
                Open the scheduler in a new tab
              </a>
              .
            </p>
          </div>
        ) : (
          <div className="wcta-done-head" data-testid="walkthrough-queued-honest">
            <ShieldCheck size={22} className="wcta-done-icon" />
            <div>
              <p className="wcta-done-title">
                Thanks — we’ll be in touch to schedule your walkthrough.
              </p>
              <p className="wcta-done-sub">
                We’ll reach out to <strong>{email.trim()}</strong> within one
                business day to find a time and tailor the session to{" "}
                <strong>{company.trim()}</strong>.
              </p>
            </div>
          </div>
        )}
        <style jsx>{`
          .wcta-done {
            display: flex;
            flex-direction: column;
            gap: 14px;
            padding: 18px 20px;
            border: 1px solid color-mix(in srgb, var(--primary) 35%, transparent);
            border-radius: 14px;
            background: color-mix(in srgb, var(--primary) 6%, var(--background));
          }
          .wcta-done-head {
            display: flex;
            gap: 12px;
            align-items: flex-start;
          }
          .wcta-done-icon {
            color: var(--primary);
            flex-shrink: 0;
            margin-top: 2px;
          }
          .wcta-done-title {
            margin: 0 0 4px;
            font-weight: 700;
            font-size: 16px;
            color: var(--foreground);
          }
          .wcta-done-sub {
            margin: 0;
            font-size: 14px;
            line-height: 1.5;
            color: var(--muted-foreground);
          }
          .wcta-embed {
            width: 100%;
            min-height: 560px;
            border: 1px solid var(--border);
            border-radius: 12px;
            background: var(--background);
          }
          .wcta-embed-fallback {
            margin: 0;
            font-size: 13px;
            color: var(--muted-foreground);
          }
          .wcta-embed-fallback a {
            color: var(--primary);
          }
        `}</style>
      </div>
    );
  }

  return (
    <form
      className="wcta-form"
      onSubmit={submit}
      noValidate
      data-testid="walkthrough-form"
      data-source={source}
    >
      <div className="wcta-grid">
        <input
          type="email"
          className="wcta-input"
          placeholder="Work email"
          aria-label="Work email"
          value={email}
          onChange={(ev) => {
            setEmail(ev.target.value);
            clearErrorOnEdit();
          }}
          disabled={phase === "submitting"}
          autoComplete="email"
          data-testid="walkthrough-email"
        />
        <input
          type="text"
          className="wcta-input"
          placeholder="Company"
          aria-label="Company"
          value={company}
          onChange={(ev) => {
            setCompany(ev.target.value);
            clearErrorOnEdit();
          }}
          disabled={phase === "submitting"}
          autoComplete="organization"
          data-testid="walkthrough-company"
        />
        <input
          type="text"
          className="wcta-input"
          placeholder="Your name"
          aria-label="Your name"
          value={name}
          onChange={(ev) => {
            setName(ev.target.value);
            clearErrorOnEdit();
          }}
          disabled={phase === "submitting"}
          autoComplete="name"
          data-testid="walkthrough-name"
        />
        <input
          type="text"
          className="wcta-input"
          placeholder="Your role"
          aria-label="Your role"
          value={role}
          onChange={(ev) => {
            setRole(ev.target.value);
            clearErrorOnEdit();
          }}
          autoComplete="organization-title"
          disabled={phase === "submitting"}
          data-testid="walkthrough-role"
        />
      </div>
      <textarea
        className="wcta-input wcta-textarea"
        placeholder="What would you hand to an AI teammate? (optional)"
        aria-label="What would you hand to an AI teammate?"
        value={handoff}
        onChange={(ev) => setHandoff(ev.target.value)}
        disabled={phase === "submitting"}
        rows={3}
        data-testid="walkthrough-handoff"
      />
      <button
        type="submit"
        className="wcta-submit"
        disabled={phase === "submitting"}
        data-testid="walkthrough-submit"
      >
        {phase === "submitting" ? (
          <>
            <Loader2 size={16} className="wcta-spin" /> Sending…
          </>
        ) : (
          <>
            Book a governance walkthrough <ArrowRight size={16} />
          </>
        )}
      </button>
      {phase === "error" && errorMsg ? (
        <p className="wcta-msg-error" role="alert" data-testid="walkthrough-error">
          <AlertCircle size={14} /> {errorMsg}
        </p>
      ) : (
        <p className="wcta-hint">
          A 30-minute session on the audit chain, truth boundary, and approval
          model — with your security team welcome. Prefer email?{" "}
          <a href="mailto:hello@skycot.com">hello@skycot.com</a>.
        </p>
      )}
      <style jsx>{`
        .wcta-form {
          display: flex;
          flex-direction: column;
          gap: 12px;
          width: 100%;
        }
        .wcta-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 12px;
        }
        @media (max-width: 520px) {
          .wcta-grid {
            grid-template-columns: 1fr;
          }
        }
        .wcta-input {
          width: 100%;
          padding: 12px 14px;
          font-size: 15px;
          font-family: inherit;
          color: var(--foreground);
          background: var(--background);
          border: 1px solid var(--border);
          border-radius: 10px;
          outline: none;
          transition: border-color 0.15s ease;
        }
        .wcta-input:focus {
          border-color: var(--primary);
        }
        .wcta-input:disabled {
          opacity: 0.6;
        }
        .wcta-textarea {
          resize: vertical;
          min-height: 76px;
        }
        .wcta-submit {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          padding: 13px 18px;
          font-size: 15px;
          font-weight: 600;
          color: #fff;
          background: linear-gradient(135deg, var(--primary), var(--gradient-end));
          border: none;
          border-radius: 10px;
          cursor: pointer;
        }
        .wcta-submit:disabled {
          opacity: 0.7;
          cursor: progress;
        }
        .wcta-spin {
          animation: wcta-spin 0.8s linear infinite;
        }
        @keyframes wcta-spin {
          to {
            transform: rotate(360deg);
          }
        }
        .wcta-msg-error {
          display: flex;
          align-items: center;
          gap: 6px;
          margin: 0;
          font-size: 13px;
          color: var(--destructive);
        }
        .wcta-hint {
          margin: 0;
          font-size: 13px;
          line-height: 1.5;
          color: var(--muted-foreground);
        }
        .wcta-hint a {
          color: var(--primary);
        }
      `}</style>
    </form>
  );
}
