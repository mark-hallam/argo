"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function HireError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:hire] hire flow error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="The hire flow couldn't continue."
      description="Something went wrong while drafting this hire. Retrying usually fixes it; your draft isn't lost — head back to your team and pick up where you left off."
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
