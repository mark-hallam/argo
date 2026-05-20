"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function WorkerDetailError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:workers/[workerId]] worker detail error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="We couldn't load this worker."
      description="Something went wrong fetching this worker's profile. Retrying usually fixes it; otherwise the cohort list is always available."
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
