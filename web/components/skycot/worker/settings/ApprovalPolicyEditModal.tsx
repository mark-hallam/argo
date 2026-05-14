"use client";

import { useEffect, useState } from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Button } from "@/components/ui/Button";
import {
  APPROVAL_POLICIES,
  APPROVAL_POLICY_HELP,
  APPROVAL_POLICY_LABELS,
  type ApprovalPolicy,
  type WorkerPolicyRow,
} from "@/lib/api/worker-policies";
import { cn } from "@/lib/utils";

/**
 * Approval-policy edit modal. Operator picks one of the four
 * canonical policies and writes a required note explaining why
 * (the note shows up in audit drill-in).
 *
 * Save is gated on: policy != current, and note is non-empty
 * after trimming. Cancel just closes — no auto-save on dismiss.
 */

export interface ApprovalPolicyEditModalProps {
  open: boolean;
  row: WorkerPolicyRow | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (vars: {
    actionType: string;
    policy: ApprovalPolicy;
    operatorNote: string;
  }) => void;
}

export function ApprovalPolicyEditModal({
  open,
  row,
  onOpenChange,
  onSubmit,
}: ApprovalPolicyEditModalProps) {
  const [policy, setPolicy] = useState<ApprovalPolicy>(
    row?.currentPolicy ?? "risk-gated",
  );
  const [note, setNote] = useState("");

  useEffect(() => {
    if (open && row) {
      setPolicy(row.currentPolicy);
      setNote("");
    }
  }, [open, row]);

  const trimmedNote = note.trim();
  const canSave =
    row !== null && trimmedNote.length > 0 && policy !== row.currentPolicy;

  const handleSave = () => {
    if (!row || !canSave) return;
    onSubmit({
      actionType: row.actionType,
      policy,
      operatorNote: trimmedNote,
    });
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm"
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="approval-policy-edit-modal"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            Change policy for{" "}
            <span className="font-mono text-sm">{row?.actionType ?? ""}</span>
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="mt-2 text-sm text-[var(--muted-foreground)]">
            Pick a new policy and explain why — the note shows up in
            the audit trail.
          </DialogPrimitive.Description>

          <ul
            role="radiogroup"
            aria-label="Approval policy"
            className="mt-4 space-y-2"
          >
            {APPROVAL_POLICIES.map((opt) => (
              <li key={opt}>
                <label
                  data-testid={`approval-policy-option-${opt}`}
                  className={cn(
                    "flex cursor-pointer items-start gap-3 rounded-[var(--radius-md)] border px-3 py-2",
                    policy === opt
                      ? "border-[var(--primary)]/60 bg-[var(--accent)]/20"
                      : "border-[var(--border)]",
                  )}
                >
                  <input
                    type="radio"
                    name="approval-policy"
                    checked={policy === opt}
                    onChange={() => setPolicy(opt)}
                    data-testid={`approval-policy-radio-${opt}`}
                    className="mt-1 size-3.5 accent-[var(--primary)]"
                  />
                  <span className="flex-1">
                    <span className="block text-sm font-medium text-[var(--foreground)]">
                      {APPROVAL_POLICY_LABELS[opt]}
                    </span>
                    <span className="block text-xs text-[var(--muted-foreground)]">
                      {APPROVAL_POLICY_HELP[opt]}
                    </span>
                  </span>
                </label>
              </li>
            ))}
          </ul>

          <label className="mt-4 block text-sm font-medium text-[var(--foreground)]">
            Operator note (required)
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              data-testid="approval-policy-note"
              placeholder="Why are you changing this?"
              rows={3}
              maxLength={1024}
              className="mt-1 block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)] focus:border-[var(--primary)] focus:outline-none"
            />
          </label>

          <div className="mt-5 flex justify-end gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onOpenChange(false)}
              data-testid="approval-policy-cancel"
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              size="sm"
              disabled={!canSave}
              onClick={handleSave}
              data-testid="approval-policy-save"
            >
              Save change
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
