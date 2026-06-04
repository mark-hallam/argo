"use client";

import { useCallback, useState } from "react";
import { ArrowRight, CheckCircle2, Loader2, AlertCircle } from "lucide-react";
import {
  submitEnterpriseInquiry,
  isValidEmail,
} from "@/lib/api/enterprise-inquiry";
import { ApiError } from "@/lib/api/types";

/**
 * Enterprise Expression-of-Interest capture form.
 *
 * Replaces the old `mailto:` "Talk to sales" CTA on `/pricing` + `/contact`
 * with a real structured inquiry that POSTs `{ company, name, email, seats?,
 * message? }` to `POST /api/v1/enterprise-inquiry` (WS6-2). The backend
 * persists the lead + writes a hash-chained audit row.
 *
 * Per NO-THEATER (DEC-007) it does NOT fake success: a real 2xx shows the
 * "we've got it" state; any failure shows a clearly-labeled error so the
 * surface never pretends a lead was recorded when no backend recorded it.
 *
 * Self-contained inline styling (CSS-variable driven) so it renders correctly
 * on any marketing surface without depending on page-specific CSS classes.
 */

type Phase = "idle" | "submitting" | "queued" | "error";

export interface EnterpriseInquiryFormProps {
  /** Origin tag (currently informational; not yet forwarded to the backend). */
  source?: string;
}

export function EnterpriseInquiryForm({
  source = "pricing",
}: EnterpriseInquiryFormProps) {
  const [company, setCompany] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [seats, setSeats] = useState("");
  const [message, setMessage] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const submit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (phase === "submitting") return;
      const c = company.trim();
      const n = name.trim();
      const em = email.trim();
      if (!c || !n || !isValidEmail(em)) {
        setPhase("error");
        setErrorMsg(
          !c
            ? "Enter your company name."
            : !n
              ? "Enter your name."
              : "Enter a valid work email address.",
        );
        return;
      }
      setPhase("submitting");
      setErrorMsg(null);
      try {
        await submitEnterpriseInquiry({
          company: c,
          name: n,
          email: em,
          seats: seats.trim() || undefined,
          message: message.trim() || undefined,
        });
        setPhase("queued");
      } catch (err) {
        setPhase("error");
        setErrorMsg(
          err instanceof ApiError && err.message
            ? err.message
            : "Couldn’t send your inquiry. Please try again or email hello@skycot.com.",
        );
      }
    },
    [company, name, email, seats, message, phase],
  );

  if (phase === "queued") {
    return (
      <div className="eoi-done" data-testid="enterprise-inquiry-success">
        <CheckCircle2 size={22} className="eoi-done-icon" />
        <div>
          <p className="eoi-done-title">Thanks — we’ve got it.</p>
          <p className="eoi-done-sub">
            Our team will reach out to <strong>{email.trim()}</strong> about
            {" "}
            <strong>{company.trim()}</strong> within one business day.
          </p>
        </div>
        <style jsx>{`
          .eoi-done {
            display: flex;
            gap: 12px;
            align-items: flex-start;
            padding: 18px 20px;
            border: 1px solid color-mix(in srgb, var(--primary) 35%, transparent);
            border-radius: 14px;
            background: color-mix(in srgb, var(--primary) 6%, var(--background));
          }
          .eoi-done-icon {
            color: var(--primary);
            flex-shrink: 0;
            margin-top: 2px;
          }
          .eoi-done-title {
            margin: 0 0 4px;
            font-weight: 700;
            font-size: 16px;
            color: var(--foreground);
          }
          .eoi-done-sub {
            margin: 0;
            font-size: 14px;
            line-height: 1.5;
            color: var(--muted-foreground);
          }
        `}</style>
      </div>
    );
  }

  return (
    <form
      className="eoi-form"
      onSubmit={submit}
      noValidate
      data-testid="enterprise-inquiry-form"
      data-source={source}
    >
      <div className="eoi-grid">
        <input
          type="text"
          className="eoi-input"
          placeholder="Company"
          aria-label="Company name"
          value={company}
          onChange={(ev) => {
            setCompany(ev.target.value);
            if (phase === "error") setPhase("idle");
          }}
          disabled={phase === "submitting"}
          autoComplete="organization"
          data-testid="enterprise-inquiry-company"
        />
        <input
          type="text"
          className="eoi-input"
          placeholder="Your name"
          aria-label="Your name"
          value={name}
          onChange={(ev) => {
            setName(ev.target.value);
            if (phase === "error") setPhase("idle");
          }}
          disabled={phase === "submitting"}
          autoComplete="name"
          data-testid="enterprise-inquiry-name"
        />
        <input
          type="email"
          className="eoi-input"
          placeholder="Work email"
          aria-label="Work email"
          value={email}
          onChange={(ev) => {
            setEmail(ev.target.value);
            if (phase === "error") setPhase("idle");
          }}
          disabled={phase === "submitting"}
          autoComplete="email"
          data-testid="enterprise-inquiry-email"
        />
        <input
          type="text"
          className="eoi-input"
          placeholder="Seats (optional)"
          aria-label="Estimated seats"
          value={seats}
          onChange={(ev) => setSeats(ev.target.value)}
          disabled={phase === "submitting"}
          data-testid="enterprise-inquiry-seats"
        />
      </div>
      <textarea
        className="eoi-input eoi-textarea"
        placeholder="What are you looking to do? (optional)"
        aria-label="Message"
        value={message}
        onChange={(ev) => setMessage(ev.target.value)}
        disabled={phase === "submitting"}
        rows={3}
        data-testid="enterprise-inquiry-message"
      />
      <button
        type="submit"
        className="eoi-submit"
        disabled={phase === "submitting"}
        data-testid="enterprise-inquiry-submit"
      >
        {phase === "submitting" ? (
          <>
            <Loader2 size={16} className="eoi-spin" /> Sending…
          </>
        ) : (
          <>
            Talk to sales <ArrowRight size={16} />
          </>
        )}
      </button>
      {phase === "error" && errorMsg ? (
        <p className="eoi-msg-error" role="alert" data-testid="enterprise-inquiry-error">
          <AlertCircle size={14} /> {errorMsg}
        </p>
      ) : (
        <p className="eoi-hint">
          We reply within one business day. Prefer email?{" "}
          <a href="mailto:hello@skycot.com">hello@skycot.com</a>.
        </p>
      )}
      <style jsx>{`
        .eoi-form {
          display: flex;
          flex-direction: column;
          gap: 12px;
          width: 100%;
        }
        .eoi-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 12px;
        }
        @media (max-width: 520px) {
          .eoi-grid {
            grid-template-columns: 1fr;
          }
        }
        .eoi-input {
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
        .eoi-input:focus {
          border-color: var(--primary);
        }
        .eoi-input:disabled {
          opacity: 0.6;
        }
        .eoi-textarea {
          resize: vertical;
          min-height: 76px;
        }
        .eoi-submit {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          padding: 12px 18px;
          font-size: 15px;
          font-weight: 600;
          color: #fff;
          background: linear-gradient(135deg, var(--primary), var(--gradient-end));
          border: none;
          border-radius: 10px;
          cursor: pointer;
        }
        .eoi-submit:disabled {
          opacity: 0.7;
          cursor: progress;
        }
        .eoi-spin {
          animation: eoi-spin 0.8s linear infinite;
        }
        @keyframes eoi-spin {
          to {
            transform: rotate(360deg);
          }
        }
        .eoi-msg-error {
          display: flex;
          align-items: center;
          gap: 6px;
          margin: 0;
          font-size: 13px;
          color: var(--destructive);
        }
        .eoi-hint {
          margin: 0;
          font-size: 13px;
          color: var(--muted-foreground);
        }
        .eoi-hint a {
          color: var(--primary);
        }
      `}</style>
    </form>
  );
}
