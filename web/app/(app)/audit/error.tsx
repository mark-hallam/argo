"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function AuditError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:audit] audit hero error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      layout="contained"
      variant="server-error"
      title="The audit feed couldn't load."
      description="We hit a problem fetching the audit ledger. Try again, or jump to your team's home while we recover."
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
            href="/"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to home
          </Link>
        </>
      }
    />
  );
}
