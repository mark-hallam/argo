"use client";

import { ShieldQuestion } from "lucide-react";
import { useState } from "react";
import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { RedactionCategory } from "@/lib/redaction/patterns";

export const ELEVATED_VIEW_AUDIT_PATH = "/api/v1/audit/elevated-view-request";

export interface ElevatedViewAuditBody {
  surface: string;
  categories: RedactionCategory[];
  reason?: string;
}

export interface RequestElevatedViewProps {
  /**
   * Categories the page has masked. Surfaced to the audit log so
   * operators can tell *what* the viewer was hidden from.
   */
  categories: RedactionCategory[];
  /**
   * Surface identifier — e.g. `"approvals-inbox"`. Goes to the audit
   * log so cross-surface elevation rates can be tracked.
   */
  surface?: string;
  /**
   * Override the POST hook (tests). Defaults to a small wrapper over
   * the typed gateway client; if `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`,
   * the request is swallowed locally with a console.info instead of
   * hitting a non-existent gateway endpoint.
   */
  onRequest?: (body: ElevatedViewAuditBody) => Promise<void>;
}

async function defaultRequest(body: ElevatedViewAuditBody): Promise<void> {
  if (process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1") {
    if (typeof console !== "undefined") {
      console.info("[skycot/audit] elevated-view-request (bypass)", body);
    }
    return;
  }
  try {
    await apiPost<{ ok: true }>(ELEVATED_VIEW_AUDIT_PATH, body);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      // Gateway endpoint not yet deployed (J2 sibling brief).
      // Swallow so the click feels successful client-side; the
      // sibling-side audit lands when the endpoint ships.
      return;
    }
    throw err;
  }
}

/**
 * Inline link surfaced on a page where ≥1 redaction is currently
 * applied. Tapping it fires an audit-emitting POST to the gateway
 * (`/api/v1/audit/elevated-view-request`); v1 logs the request but
 * does NOT unmask — see brief E1-e Open Question #3. v1.5 may gate
 * the unmask behind an admin-approval step.
 *
 * Renders nothing if `categories` is empty (i.e. nothing was masked
 * on this surface).
 */
export function RequestElevatedView({
  categories,
  surface = "approvals-inbox",
  onRequest = defaultRequest,
}: RequestElevatedViewProps) {
  const [submitting, setSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (categories.length === 0) return null;

  async function handleClick() {
    if (submitting || submitted) return;
    setSubmitting(true);
    setError(null);
    try {
      await onRequest({ surface, categories });
      setSubmitted(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't send request");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      className="flex items-center gap-2 text-xs text-[var(--muted-foreground)]"
      data-testid="request-elevated-view"
    >
      <ShieldQuestion className="size-3.5 shrink-0" aria-hidden />
      {submitted ? (
        <span>Request logged — an admin will review your access.</span>
      ) : (
        <>
          <span>Some fields are masked for your role.</span>
          <button
            type="button"
            onClick={handleClick}
            disabled={submitting}
            className="font-medium text-[var(--primary)] underline-offset-2 hover:underline disabled:opacity-60"
            data-testid="request-elevated-view-link"
          >
            {submitting ? "Sending…" : "Request elevated view"}
          </button>
          {error ? <span className="text-[var(--destructive)]">· {error}</span> : null}
        </>
      )}
    </div>
  );
}
