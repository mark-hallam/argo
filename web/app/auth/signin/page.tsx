"use client";

import Link from "next/link";
import { Suspense, useEffect, useState, type FormEvent } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useSignup } from "@/lib/auth/use-signup";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";
import { SsoButtons } from "@/components/auth/SsoButtons";
import {
  discoverSso,
  startOAuth,
  type OAuthProvider,
  type SsoDiscoveryResult,
} from "@/lib/api/sso";
import { parseReturnTo, storeReturnToPending } from "@/lib/auth/return-to";

/**
 * Signin page — locked editorial-A design (2026-05-18 design lock).
 *
 * Flow:
 *  1. Three SSO buttons (Google / GitHub / Microsoft) start OAuth.
 *  2. Email + Continue triggers an SSO-discovery probe:
 *     - SAML enforced for this email's workspace → "Sign in with SSO"
 *       replaces the magic-link button and routes to the IdP.
 *     - OAuth-only workspace → highlight the matching SSO button and
 *       offer a one-tap "Continue with <Provider>" CTA.
 *     - No SSO → fall through to the standard magic-link flow.
 *
 * The bearer-token developer auth path lives at `/auth/dev` (separate
 * surface). Customers never see "bearer token" terminology.
 */

type SsoBranch =
  | { kind: "idle" }
  | { kind: "probing" }
  | { kind: "match-saml"; startUrl: string }
  | { kind: "match-oauth"; provider: OAuthProvider }
  | { kind: "match-magic" };

export default function SignInPage() {
  return (
    <Suspense fallback={<SignInScaffold />}>
      <SignInInner />
    </Suspense>
  );
}

function SignInScaffold() {
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
    </AuthEditorialShell>
  );
}

function SignInInner() {
  const router = useRouter();
  const search = useSearchParams();
  const { state, submit } = useSignup();
  const [email, setEmail] = useState("");
  const [sso, setSso] = useState<SsoBranch>({ kind: "idle" });
  const [probeError, setProbeError] = useState<string | null>(null);

  // Capture `?returnTo=` from the URL on mount and persist for the
  // magic-link round-trip. The verify page reads + clears the value
  // post-`POST /api/auth/verify`. localStorage (not sessionStorage)
  // because the magic-link typically opens in a fresh tab.
  useEffect(() => {
    const returnTo = parseReturnTo(search);
    if (returnTo) storeReturnToPending(returnTo);
  }, [search]);

  useEffect(() => {
    if (state.kind === "sent") {
      router.replace(`/auth/check-email?email=${encodeURIComponent(state.email)}`);
    }
  }, [state, router]);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = email.trim();
    if (trimmed.length === 0) return;

    setProbeError(null);
    setSso({ kind: "probing" });
    let result: SsoDiscoveryResult;
    try {
      result = await discoverSso(trimmed);
    } catch {
      // Discovery is a soft probe — on failure, fall through to magic-link.
      result = { mode: "magic_link" };
    }

    if (result.mode === "saml" && result.ssoStartUrl) {
      setSso({ kind: "match-saml", startUrl: result.ssoStartUrl });
      if (typeof window !== "undefined") {
        window.location.href = result.ssoStartUrl;
      }
      return;
    }
    if (result.mode === "oauth" && result.provider) {
      setSso({ kind: "match-oauth", provider: result.provider });
      return;
    }
    setSso({ kind: "match-magic" });
    await submit({ email: trimmed });
  }

  const pending = state.kind === "submitting" || sso.kind === "probing";
  const formError =
    state.kind === "error" ? state.message : probeError ?? null;
  const highlightProvider = sso.kind === "match-oauth" ? sso.provider : undefined;

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

      <SsoButtons highlight={highlightProvider} />

      {sso.kind === "match-saml" ? (
        <p className="below-form" data-testid="sso-saml-redirect">
          Your workspace uses SAML single sign-on. Redirecting to your identity provider…
        </p>
      ) : null}

      <div className="divider">
        <span className="divider-lbl">OR WITH EMAIL</span>
      </div>

      <form onSubmit={onSubmit} noValidate data-testid="signin-form">
        <div className="field-block">
          <label htmlFor="signin-email-input" className="field-label">
            Work email
          </label>
          <input
            id="signin-email-input"
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => {
              setEmail(e.target.value);
              if (sso.kind !== "idle") setSso({ kind: "idle" });
            }}
            aria-invalid={state.kind === "error" || undefined}
            aria-describedby={formError ? "signin-error" : undefined}
            placeholder="you@yourcompany.com"
            className="field-input"
            data-testid="signin-email"
            autoFocus
          />
        </div>
        {formError ? (
          <p id="signin-error" role="alert" className="form-error">
            {formError}
          </p>
        ) : null}
        {sso.kind === "match-oauth" ? (
          <button
            type="button"
            className="cta"
            onClick={() => startOAuth(sso.provider)}
            data-testid="signin-sso-cta"
          >
            Continue with {providerLabel(sso.provider)}
            <span className="kbd">↵</span>
          </button>
        ) : (
          <button
            type="submit"
            className="cta"
            disabled={pending || email.trim().length === 0}
            data-testid="signin-submit"
          >
            {pending ? "Checking…" : "Continue"}
            {pending ? null : <span className="kbd">↵</span>}
          </button>
        )}
      </form>

      <p className="below-form">
        We&apos;ll email you a one-tap sign-in link. No password to remember.
      </p>

      <div className="trouble">
        Trouble signing in? <Link href="/help">Get help</Link> or{" "}
        <a href="mailto:support@skycot.com">contact support</a>
      </div>
    </AuthEditorialShell>
  );
}

function providerLabel(p: OAuthProvider): string {
  switch (p) {
    case "google":
      return "Google";
    case "github":
      return "GitHub";
    case "microsoft":
      return "Microsoft";
  }
}
