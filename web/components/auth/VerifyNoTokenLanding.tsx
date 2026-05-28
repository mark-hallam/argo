"use client";

import Link from "next/link";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";

/**
 * Renders the soft `/auth/verify` landing for visitors who reached the
 * route with no `?token=` query param. Distinct from the "That link
 * didn't work" surface, which is reserved for tokens that are present
 * but fail verification (expired / used / invalid). Sibling brief:
 * `fix-auth-verify-bare-url-no-token-distinct-from-invalid-token`.
 */
export function VerifyNoTokenLanding() {
  return (
    <AuthEditorialShell
      topRightLabel="Need help?"
      topRightHref="mailto:support@skycot.com"
      topRightCta="Contact support"
      showTrustFoot={false}
    >
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        LOOKING FOR YOUR SIGN-IN LINK?
      </div>
      <h1 className="hero hero-small">
        Check your<br />
        <strong>inbox</strong>.
      </h1>
      <p className="sub" data-testid="verify-no-token-body">
        We email a magic link the moment you ask for one. If you didn&apos;t
        get one, request a new link — we&apos;ll send it in seconds.
      </p>
      <Link
        href="/auth/signin?from=verify"
        className="cta"
        data-testid="verify-no-token-primary"
      >
        Request a new link
        <span className="kbd">↵</span>
      </Link>
      <div className="trouble">
        <Link
          href="/auth/signin"
          data-testid="verify-no-token-secondary"
        >
          Back to sign-in
        </Link>
      </div>
    </AuthEditorialShell>
  );
}
