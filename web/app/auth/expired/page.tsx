"use client";

import { Suspense } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { AuthEditorialShell } from "@/components/auth/AuthEditorialShell";

/**
 * Expired-access page — locked editorial-A design (matches signin /
 * verify / check-email). Members whose `expires_at` is in the past
 * hit this surface when they try to sign in or click an expired
 * magic link. The auth-error path (gateway / NextAuth) short-
 * circuits to `/auth/expired?on=<ISO>&email=<email>` when the
 * member record's `revoked_at` is set; this page is the friendly
 * dead-end with a clear contact path.
 */
export default function ExpiredPage() {
  return (
    <Suspense
      fallback={
        <ExpiredShell>
          <ExpiredBody on={null} email={null} />
        </ExpiredShell>
      }
    >
      <ExpiredInner />
    </Suspense>
  );
}

function ExpiredInner() {
  const params = useSearchParams();
  const on = params?.get("on") ?? null;
  const email = params?.get("email") ?? null;
  return (
    <ExpiredShell>
      <ExpiredBody on={on} email={email} />
    </ExpiredShell>
  );
}

function ExpiredShell({ children }: { children: React.ReactNode }) {
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

function ExpiredBody({ on, email }: { on: string | null; email: string | null }) {
  const onPretty = on ? formatDate(on) : null;
  return (
    <>
      <div className="eyebrow">
        <span className="eyebrow-dot" />
        ACCESS EXPIRED
      </div>
      <h1 className="hero hero-small">
        Your access
        <br />
        <strong>has expired</strong>.
      </h1>
      <p className="sub" data-testid="expired-body">
        {onPretty
          ? `Your SKYCOT access expired on ${onPretty}.`
          : "Your SKYCOT access has expired."}{" "}
        Contact your workspace admin if you need to renew.
      </p>

      {email ? (
        <p
          style={{
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 12,
            color: "var(--ink-3)",
            letterSpacing: "0.12em",
            textTransform: "uppercase",
            fontWeight: 700,
            marginTop: 18,
          }}
          data-testid="expired-email"
        >
          {email}
        </p>
      ) : null}

      <Link href="/auth/signin" className="cta" data-testid="expired-back-to-signin">
        Back to sign in
        <span className="kbd">↵</span>
      </Link>

      <div className="trouble">
        Need a renewal?{" "}
        <a href="mailto:support@skycot.com">support@skycot.com</a>
      </div>
    </>
  );
}

function formatDate(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  return new Date(t).toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}
