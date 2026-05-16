"use client";

import Link from "next/link";
import { Suspense, useEffect } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { useVerify } from "@/lib/auth/use-verify";

export default function VerifyPage() {
  return (
    <Suspense fallback={<Scaffold><Spinner label="Loading…" /></Scaffold>}>
      <VerifyInner />
    </Suspense>
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
      <Scaffold>
        <Spinner label="Signing you in…" />
      </Scaffold>
    );
  }
  if (state.kind === "verified") {
    return (
      <Scaffold>
        <Spinner label="Redirecting…" />
      </Scaffold>
    );
  }
  return (
    <Scaffold>
      <div className="space-y-1">
        <h1 className="text-lg font-semibold text-[var(--foreground)]">
          That link didn&apos;t work
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]" data-testid="verify-error">
          {state.message}
        </p>
      </div>
      <Link
        href="/auth/signup"
        className="inline-flex w-full items-center justify-center h-9 px-4 rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] text-sm font-medium transition-colors duration-[var(--duration-fast)] ease-out hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]"
        data-testid="verify-retry"
      >
        Get a new link
      </Link>
    </Scaffold>
  );
}

function Spinner({ label }: { label: string }) {
  return (
    <div
      className="flex items-center gap-2 text-sm text-[var(--muted-foreground)]"
      role="status"
      data-testid="verify-status"
    >
      <Loader2 className="size-4 animate-spin" aria-hidden />
      {label}
    </div>
  );
}

function Scaffold({ children }: { children: React.ReactNode }) {
  return (
    <main className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div
        className="w-full max-w-sm space-y-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm"
        data-testid="verify-card"
      >
        {children}
      </div>
    </main>
  );
}
