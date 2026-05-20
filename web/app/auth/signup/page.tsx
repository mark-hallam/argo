"use client";

import { useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { useSignup } from "@/lib/auth/use-signup";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";
import { SsoButtons } from "@/components/auth/SsoButtons";

/**
 * Signup page — locked editorial-A design (2026-05-18 design lock).
 *
 * Customer-facing entry. Three SSO buttons (Google / GitHub / Microsoft)
 * start the OAuth authorization-code flow; magic-link is the email
 * fallback for anyone without an SSO provider.
 */
export default function SignupPage() {
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
      topRightLabel="Already have an account?"
      topRightHref="/auth/signin"
      topRightCta="Sign in"
    >
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        START YOUR FREE TRIAL
      </div>

      <h1 className="hero">
        Hire your first
        <br />
        <strong>AI teammate</strong>.
      </h1>

      <p className="sub">
        In <span className="emphasis">four minutes</span>, you&apos;ll set up your workspace,
        link a provider, and have a teammate working alongside you.{" "}
        <span className="emphasis">No credit card</span> required.
      </p>

      <SsoButtons />

      <div className="divider">
        <span className="divider-lbl">OR WITH EMAIL</span>
      </div>

      <form onSubmit={onSubmit} noValidate data-testid="signup-form">
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
            aria-describedby={error ? "signup-error" : undefined}
            placeholder="you@yourcompany.com"
            className="field-input"
            data-testid="signup-email"
            autoFocus
          />
        </div>
        {error ? (
          <p id="signup-error" role="alert" className="form-error">
            {error}
          </p>
        ) : null}
        <button
          type="submit"
          className="cta"
          disabled={pending || email.trim().length === 0}
          data-testid="signup-submit"
        >
          {pending ? "Sending magic link…" : "Send magic link"}
          {pending ? null : <span className="kbd">↵</span>}
        </button>
      </form>

      <p className="below-form">
        We&apos;ll email you a one-tap sign-in link. No password to remember.
      </p>
    </AuthEditorialShell>
  );
}
