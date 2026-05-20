"use client";

import { useEffect } from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import {
  APPROVAL_POLICIES,
  APPROVAL_POLICY_HELP,
  APPROVAL_POLICY_LABELS,
  type ApprovalPolicy,
  type WorkerPolicyRow,
} from "@/lib/api/worker-policies";
import { approvalPolicyFormSchema } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Approval-policy edit modal. Operator picks one of the four
 * canonical policies and writes a required note explaining why.
 * RHF + Zod gates Save behind a non-empty trimmed note + a policy
 * that differs from the current one.
 */

export interface ApprovalPolicyEditModalProps {
  open: boolean;
  row: WorkerPolicyRow | null;
  isSaving?: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (vars: {
    actionType: string;
    policy: ApprovalPolicy;
    operatorNote: string;
  }) => Promise<void> | void;
}

export function ApprovalPolicyEditModal({
  open,
  row,
  isSaving = false,
  onOpenChange,
  onSubmit,
}: ApprovalPolicyEditModalProps) {
  const toast = useToast();
  const form = useZodForm(approvalPolicyFormSchema, {
    defaultValues: {
      policy: row?.currentPolicy ?? "risk-gated",
      operatorNote: "",
    },
  });
  const { register, handleSubmit, reset, formState, watch } = form;

  useEffect(() => {
    if (open && row) {
      reset({ policy: row.currentPolicy, operatorNote: "" });
    }
  }, [open, row, reset]);

  const policy = watch("policy");
  const note = watch("operatorNote") ?? "";
  const policyUnchanged = row !== null && policy === row.currentPolicy;
  const submitDisabled =
    !row ||
    policyUnchanged ||
    note.trim().length === 0 ||
    formState.isSubmitting ||
    isSaving;

  const submit = handleSubmit(async (values) => {
    if (!row) return;
    try {
      await onSubmit({
        actionType: row.actionType,
        policy: values.policy,
        operatorNote: values.operatorNote,
      });
      toast.success("Approval policy updated");
    } catch (err) {
      toast.error("Couldn't update policy", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

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

          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
            noValidate
          >
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
                      value={opt}
                      data-testid={`approval-policy-radio-${opt}`}
                      className="mt-1 size-3.5 accent-[var(--primary)]"
                      {...register("policy")}
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
                data-testid="approval-policy-note"
                placeholder="Why are you changing this?"
                rows={3}
                maxLength={1024}
                aria-invalid={Boolean(formState.errors.operatorNote)}
                className="mt-1 block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)] focus:border-[var(--primary)] focus:outline-none"
                {...register("operatorNote")}
              />
              {formState.errors.operatorNote ? (
                <span
                  role="alert"
                  data-testid="approval-policy-error-note"
                  className="mt-1 block text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.operatorNote.message}
                </span>
              ) : null}
            </label>

            <div className="mt-5 flex justify-end gap-2">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => onOpenChange(false)}
                data-testid="approval-policy-cancel"
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="primary"
                size="sm"
                disabled={submitDisabled}
                loading={formState.isSubmitting || isSaving}
                data-testid="approval-policy-save"
              >
                Save change
              </Button>
            </div>
          </form>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
