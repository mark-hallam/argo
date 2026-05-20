"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function AuditReplayError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:audit/action/[actionId]/replay] replay stream error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="The replay stream couldn't render."
      description="The replay viewer ran into bad data while rebuilding this action's timeline. Try reloading the replay; if it persists, the action's summary view still works."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Reload replay
          </button>
          <Link
            href="/audit"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to audit
          </Link>
        </>
      }
    />
  );
}
