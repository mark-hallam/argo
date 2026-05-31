"use client";

import { useCallback, useState } from "react";
import { ArrowRight, CheckCircle2, Loader2, AlertCircle } from "lucide-react";
import { joinWaitlist, isValidEmail } from "@/lib/api/waitlist";
import { ApiError } from "@/lib/api/types";

/**
 * Early-access waitlist email-capture form for the public `/landing` page.
 *
 * ⚠️ DRAFT — BACKEND PENDING. The `POST /api/v1/waitlist` route does not
 * exist yet (see `web/lib/api/waitlist.ts` header). This form is UI-complete
 * and validates the email client-side, but on submit it will hit a 404 until
 * the backend lands. Per NO-THEATER (DEC-007) it does NOT fake a success
 * banner: a real 2xx shows the "you're on the list" state; any failure shows
 * a clearly-labeled error (and, for the expected 404/network case, a
 * "we're wiring this up" note) so a draft surface never pretends a signup
 * was recorded when no backend recorded it.
 *
 * `source` lets the same component distinguish the hero capture from the
 * final-CTA capture in the (future) backend's analytics.
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
  // Distinguishes "the backend isn't wired yet" (expected on this draft)
  // from a genuine validation/server error, so the copy can be honest
  // about which one happened.
  const [backendPending, setBackendPending] = useState(false);

  const submit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      if (phase === "submitting") return;
      const trimmed = email.trim();
      if (!isValidEmail(trimmed)) {
        setPhase("error");
        setBackendPending(false);
        setErrorMsg("Enter a valid email address.");
        return;
      }
      setPhase("submitting");
      setErrorMsg(null);
      setBackendPending(false);
      try {
        await joinWaitlist({ email: trimmed, source });
        setPhase("queued");
      } catch (err) {
        setPhase("error");
        // A 404 (route not built) or a network/transport failure is the
        // EXPECTED state for this draft — surface it honestly rather than
        // claiming success.
        const is404 = err instanceof ApiError && err.status === 404;
        const isNetwork =
          !(err instanceof ApiError) ||
          (err instanceof ApiError && err.status >= 500);
        if (is404 || isNetwork) {
          setBackendPending(true);
          setErrorMsg(
            "Waitlist capture isn't connected yet — this is a draft surface. Your email was NOT saved.",
          );
        } else {
          setErrorMsg(
            err instanceof ApiError && err.message
              ? err.message
              : "Something went wrong. Please try again.",
          );
        }
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
          className={`wl-msg ${backendPending ? "wl-msg-pending" : "wl-msg-error"}`}
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
