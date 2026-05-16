"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/Button";
import { useSignup } from "@/lib/auth/use-signup";

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
    <main className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div className="w-full max-w-sm space-y-6 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm">
        <div className="space-y-1">
          <h1 className="text-lg font-semibold text-[var(--foreground)]">
            Create your SKYCOT workspace
          </h1>
          <p className="text-sm text-[var(--muted-foreground)]">
            We&apos;ll email you a sign-in link. No password.
          </p>
        </div>
        <form className="space-y-4" onSubmit={onSubmit} noValidate data-testid="signup-form">
          <label className="block space-y-1.5">
            <span className="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]">
              Work email
            </span>
            <input
              type="email"
              name="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              aria-invalid={state.kind === "error" || undefined}
              aria-describedby={error ? "signup-error" : undefined}
              placeholder="you@company.com"
              className="w-full h-10 px-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
              data-testid="signup-email"
            />
          </label>
          {error ? (
            <p id="signup-error" role="alert" className="text-sm text-[var(--destructive)]">
              {error}
            </p>
          ) : null}
          <Button
            type="submit"
            variant="primary"
            size="md"
            className="w-full"
            loading={pending}
            disabled={pending || email.trim().length === 0}
            data-testid="signup-submit"
          >
            {pending ? "Sending magic link…" : "Send magic link"}
          </Button>
        </form>
        <p className="text-xs text-[var(--muted-foreground)] text-center">
          Already have an account?{" "}
          <Link
            href="/auth/signin"
            className="text-[var(--primary)] underline-offset-4 hover:underline"
          >
            Sign in
          </Link>
        </p>
      </div>
    </main>
  );
}
