"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function TeamError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:team] team route error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      layout="contained"
      variant="server-error"
      title="The team roster couldn't load."
      description="We're having trouble loading your team. Try again in a moment, or hire a teammate while we recover."
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
            href="/hire"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Hire teammate
          </Link>
        </>
      }
    />
  );
}
