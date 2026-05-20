"use client";

import { Suspense, useState } from "react";
import { useSearchParams } from "next/navigation";
import { useSignup } from "@/lib/auth/use-signup";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";

/**
 * Check-email page — locked editorial-A design (2026-05-18 design lock).
 *
 * Post-signup confirmation. User has been told to check their inbox.
 * Includes a "Resend the link" affordance to handle the most common
 * support ticket (didn't get the email).
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

  async function onResend() {
    if (!email) return;
    const result = await submit({ email });
    if (result.ok) setResentAt(Date.now());
  }

  const pending = state.kind === "submitting";
  const resendError = state.kind === "error" ? state.message : null;

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

      <div className="trouble">
        Didn&apos;t get the email? Check spam, or{" "}
        <a href="mailto:support@skycot.com">contact support</a>.
      </div>
    </CheckEmailScaffold>
  );
}
