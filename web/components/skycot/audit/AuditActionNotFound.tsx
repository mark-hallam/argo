"use client";

import Link from "next/link";
import { FileQuestion } from "lucide-react";

/**
 * Inside-AppShell 404 surface for an unknown action id. Routed to by
 * the drill-in page when `fetchAuditDetail` throws
 * `AuditDetailNotFoundError`, or when the gateway returns 404 in
 * strict-production mode.
 */

export interface AuditActionNotFoundProps {
  actionId: string;
}

export function AuditActionNotFound({ actionId }: AuditActionNotFoundProps) {
  return (
    <div
      data-testid="audit-action-not-found"
      className="mx-auto max-w-2xl px-4 sm:px-6 lg:px-8 py-16 text-center space-y-3"
    >
      <FileQuestion aria-hidden className="size-10 mx-auto text-[var(--muted-foreground)]" />
      <h1 className="text-lg font-semibold text-[var(--foreground)]">
        Action not found
      </h1>
      <p className="text-sm text-[var(--muted-foreground)]">
        No audit record matches{" "}
        <code className="font-mono text-[12px] text-[var(--foreground)]">{actionId}</code>.
      </p>
      <p className="text-xs text-[var(--muted-foreground)]">
        The action may have been redacted, hard-deleted by a retention policy, or
        the id is incorrect.
      </p>
      <div>
        <Link
          href="/audit"
          data-testid="audit-action-not-found-back"
          className="inline-flex items-center text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
        >
          ← Back to audit timeline
        </Link>
      </div>
    </div>
  );
}
