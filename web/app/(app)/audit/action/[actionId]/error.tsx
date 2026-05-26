"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

/**
 * Route-segment error boundary for `/audit/action/[actionId]`. The
 * happy-path drill-in handles its own loading + error states inside
 * the page component (mock fallback in dev, in-shell empty state on
 * unknown-id, retry button on transient errors). This boundary only
 * catches errors that escape that handling — e.g. a render-time
 * exception in one of the detail cards — and renders a CONTAINED
 * shell that keeps the AppShell chrome around it rather than the
 * brand-loud full-canvas 500 page. The audit feed remains one click
 * away via the "Back to audit" link.
 */
export default function AuditActionError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:audit/action/[actionId]] action detail error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="Couldn't load this row"
      variant="server-error"
      layout="contained"
      title="The action detail couldn't load."
      description="The audit feed remains the source of truth — drill into another row, or retry."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            data-testid="audit-detail-error-retry"
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Try again
          </button>
          <Link
            href="/audit"
            data-testid="audit-detail-error-back"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to audit
          </Link>
        </>
      }
    />
  );
}
