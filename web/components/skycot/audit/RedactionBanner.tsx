"use client";

import { useState } from "react";
import { ShieldQuestion, X } from "lucide-react";
import { ElevatedViewRequestDialog } from "./ElevatedViewRequestDialog";
import type { RbacPermission, ViewerPolicy } from "@/lib/redaction/types";

/**
 * Top-of-page banner shown whenever the viewer's policy is masking
 * something present on the surface. Subtle warning-tone background.
 * `×` dismisses for the session (state, not persisted).
 *
 * Placement is in the page body (not the header) so the audit page's
 * sticky header keeps its scroll behavior.
 */

export interface RedactionBannerProps {
  /** Currently-effective viewer policy. Banner renders only when at
   *  least one of these is true and `present` lists ≥1 sentinel. */
  policy: ViewerPolicy;
  /** Which categories actually have masked spans on the page. Drives
   *  the dialog's pre-selected checkboxes. Empty → banner hidden. */
  presentCategories: RbacPermission[];
  /** Surface identifier forwarded to the audit-the-audit log. */
  surface: string;
  /** Optional test seam: skip dismiss state (always-rendered). */
  forceOpen?: boolean;
}

export function RedactionBanner({
  policy,
  presentCategories,
  surface,
  forceOpen = false,
}: RedactionBannerProps) {
  const [dismissed, setDismissed] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);

  const masked = presentCategories.filter((perm) => !policy.permissions[permKey(perm)]);
  if (!forceOpen && (dismissed || masked.length === 0)) return null;

  return (
    <>
      <div
        role="status"
        data-testid="audit-redaction-banner"
        data-masked-count={masked.length}
        className="flex flex-wrap items-center justify-between gap-3 rounded-[var(--radius-md)] border border-[var(--warning)]/30 bg-[var(--warning)]/10 px-3 py-2 text-xs text-[var(--foreground)]"
      >
        <div className="flex items-center gap-2">
          <ShieldQuestion aria-hidden className="size-4 text-[var(--warning)]" />
          <span>
            <strong className="font-medium">PII redacted</strong> — your role shows masked data.
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-testid="audit-redaction-banner-request"
            onClick={() => setDialogOpen(true)}
            className="font-medium text-[var(--primary)] underline-offset-4 hover:underline"
          >
            Request elevated view →
          </button>
          <button
            type="button"
            data-testid="audit-redaction-banner-dismiss"
            aria-label="Dismiss banner for this session"
            onClick={() => setDismissed(true)}
            className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)]"
          >
            <X aria-hidden className="size-3.5" />
          </button>
        </div>
      </div>
      <ElevatedViewRequestDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        initialCategories={masked}
        surface={surface}
      />
    </>
  );
}

function permKey(perm: RbacPermission): keyof ViewerPolicy["permissions"] {
  switch (perm) {
    case "unredacted-email":
      return "unredactedEmail";
    case "unredacted-phone":
      return "unredactedPhone";
    case "unredacted-card":
      return "unredactedCard";
    case "unredacted-iban":
      return "unredactedIBAN";
    case "unredacted-ird":
      return "unredactedIRD";
    case "unredacted-classification":
      return "unredactedClassification";
    case "unredacted-approver":
      return "unredactedApprover";
    case "unredacted-counterparty":
      return "unredactedCounterparty";
  }
}
