"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

/**
 * Route-group error boundary for the entire `(app)` group (~51
 * in-product routes). Next.js renders the NEAREST `error.tsx` on an
 * uncaught render error; this one sits INSIDE the `(app)` layout, so a
 * blow-up in any app route is CONTAINED to the page body — the nav
 * shell (sidebar + top bar) stays mounted around it.
 *
 * Before this file existed, the only boundary was `app/error.tsx`,
 * which lives ABOVE the `(app)` layout: any uncaught error there
 * replaced the whole shell with a full-page 500, stranding the
 * customer with no navigation. This is the contained counterpart.
 *
 * CLIENT-only (it receives `error` + `reset` from Next). It renders a
 * static shell + retry button — it never dereferences server data,
 * because hydration may not have completed when it mounts.
 */
export default function AppError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:app] unhandled route error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      layout="contained"
      variant="server-error"
      title="This page hit a snag."
      description="Something went wrong loading this view. Try again, or jump back to your team's home — the rest of your workspace is still here."
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
