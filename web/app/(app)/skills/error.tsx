"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function SkillsError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:skills] skills marketplace error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="The skill marketplace couldn't load."
      description="Something went wrong fetching skills. Try again, or head back to your team while we recover."
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
            href="/team"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to team
          </Link>
        </>
      }
    />
  );
}
