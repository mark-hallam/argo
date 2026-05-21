"use client";

import Link from "next/link";
import { Suspense, useEffect } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { useVerify } from "@/lib/auth/use-verify";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";

/**
 * Verify page — locked editorial-A design (2026-05-18 design lock).
 *
 * Handles the magic-link callback. Three states:
 *  - verifying → spinner "Signing you in…"
 *  - verified → spinner "Redirecting…" before route push
 *  - error → branded retry surface
 */
export default function VerifyPage() {
  return (
    <Suspense fallback={<VerifyScaffold><VerifySpinner label="Loading…" /></VerifyScaffold>}>
      <VerifyInner />
    </Suspense>
  );
}

function VerifyScaffold({ children }: { children?: React.ReactNode }) {
  return (
    <AuthEditorialShell
      topRightLabel="Need help?"
      topRightHref="mailto:support@skycot.com"
      topRightCta="Contact support"
      showTrustFoot={false}
    >
      {children}
    </AuthEditorialShell>
  );
}

function VerifyInner() {
  const router = useRouter();
  const search = useSearchParams();
  const token = search?.get("token") ?? null;
  const state = useVerify(token);

  useEffect(() => {
    if (state.kind !== "verified") return;
    const dest = state.needsOnboarding ? "/onboarding/welcome" : "/";
    router.replace(dest);
  }, [state, router]);

  if (state.kind === "verifying" || state.kind === "idle") {
    return (
      <VerifyScaffold>
        <div className="eyebrow">
          <span className="eyebrow-dot" />
          SIGNING YOU IN
        </div>
        <h1 className="hero hero-small">
          Just a<br />
          <strong>moment</strong>.
        </h1>
        <p className="sub">We&apos;re verifying your magic link and unlocking your workspace…</p>
        <VerifySpinner label="Signing you in…" />
      </VerifyScaffold>
    );
  }
  if (state.kind === "verified") {
    return (
      <VerifyScaffold>
        <div className="eyebrow">
          <span className="eyebrow-dot" />
          WELCOME
        </div>
        <h1 className="hero hero-small">
          You&apos;re
          <br />
          <strong>in</strong>.
        </h1>
        <p className="sub">Redirecting you to your team…</p>
        <VerifySpinner label="Redirecting…" />
      </VerifyScaffold>
    );
  }
  return (
    <VerifyScaffold>
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        LINK PROBLEM
      </div>
      <h1 className="hero hero-small">
        That link
        <br />
        <strong>didn&apos;t work</strong>.
      </h1>
      <p className="sub" data-testid="verify-error">
        {state.message}
      </p>
      <Link href="/auth/signup" className="cta" data-testid="verify-retry">
        Get a new link
        <span className="kbd">↵</span>
      </Link>
      <div className="trouble">
        Still having trouble?{" "}
        <a href="mailto:support@skycot.com">Contact support</a>
      </div>
    </VerifyScaffold>
  );
}

function VerifySpinner({ label }: { label: string }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 10,
        marginTop: 24,
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        color: "var(--ink-3)",
        letterSpacing: "0.16em",
        textTransform: "uppercase",
        fontWeight: 700,
      }}
      role="status"
      data-testid="verify-status"
    >
      <Loader2 size={14} className="animate-spin" aria-hidden />
      {label}
    </div>
  );
}
