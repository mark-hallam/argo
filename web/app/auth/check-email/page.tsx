"use client";

import { Suspense, useState } from "react";
import { useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/Button";
import { useSignup } from "@/lib/auth/use-signup";

export default function CheckEmailPage() {
  return (
    <Suspense fallback={<Scaffold>Loading…</Scaffold>}>
      <CheckEmailInner />
    </Suspense>
  );
}

function CheckEmailInner() {
  const search = useSearchParams();
  const email = search?.get("email") ?? "";
  const { state, submit } = useSignup();
  const [resentAt, setResentAt] = useState<number | null>(null);

  async function onResend() {
    if (!email) return;
    const result = await submit({ email });
    if (result.ok) setResentAt(Date.now());
  }

  const pending = state.kind === "submitting";
  const resendError = state.kind === "error" ? state.message : null;

  return (
    <Scaffold>
      <h1 className="text-lg font-semibold text-[var(--foreground)]">Check your inbox</h1>
      <p className="text-sm text-[var(--muted-foreground)]">
        We sent a magic sign-in link
        {email ? <> to <span className="text-[var(--foreground)] font-medium">{email}</span></> : null}
        . It expires in 15 minutes.
      </p>
      <div className="space-y-2">
        <Button
          type="button"
          variant="secondary"
          size="md"
          className="w-full"
          loading={pending}
          disabled={pending || !email}
          onClick={onResend}
          data-testid="check-email-resend"
        >
          {pending ? "Sending…" : "Resend the link"}
        </Button>
        {resentAt ? (
          <p role="status" className="text-xs text-[var(--muted-foreground)]" data-testid="check-email-resent">
            Sent again just now.
          </p>
        ) : null}
        {resendError ? (
          <p role="alert" className="text-xs text-[var(--destructive)]" data-testid="check-email-error">
            {resendError}
          </p>
        ) : null}
      </div>
    </Scaffold>
  );
}

function Scaffold({ children }: { children: React.ReactNode }) {
  return (
    <main className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div
        className="w-full max-w-sm space-y-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm"
        data-testid="check-email-card"
      >
        {children}
      </div>
    </main>
  );
}
