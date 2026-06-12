"use client";

import { Suspense, useState } from "react";
import { useSearchParams } from "next/navigation";
import {
  normalizeDelivery,
  useSignup,
  type DeliveryStatus,
} from "@/lib/auth/use-signup";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";

/**
 * Check-email page — locked editorial-A design (2026-05-18 design lock).
 *
 * Post-signup confirmation. The gateway now threads an HONEST delivery
 * outcome into the signup response
 * (launch-email-delivery-honesty-and-public-base-url), carried here via
 * the `?delivery=` query param:
 *  - `sent`            → "Check your inbox" happy path.
 *  - `delivery_failed` → a provider was configured but the send failed.
 *  - `not_configured`  → no outbound email provider is wired (log-only).
 * The failure states stop a stranger from waiting forever for a link
 * that never went out, and give them a support path. Provider error
 * detail is NEVER surfaced here — only the coarse status.
 *
 * Includes a "Resend the link" affordance to handle the most common
 * support ticket (didn't get the email); a successful resend updates the
 * displayed state with the fresh delivery outcome.
 */
export default function CheckEmailPage() {
  return (
    <Suspense fallback={<CheckEmailScaffold>Loading…</CheckEmailScaffold>}>
      <CheckEmailInner />
    </Suspense>
  );
}

function CheckEmailScaffold({ children }: { children?: React.ReactNode }) {
  return (
    <AuthEditorialShell
      topRightLabel="Wrong email?"
      topRightHref="/auth/signup"
      topRightCta="Try again"
    >
      {children}
    </AuthEditorialShell>
  );
}

function CheckEmailInner() {
  const search = useSearchParams();
  const email = search?.get("email") ?? "";
  const { state, submit } = useSignup();
  const [resentAt, setResentAt] = useState<number | null>(null);
  const [delivery, setDelivery] = useState<DeliveryStatus>(
    normalizeDelivery(search?.get("delivery")),
  );

  async function onResend() {
    if (!email) return;
    const result = await submit({ email });
    if (result.ok) {
      setResentAt(Date.now());
      setDelivery(result.delivery);
    }
  }

  const pending = state.kind === "submitting";
  const resendError = state.kind === "error" ? state.message : null;

  return delivery === "sent" ? (
    <SentState
      email={email}
      pending={pending}
      onResend={onResend}
      resentAt={resentAt}
      resendError={resendError}
    />
  ) : (
    <HonestFailureState
      delivery={delivery}
      email={email}
      pending={pending}
      onResend={onResend}
      resentAt={resentAt}
      resendError={resendError}
    />
  );
}

interface StateProps {
  email: string;
  pending: boolean;
  onResend: () => void;
  resentAt: number | null;
  resendError: string | null;
}

function SentState({ email, pending, onResend, resentAt, resendError }: StateProps) {
  return (
    <CheckEmailScaffold>
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        MAGIC LINK SENT
      </div>

      <h1 className="hero hero-small">
        Check your
        <br />
        <strong>inbox</strong>.
      </h1>

      <p className="sub">
        We&apos;ve emailed a one-tap sign-in link
        {email ? (
          <>
            {" "}to <span className="emphasis">{email}</span>
          </>
        ) : null}
        . It expires in <span className="emphasis">15 minutes</span>.
      </p>

      <ResendControls
        pending={pending}
        email={email}
        onResend={onResend}
        resentAt={resentAt}
        resendError={resendError}
      />

      <div className="trouble">
        Didn&apos;t get the email? Check spam, or{" "}
        <a href="mailto:support@skycot.com">contact support</a>.
      </div>
    </CheckEmailScaffold>
  );
}

function HonestFailureState({
  delivery,
  email,
  pending,
  onResend,
  resentAt,
  resendError,
}: StateProps & { delivery: DeliveryStatus }) {
  const notConfigured = delivery === "not_configured";
  return (
    <CheckEmailScaffold>
      <div className="eyebrow" data-testid="check-email-honest" data-delivery={delivery}>
        <span className="eyebrow-dot" />
        {notConfigured ? "EMAIL NOT SET UP" : "DELIVERY FAILED"}
      </div>

      <h1 className="hero hero-small">
        We couldn&apos;t
        <br />
        <strong>send it</strong>.
      </h1>

      <p className="sub">
        {notConfigured ? (
          <>
            Your account is ready, but email delivery isn&apos;t configured on
            this workspace yet, so we couldn&apos;t send your sign-in link
            {email ? (
              <>
                {" "}to <span className="emphasis">{email}</span>
              </>
            ) : null}
            . Reach out and we&apos;ll get you in.
          </>
        ) : (
          <>
            Something went wrong sending your sign-in link
            {email ? (
              <>
                {" "}to <span className="emphasis">{email}</span>
              </>
            ) : null}
            . You can try again, or contact support and we&apos;ll sort it out.
          </>
        )}
      </p>

      <ResendControls
        pending={pending}
        email={email}
        onResend={onResend}
        resentAt={resentAt}
        resendError={resendError}
      />

      <div className="trouble">
        Still stuck?{" "}
        <a href="mailto:support@skycot.com">contact support</a> and we&apos;ll
        send your link manually.
      </div>
    </CheckEmailScaffold>
  );
}

function ResendControls({ email, pending, onResend, resentAt, resendError }: StateProps) {
  return (
    <>
      <button
        type="button"
        className="cta cta-secondary"
        disabled={pending || !email}
        onClick={onResend}
        data-testid="check-email-resend"
      >
        {pending ? "Sending…" : "Resend the link"}
      </button>

      {resentAt ? (
        <p role="status" className="below-form" data-testid="check-email-resent">
          ✓ Sent again just now.
        </p>
      ) : null}
      {resendError ? (
        <p
          role="alert"
          className="form-error"
          style={{ marginTop: 12, textAlign: "center" }}
          data-testid="check-email-error"
        >
          {resendError}
        </p>
      ) : null}
    </>
  );
}
