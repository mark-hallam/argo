"use client";

import { Loader2 } from "lucide-react";
import { signIn } from "next-auth/react";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/Button";

export default function SignInPage() {
  return (
    <Suspense fallback={<SignInScaffold />}>
      <SignInForm />
    </Suspense>
  );
}

function SignInForm() {
  const router = useRouter();
  const search = useSearchParams();
  const from = search?.get("from") ?? "/";
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      const result = await signIn("gateway-bearer", {
        token,
        redirect: false,
      });
      if (!result || result.error) {
        setError("That token wasn't accepted by the gateway. Check it and try again.");
        return;
      }
      router.replace(from);
      router.refresh();
    } catch {
      setError("Couldn't reach the gateway. Confirm it's running and try again.");
    } finally {
      setPending(false);
    }
  }

  return (
    <SignInScaffold>
      <form className="space-y-4" onSubmit={onSubmit}>
        <label className="block space-y-1.5">
          <span className="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]">
            Bearer token
          </span>
          <input
            type="password"
            autoComplete="off"
            required
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="Paste your gateway bearer token"
            className="w-full h-10 px-3 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
            data-testid="signin-token"
          />
        </label>
        {error ? (
          <p role="alert" className="text-sm text-[var(--destructive)]">
            {error}
          </p>
        ) : null}
        <Button
          type="submit"
          variant="primary"
          size="md"
          className="w-full"
          disabled={pending || token.trim().length === 0}
          data-testid="signin-submit"
        >
          {pending ? (
            <>
              <Loader2 className="size-4 animate-spin" /> Signing in…
            </>
          ) : (
            "Sign in"
          )}
        </Button>
      </form>
    </SignInScaffold>
  );
}

function SignInScaffold({ children }: { children?: React.ReactNode }) {
  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--background)] px-4">
      <div className="w-full max-w-sm space-y-6 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-6 shadow-sm">
        <div className="flex items-center gap-2.5">
          <span
            aria-hidden
            className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-semibold text-sm shadow-sm"
          >
            S
          </span>
          <div className="flex flex-col leading-tight">
            <span className="text-sm font-semibold text-[var(--foreground)]">SKYCOT</span>
            <span className="text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
              AI teammates
            </span>
          </div>
        </div>
        <div className="space-y-1">
          <h1 className="text-lg font-semibold text-[var(--foreground)]">Sign in to SKYCOT</h1>
          <p className="text-sm text-[var(--muted-foreground)]">
            Use the bearer token issued by your gateway operator.
          </p>
        </div>
        {children}
      </div>
    </div>
  );
}
