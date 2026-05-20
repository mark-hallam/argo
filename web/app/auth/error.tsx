"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function AuthError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:auth] auth flow error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="Sign-in hit a problem."
      description="Something went wrong with the magic-link flow. Try requesting a fresh link, or head back to the landing page."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Try again
          </button>
          <Link
            href="/auth/signin"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to sign in
          </Link>
        </>
      }
    />
  );
}
