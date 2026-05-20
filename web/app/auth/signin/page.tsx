"use client";

import Link from "next/link";
import { useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { useSignup } from "@/lib/auth/use-signup";
import { AuthEditorialShell, AuthSsoButtons } from "@/components/auth/AuthEditorialShell";

/**
 * Signin page — locked editorial-A design (2026-05-18 design lock).
 *
 * Returning customers re-request a magic link by email. Same backend
 * flow as signup (gateway is idempotent on (email, tenant) — new
 * workspaces are created on first send; existing workspaces match).
 * Distinction is in copy + the "WELCOME BACK" eyebrow.
 *
 * The bearer-token developer auth path lives at `/auth/dev` (separate
 * surface). Customers never see "bearer token" terminology.
 */
export default function SignInPage() {
  const router = useRouter();
  const { state, submit } = useSignup();
  const [email, setEmail] = useState("");

  useEffect(() => {
    if (state.kind === "sent") {
      router.replace(`/auth/check-email?email=${encodeURIComponent(state.email)}`);
    }
  }, [state, router]);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await submit({ email });
  }

  const pending = state.kind === "submitting";
  const error = state.kind === "error" ? state.message : null;

  return (
    <AuthEditorialShell
      topRightLabel="New to SKYCOT?"
      topRightHref="/auth/signup"
      topRightCta="Create an account"
    >
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        WELCOME BACK
      </div>

      <h1 className="hero">
        Sign in to
        <br />
        <strong>SKYCOT</strong>.
      </h1>

      <p className="sub">Pick how you want to sign in. We&apos;ll get you back to your team in a moment.</p>

      <AuthSsoButtons />

      <div className="divider">
        <span className="divider-lbl">OR WITH EMAIL</span>
      </div>

      <form onSubmit={onSubmit} noValidate data-testid="signin-form">
        <div className="field-block">
          <div className="field-label">Work email</div>
          <input
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            aria-invalid={state.kind === "error" || undefined}
            aria-describedby={error ? "signin-error" : undefined}
            placeholder="you@yourcompany.com"
            className="field-input"
            data-testid="signin-email"
            autoFocus
          />
        </div>
        {error ? (
          <p id="signin-error" role="alert" className="form-error">
            {error}
          </p>
        ) : null}
        <button
          type="submit"
          className="cta"
          disabled={pending || email.trim().length === 0}
          data-testid="signin-submit"
        >
          {pending ? "Sending sign-in link…" : "Send sign-in link"}
          {pending ? null : <span className="kbd">↵</span>}
        </button>
      </form>

      <p className="below-form">
        We&apos;ll email you a one-tap sign-in link. No password to remember.
      </p>

      <div className="trouble">
        Trouble signing in? <Link href="/support">Get help</Link> or{" "}
        <a href="mailto:support@skycot.com">contact support</a>
      </div>
    </AuthEditorialShell>
  );
}
