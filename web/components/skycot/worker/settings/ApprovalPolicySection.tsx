"use client";

import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { ApprovalPolicyEditModal } from "./ApprovalPolicyEditModal";
import {
  APPROVAL_POLICY_LABELS,
  type ApprovalPolicy,
  type InferredTrustGrade,
  type WorkerPolicyRow,
} from "@/lib/api/worker-policies";
import { cn } from "@/lib/utils";

/**
 * Approval-Policy section — per-action-type table that surfaces
 * the worker's current approval policy and lets the operator
 * change it (with a required operator-note). Goes read-only when
 * the worker is paused.
 *
 * The runtime doesn't yet consume per-action policies; the UI lets
 * operators author them now so they're ready when the evaluator
 * grows the support (and so the audit trail has a record).
 */

export interface ApprovalPolicySectionProps {
  workerName: string;
  rows: WorkerPolicyRow[];
  readOnly: boolean;
  onSubmit: (vars: {
    actionType: string;
    policy: ApprovalPolicy;
    operatorNote: string;
  }) => void;
}

export function ApprovalPolicySection({
  workerName,
  rows,
  readOnly,
  onSubmit,
}: ApprovalPolicySectionProps) {
  const [editing, setEditing] = useState<WorkerPolicyRow | null>(null);

  return (
    <Card data-testid="card-settings-approval-policy">
      <CardHeader>
        <CardTitle>Approval policy</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          Per-action approval rules for {workerName}. Click a row to change a
          policy.{" "}
          {readOnly ? (
            <span
              data-testid="approval-policy-readonly-banner"
              className="text-[var(--muted-foreground)]"
            >
              {workerName} is paused — policy changes are disabled until she
              resumes.
            </span>
          ) : null}
        </p>
      </CardHeader>
      <CardBody className="pt-1">
        {rows.length === 0 ? (
          <p
            data-testid="approval-policy-empty"
            className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] px-4 py-6 text-center text-sm text-[var(--muted-foreground)]"
          >
            No action types declared in this worker's vertical yet.
          </p>
        ) : (
          <table
            className="w-full text-sm"
            data-testid="approval-policy-table"
          >
            <thead className="text-left text-xs uppercase tracking-wide text-[var(--muted-foreground)]">
              <tr>
                <th className="py-2 font-medium">Action</th>
                <th className="py-2 font-medium">Current policy</th>
                <th className="py-2 font-medium">Inferred trust grade</th>
                <th />
              </tr>
            </thead>
            <tbody className="divide-y divide-[var(--border)]/60">
              {rows.map((row) => (
                <tr
                  key={row.actionType}
                  data-testid={`approval-policy-row-${row.actionType}`}
                  className="align-middle"
                >
                  <td className="py-3 font-mono text-[13px] text-[var(--foreground)]">
                    {row.actionType}
                  </td>
                  <td className="py-3">
                    <PolicyBadge
                      policy={row.currentPolicy}
                      isDefault={row.isDefault}
                    />
                  </td>
                  <td className="py-3">
                    <TrustGradePill grade={row.inferredTrustGrade} />
                  </td>
                  <td className="py-3 text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={readOnly}
                      onClick={() => setEditing(row)}
                      data-testid={`approval-policy-edit-${row.actionType}`}
                    >
                      Edit
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </CardBody>
      <ApprovalPolicyEditModal
        open={editing !== null}
        row={editing}
        onOpenChange={(open) => {
          if (!open) setEditing(null);
        }}
        onSubmit={(vars) => {
          onSubmit(vars);
          setEditing(null);
        }}
      />
    </Card>
  );
}

function PolicyBadge({
  policy,
  isDefault,
}: {
  policy: ApprovalPolicy;
  isDefault: boolean;
}) {
  return (
    <span
      data-testid={`approval-policy-badge-${policy}`}
      data-default={isDefault || undefined}
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs",
        policy === "auto-approve" &&
          "border-[var(--success)]/40 bg-[var(--success)]/10 text-[var(--success)]",
        policy === "always-ask" &&
          "border-[var(--primary)]/40 bg-[var(--primary)]/10 text-[var(--primary)]",
        policy === "always-refuse" &&
          "border-[var(--destructive)]/40 bg-[var(--destructive)]/10 text-[var(--destructive)]",
        policy === "risk-gated" &&
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
      )}
    >
      <ShieldCheck className="size-3" aria-hidden />
      {APPROVAL_POLICY_LABELS[policy]}
      {isDefault ? (
        <span className="text-[var(--muted-foreground)]">·default</span>
      ) : null}
    </span>
  );
}

function TrustGradePill({ grade }: { grade: InferredTrustGrade }) {
  if (grade === "unknown") {
    return (
      <span className="text-xs text-[var(--muted-foreground)]">unknown</span>
    );
  }
  return (
    <span
      data-testid={`approval-policy-trust-${grade}`}
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs",
        grade === "low" && "bg-[var(--success)]/10 text-[var(--success)]",
        grade === "medium" && "bg-[var(--primary)]/10 text-[var(--primary)]",
        grade === "high" &&
          "bg-[var(--destructive)]/10 text-[var(--destructive)]",
      )}
    >
      {grade}
    </span>
  );
}
