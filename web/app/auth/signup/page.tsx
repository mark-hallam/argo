"use client";

import { useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { signIn } from "next-auth/react";
import { useSignup } from "@/lib/auth/use-signup";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";
import { SsoButtons } from "@/components/auth/SsoButtons";

/**
 * Signup page — locked editorial-A design (2026-05-18 design lock).
 *
 * Customer-facing entry. Three SSO buttons (Google / GitHub / Microsoft)
 * start the OAuth authorization-code flow; magic-link is the email
 * fallback for anyone without an SSO provider.
 *
 * Two demo-mint surfaces (both bypass the SMTP round-trip):
 *
 *  1. Full demo mode — `NEXT_PUBLIC_ARGO_DEMO_MODE=1` replaces the main
 *     submit button with "Enter demo workspace". The whole form is
 *     repurposed for the v1.0 mp4 recording session. Needs the gateway
 *     paired flag (`ARGO_DEMO_MODE=1`); otherwise the demo-mint route
 *     returns 404.
 *
 *  2. "Or try a demo workspace" escape hatch — added so the demo path
 *     is discoverable in local + preview environments without the
 *     operator having to remember the env-var dance before recording.
 *     The button shares the demo-mint handler with (1) but stays out
 *     of the way in production: hidden unless `NEXT_PUBLIC_ENV !==
 *     "production"` OR `NEXT_PUBLIC_ALLOW_DEMO_MINT=1` is set.
 */
const DEMO_MODE =
  typeof process !== "undefined" && process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1";

function shouldShowDemoMintButton(): boolean {
  if (typeof process === "undefined") return false;
  if (process.env.NEXT_PUBLIC_ALLOW_DEMO_MINT === "1") return true;
  // Default: visible in non-production environments so local dev and
  // preview branches surface the demo path automatically. Production
  // builds (`NEXT_PUBLIC_ENV=production`) must opt in via the flag.
  return process.env.NEXT_PUBLIC_ENV !== "production";
}

export default function SignupPage() {
  const router = useRouter();
  const { state, submit } = useSignup();
  const [email, setEmail] = useState("");
  const [demoPending, setDemoPending] = useState(false);
  const [demoError, setDemoError] = useState<string | null>(null);
  const showSecondaryDemoButton = !DEMO_MODE && shouldShowDemoMintButton();

  useEffect(() => {
    if (state.kind === "sent") {
      router.replace(`/auth/check-email?email=${encodeURIComponent(state.email)}`);
    }
  }, [state, router]);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (DEMO_MODE) {
      await submitDemoMint();
      return;
    }
    await submit({ email });
  }

  async function submitDemoMint() {
    const trimmed = email.trim().toLowerCase();
    if (trimmed.length === 0) {
      setDemoError("Enter an email to continue.");
      return;
    }
    setDemoPending(true);
    setDemoError(null);
    try {
      // Call the same-origin Next.js proxy (NOT the gateway
      // directly): the proxy lands the __Host-session + __Host-csrf
      // cookies on the SPA origin so NextAuth surfaces the principal
      // on subsequent calls. Hitting the gateway directly throws the
      // JWT away and re-creates the 401-on-step-2 demo blocker.
      const res = await fetch("/api/auth/demo-mint", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ email: trimmed }),
      });
      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as
          | { error?: { message?: string } }
          | null;
        const message =
          body?.error?.message ??
          "We couldn't start your demo workspace. Try again in a moment.";
        setDemoError(message);
        return;
      }
      const minted = (await res.json().catch(() => null)) as
        | { session_token?: string }
        | null;
      const sessionToken = minted?.session_token;
      if (!sessionToken) {
        setDemoError("Demo session response was malformed. Refresh and try again.");
        return;
      }
      // Hand the demo JWT to NextAuth's Credentials provider so the
      // session callback fires and surfaces `gatewayToken` + `principal`
      // to client-side `readSessionPeek()`. Without this step, the
      // __Host-session cookie is set but `/api/auth/session` returns
      // null, so the API client can't stamp `X-Argo-Principal` on
      // subsequent gateway calls and onboarding routes 401.
      const signInRes = await signIn("gateway-bearer", {
        token: sessionToken,
        redirect: false,
      });
      if (signInRes?.error) {
        setDemoError(
          "Couldn't sign you into the demo workspace. Refresh and try again.",
        );
        return;
      }
      // Use a document-level replace so no NextAuth/session router callback can
      // win the race and send the demo operator to the post-login app shell.
      window.location.replace("/onboarding/welcome");
    } catch {
      setDemoError(
        "We couldn't reach SKYCOT just now. Check your internet connection and try again.",
      );
    } finally {
      setDemoPending(false);
    }
  }

  const pending = state.kind === "submitting" || demoPending;
  const error = demoError ?? (state.kind === "error" ? state.message : null);
  const buttonLabel = DEMO_MODE
    ? pending
      ? "Minting demo session…"
      : "Enter demo workspace"
    : pending
      ? "Sending magic link…"
      : "Send magic link";

  return (
    <AuthEditorialShell
      topRightLabel="Already have an account?"
      topRightHref="/auth/signin"
      topRightCta="Sign in"
    >
      {DEMO_MODE ? <DemoModeBanner /> : null}

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
          <label htmlFor="signup-email-input" className="field-label">
            Work email
          </label>
          <input
            id="signup-email-input"
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            aria-invalid={state.kind === "error" || Boolean(demoError) || undefined}
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
          {buttonLabel}
          {pending ? null : <span className="kbd">↵</span>}
        </button>
      </form>

      <p className="below-form">
        {DEMO_MODE
          ? "Demo mode is active — no email is sent. You'll land in onboarding immediately."
          : "We'll email you a one-tap sign-in link. No password to remember."}
      </p>

      {showSecondaryDemoButton ? (
        <button
          type="button"
          onClick={submitDemoMint}
          disabled={pending || email.trim().length === 0}
          data-testid="signup-demo-mint"
          style={{
            marginTop: 16,
            width: "100%",
            background: "transparent",
            border: "1px dashed rgba(30, 27, 75, 0.35)",
            borderRadius: 10,
            padding: "12px 14px",
            fontSize: 13,
            fontWeight: 600,
            color: "#1e1b4b",
            cursor: pending || email.trim().length === 0 ? "not-allowed" : "pointer",
            opacity: pending || email.trim().length === 0 ? 0.55 : 1,
          }}
        >
          {demoPending
            ? "Minting demo workspace…"
            : "Or try a demo workspace · no email required"}
        </button>
      ) : null}
    </AuthEditorialShell>
  );
}

function DemoModeBanner() {
  return (
    <div
      role="status"
      data-testid="demo-mode-banner"
      style={{
        background: "#fef3c7",
        border: "1px solid #f59e0b",
        color: "#78350f",
        padding: "10px 14px",
        borderRadius: 8,
        marginBottom: 20,
        fontSize: 13,
        fontWeight: 600,
      }}
    >
      Demo mode is active — no email round-trip and any address you enter will
      drop you straight into onboarding. Only use this on your own machine for
      recording or evaluation.
    </div>
  );
}
