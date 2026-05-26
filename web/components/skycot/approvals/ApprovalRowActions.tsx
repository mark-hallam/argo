"use client";

import { Check, X } from "lucide-react";
import { type MouseEvent } from "react";
import { Button } from "@/components/ui/Button";
import type { Severity } from "@/components/skycot/approvals/SeverityPill";
import { cn } from "@/lib/utils";

/**
 * Inline Approve / Refuse cluster surfaced on every pending row of
 * the locked E-ops-dashboard at `/approvals`. Before this component
 * the row was a click-to-drill-in `<button>`-only — Scene 6's central
 * beat ("Operator clicks Approve") had no Approve button to click
 * anywhere on the page. See brief `fix-approvals-ops-dashboard-
 * inline-approve-reject-buttons`.
 *
 * Action buttons stop propagation so a click on Approve / Refuse does
 * NOT also fire the row's drill-in handler.
 *
 * Severity tints the Approve button's border on p0 rows; p1 / p2 stay
 * neutral. Matches the locked design (severity-tinted call to action).
 */

interface ApprovalRowActionsProps {
  approvalId: string;
  severity: Severity;
  onApprove: (id: string) => void;
  onRefuse: (id: string) => void;
  disabled?: boolean;
}

function stop(e: MouseEvent<HTMLButtonElement>) {
  e.stopPropagation();
}

export function ApprovalRowActions({
  approvalId,
  severity,
  onApprove,
  onRefuse,
  disabled,
}: ApprovalRowActionsProps) {
  const approveTint =
    severity === "p0"
      ? "border border-[var(--destructive)]/40 shadow-[0_0_0_1px_var(--destructive)]/20"
      : "";
  return (
    <div
      className="flex items-center gap-2"
      data-testid={`approval-row-actions-${approvalId}`}
      onClick={(e) => e.stopPropagation()}
    >
      <Button
        variant="primary"
        size="sm"
        className={cn(approveTint)}
        disabled={disabled}
        data-testid={`approve-${approvalId}`}
        aria-label="Approve"
        onClick={(e) => {
          stop(e);
          onApprove(approvalId);
        }}
      >
        <Check />
        Approve
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={disabled}
        data-testid={`refuse-${approvalId}`}
        aria-label="Refuse"
        onClick={(e) => {
          stop(e);
          onRefuse(approvalId);
        }}
      >
        <X />
        Refuse
      </Button>
    </div>
  );
}
