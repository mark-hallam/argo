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
 *
 * Demo-mode branch (demo-ops-add-magic-link-bypass-for-recording):
 * when `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, the "Send magic link" button
 * calls `POST /api/v1/demo/mint-principal` against the gateway
 * directly, bypassing the SMTP round-trip, and routes the operator
 * straight to `/onboarding/welcome`. The branch is only meaningful
 * paired with the gateway flag — without `ARGO_DEMO_MODE=1` on the
 * gateway, the call returns 404 and the form falls back to the
 * normal magic-link flow.
 */
const DEMO_MODE =
  typeof process !== "undefined" && process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

export default function SignupPage() {
  const router = useRouter();
  const { state, submit } = useSignup();
  const [email, setEmail] = useState("");
  const [demoPending, setDemoPending] = useState(false);
  const [demoError, setDemoError] = useState<string | null>(null);

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
    if (trimmed.length === 0) return;
    setDemoPending(true);
    setDemoError(null);
    try {
      const res = await fetch(`${gatewayBase()}/api/v1/demo/mint-principal`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ email: trimmed }),
      });
      if (!res.ok) {
        setDemoError(
          `Demo mint failed (HTTP ${res.status}). Confirm the gateway booted with ARGO_DEMO_MODE=1.`,
        );
        return;
      }
      router.replace("/onboarding/welcome");
    } catch {
      setDemoError("Couldn't reach the gateway. Confirm it's running on the configured port.");
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
      DEMO MODE — auth bypass active. ARGO_DEMO_MODE=1 is set; signup mints a
      verified principal without an email round-trip. Never enable this in
      production.
    </div>
  );
}
