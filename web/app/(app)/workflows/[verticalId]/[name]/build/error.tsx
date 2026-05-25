"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

/**
 * Error boundary for the workflow builder. The React Flow canvas can
 * throw on malformed seed data, missing node types, or browser
 * compatibility issues. We catch it here so the demo doesn't
 * white-screen mid-build — instead the user sees a polished retry +
 * "back to viewer" path.
 */
export default function BuilderError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot] builder canvas error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      layout="contained"
      variant="server-error"
      title="The builder couldn't open this workflow."
      description="The visual canvas ran into a problem rendering this workflow. Reloading the canvas usually fixes it; if not, the read-only timeline view always works."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Reload the canvas
          </button>
          <Link
            href="/workflows"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to all workflows
          </Link>
        </>
      }
    />
  );
}
