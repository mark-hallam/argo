"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Confirm dialog opened when an operator clicks the × on a skill chip
 * in the cohort detail drawer's members list. Built on Radix's Dialog
 * primitive (focus trap + Esc-to-close + aria wiring). The caller
 * owns the DELETE call inside `onConfirm`; this modal only renders
 * the prompt, drives the in-flight state, and surfaces a short
 * inline error if the caller's promise rejects so the operator can
 * read the failure before retrying.
 */
export interface RemoveSkillConfirmModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  skillLabel: string;
  workerLabel: string;
}

export function RemoveSkillConfirmModal({
  open,
  onClose,
  onConfirm,
  skillLabel,
  workerLabel,
}: RemoveSkillConfirmModalProps) {
  const [submitting, setSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setErrorMessage(null);
      setSubmitting(false);
    }
  }, [open]);

  const handleConfirm = async () => {
    setErrorMessage(null);
    setSubmitting(true);
    try {
      await onConfirm();
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const handleOpenChange = (next: boolean) => {
    if (!next && !submitting) {
      setErrorMessage(null);
      onClose();
    }
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="remove-skill-confirm-modal"
          className={cn(
            "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
            "sm:w-[92vw] sm:max-w-md sm:-translate-x-1/2 sm:-translate-y-1/2",
            "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
            "flex flex-col focus:outline-none",
          )}
        >
          <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-6 py-5">
            <div className="flex items-center gap-2 min-w-0">
              <span
                aria-hidden
                className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--muted)] text-[var(--muted-foreground)]"
              >
                <Trash2 size={16} />
              </span>
              <div className="min-w-0">
                <DialogPrimitive.Title
                  className="text-base font-semibold text-[var(--foreground)]"
                  data-testid="remove-skill-confirm-title"
                >
                  Remove skill
                </DialogPrimitive.Title>
                <DialogPrimitive.Description className="mt-0.5 text-xs text-[var(--muted-foreground)]">
                  This removes the worker&apos;s overlay row. Reassign anytime.
                </DialogPrimitive.Description>
              </div>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              data-testid="remove-skill-confirm-close"
              disabled={submitting}
              className={cn(
                "grid size-8 place-items-center rounded-[var(--radius-md)]",
                "text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                "disabled:cursor-not-allowed disabled:opacity-50",
              )}
            >
              <X aria-hidden size={18} />
            </DialogPrimitive.Close>
          </header>

          <div className="px-6 py-4 text-sm text-[var(--foreground)]">
            <p data-testid="remove-skill-confirm-prompt">
              Remove skill{" "}
              <span className="font-semibold">{skillLabel}</span> from worker{" "}
              <span className="font-semibold">{workerLabel}</span>?
            </p>
          </div>

          {errorMessage ? (
            <div
              role="alert"
              data-testid="remove-skill-confirm-error"
              className="mx-6 mb-2 rounded-[var(--radius-md)] border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700"
            >
              {errorMessage}
            </div>
          ) : null}

          <footer className="flex items-center justify-end gap-2 border-t border-[var(--border)] px-6 py-4">
            <button
              type="button"
              data-testid="remove-skill-confirm-cancel"
              onClick={() => handleOpenChange(false)}
              disabled={submitting}
              className={cn(
                "rounded-[var(--radius-md)] border border-[var(--border)] bg-transparent px-3 py-1.5 text-sm text-[var(--foreground)]",
                "hover:bg-[var(--muted)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                "disabled:cursor-not-allowed disabled:opacity-50",
              )}
            >
              Cancel
            </button>
            <button
              type="button"
              data-testid="remove-skill-confirm-submit"
              onClick={handleConfirm}
              disabled={submitting}
              className={cn(
                "rounded-[var(--radius-md)] bg-red-600 px-3 py-1.5 text-sm font-medium text-white",
                "hover:bg-red-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500",
                "disabled:cursor-not-allowed disabled:opacity-50",
              )}
            >
              {submitting ? "Removing…" : "Remove"}
            </button>
          </footer>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
