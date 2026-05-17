"use client";

import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

const STATUS_URL = "https://status.skycot.com";
const SUPPORT_EMAIL = "support@skycot.com";

/**
 * App Router error boundary. CLIENT-only (it receives `error` and
 * `reset` props from Next). Static shell + retry button — never
 * dereferences server data because hydration may not have completed.
 */
export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot] unhandled app error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="Something went wrong on our side."
      description="We've been notified. You can try again in a moment, or check our status page if the problem persists."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Try again
          </button>
          <a
            href={STATUS_URL}
            className={buttonVariants({ variant: "secondary", size: "md" })}
            rel="noreferrer noopener"
          >
            View status
          </a>
          <a
            href={`mailto:${SUPPORT_EMAIL}`}
            className={buttonVariants({ variant: "ghost", size: "md" })}
          >
            Contact support
          </a>
        </>
      }
    />
  );
}
