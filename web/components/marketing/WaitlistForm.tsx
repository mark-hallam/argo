"use client";

import { useCallback, useState } from "react";
import { ArrowRight, CheckCircle2, Loader2, AlertCircle } from "lucide-react";
import { joinWaitlist, isValidEmail } from "@/lib/api/waitlist";
import { ApiError } from "@/lib/api/types";

/**
 * Early-access waitlist email-capture form for the public `/landing` page.
 *
 * Posts to the live `POST /api/v1/waitlist` route (see
 * `web/lib/api/waitlist.ts` header). The email is validated client-side, then
 * a real `2xx` shows the "you're on the list" state. Per NO-THEATER (DEC-007)
 * the form never fakes a success banner: any non-2xx is a genuine
 * routing/deploy fault and surfaces as a clearly-labeled, honest error — it is
 * no longer treated as an expected "backend not built" state.
 *
 * `source` lets the same component distinguish the hero capture from the
 * final-CTA capture in the backend's analytics.
 */

type Phase = "idle" | "submitting" | "queued" | "error";

export interface WaitlistFormProps {
  /** Origin tag forwarded to the backend, e.g. "landing-hero". */
  source?: string;
  /** Visual size. "lg" is the hero capture; "md" the inline final-CTA one. */
  size?: "md" | "lg";
}

export function WaitlistForm({ source = "landing", size = "lg" }: WaitlistFormProps) {
  const [email, setEmail] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const submit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (phase === "submitting") return;
      const trimmed = email.trim();
      if (!isValidEmail(trimmed)) {
        setPhase("error");
        setErrorMsg("Enter a valid email address.");
        return;
      }
      setPhase("submitting");
      setErrorMsg(null);
      try {
        await joinWaitlist({ email: trimmed, source });
        setPhase("queued");
      } catch (err) {
        // The route is live, so any failure is a genuine routing/server
        // fault — surface it honestly, never a "backend not built" excuse.
        setPhase("error");
        setErrorMsg(
          err instanceof ApiError && err.message
            ? err.message
            : "Something went wrong. Please try again.",
        );
      }
    },
    [email, phase, source],
  );

  if (phase === "queued") {
    return (
      <div className={`wl-card wl-${size} wl-done`} data-testid="waitlist-success">
        <CheckCircle2 size={22} className="wl-done-icon" />
        <div>
          <p className="wl-done-title">You&apos;re on the list.</p>
          <p className="wl-done-sub">
            We&apos;ll email <strong>{email.trim()}</strong> when your early-access slot opens.
          </p>
        </div>
      </div>
    );
  }

  return (
    <form
      className={`wl-card wl-${size}`}
      onSubmit={submit}
      noValidate
      data-testid="waitlist-form"
      data-source={source}
    >
      <div className="wl-row">
        <input
          type="email"
          className="wl-input"
          placeholder="you@company.com"
          aria-label="Work email for early access"
          value={email}
          onChange={(ev) => {
            setEmail(ev.target.value);
            if (phase === "error") setPhase("idle");
          }}
          disabled={phase === "submitting"}
          autoComplete="email"
          data-testid="waitlist-email"
        />
        <button
          type="submit"
          className="wl-submit"
          disabled={phase === "submitting"}
          data-testid="waitlist-submit"
        >
          {phase === "submitting" ? (
            <>
              <Loader2 size={16} className="wl-spin" /> Joining…
            </>
          ) : (
            <>
              Request early access <ArrowRight size={16} />
            </>
          )}
        </button>
      </div>
      {phase === "error" && errorMsg ? (
        <p
          className="wl-msg wl-msg-error"
          role="alert"
          data-testid="waitlist-error"
        >
          <AlertCircle size={14} /> {errorMsg}
        </p>
      ) : (
        <p className="wl-hint">No credit card. No spam. Unsubscribe anytime.</p>
      )}
    </form>
  );
}
