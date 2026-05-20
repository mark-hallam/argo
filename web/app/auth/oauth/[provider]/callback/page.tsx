"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";
import { completeOAuth, OAUTH_PROVIDERS, type OAuthProvider } from "@/lib/api/sso";

/**
 * OAuth callback handler.
 *
 * The IdP redirects the browser here with `code` + `state` query
 * params. We POST them to the Next API proxy
 * (`/api/auth/oauth/<provider>/callback`), which forwards to the
 * gateway, receives the session cookie, and re-emits it on the SPA
 * origin (same pattern as `/api/auth/verify`).
 *
 * Routing:
 *  - Gateway reports `needs_onboarding=true` (new account) → `/hire`
 *  - Otherwise → `redirect_to` query param (if safe) or `/`
 */

type Phase =
  | { kind: "exchanging" }
  | { kind: "success" }
  | { kind: "error"; message: string };

export default function OAuthCallbackPage() {
  const params = useParams<{ provider: string }>();
  const search = useSearchParams();
  const router = useRouter();
  const [phase, setPhase] = useState<Phase>({ kind: "exchanging" });

  useEffect(() => {
    const provider = params?.provider;
    const code = search?.get("code");
    const state = search?.get("state");
    const idpError = search?.get("error");
    const idpErrorDescription = search?.get("error_description");
    const redirectTo = sanitizeRedirect(search?.get("redirect_to"));

    if (!provider || !isOAuthProvider(provider)) {
      setPhase({ kind: "error", message: "Unknown SSO provider." });
      return;
    }
    if (idpError) {
      setPhase({
        kind: "error",
        message: idpErrorDescription ?? `Sign-in cancelled (${idpError}).`,
      });
      return;
    }
    if (!code || !state) {
      setPhase({
        kind: "error",
        message: "Missing authorization code from the identity provider.",
      });
      return;
    }

    let cancelled = false;
    completeOAuth(provider, code, state)
      .then((result) => {
        if (cancelled) return;
        setPhase({ kind: "success" });
        const destination = result.needs_onboarding ? "/hire" : redirectTo ?? "/";
        router.replace(destination);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const message =
          err instanceof Error ? err.message : "Sign-in failed. Try again.";
        setPhase({ kind: "error", message });
      });
    return () => {
      cancelled = true;
    };
  }, [params, search, router]);

  return (
    <AuthEditorialShell
      topRightLabel="Need help?"
      topRightHref="/support"
      topRightCta="Contact support"
      showTrustFoot={false}
    >
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        FINISHING SIGN-IN
      </div>
      <h1 className="hero hero-small">
        Welcome
        <br />
        <strong>back</strong>.
      </h1>
      {phase.kind === "exchanging" ? (
        <p className="sub" data-testid="oauth-exchanging">
          Hold on — we&apos;re completing your sign-in with{" "}
          {providerLabel(params?.provider as OAuthProvider)}.
        </p>
      ) : null}
      {phase.kind === "success" ? (
        <p className="sub" data-testid="oauth-success">Signed in. Redirecting…</p>
      ) : null}
      {phase.kind === "error" ? (
        <>
          <p className="sub form-error" role="alert" data-testid="oauth-error">
            {phase.message}
          </p>
          <div className="trouble">
            <Link href="/auth/signin">Back to sign in</Link>
          </div>
        </>
      ) : null}
    </AuthEditorialShell>
  );
}

function isOAuthProvider(value: string): value is OAuthProvider {
  return (OAUTH_PROVIDERS as readonly string[]).includes(value);
}

function providerLabel(p: OAuthProvider | undefined): string {
  switch (p) {
    case "google":
      return "Google";
    case "github":
      return "GitHub";
    case "microsoft":
      return "Microsoft";
    default:
      return "your provider";
  }
}

/**
 * Only honor relative paths beginning with `/` so an attacker can't
 * use a crafted callback URL to bounce users to a phishing site after
 * sign-in (open-redirect class).
 */
function sanitizeRedirect(value: string | null | undefined): string | null {
  if (!value) return null;
  if (!value.startsWith("/")) return null;
  if (value.startsWith("//")) return null;
  return value;
}
