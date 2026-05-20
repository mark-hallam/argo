"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import { addFactFormSchema } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Add-a-fact dialog. Modal with a free-text textarea (≥ 4 rows)
 * plus Save / Cancel. RHF + Zod handles validation; the form
 * disables Save while the parent's mutation is in flight to block
 * double-submit.
 */

export interface AddFactDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Worker first-name; surfaces in the heading. */
  workerName: string;
  /** Returns a promise so the dialog can hold the button in a
   *  pending state while the optimistic mutation resolves. */
  onSave: (text: string) => void | Promise<void>;
}

export function AddFactDialog({
  open,
  onOpenChange,
  workerName,
  onSave,
}: AddFactDialogProps) {
  const toast = useToast();
  const form = useZodForm(addFactFormSchema, {
    defaultValues: { text: "" },
  });
  const { register, handleSubmit, reset, formState, watch } = form;
  const draft = watch("text");
  const saveDisabled = formState.isSubmitting || draft.trim().length === 0;
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (!open) reset({ text: "" });
  }, [open, reset]);

  useEffect(() => {
    if (open && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [open]);

  const submit = handleSubmit(async (values) => {
    try {
      await onSave(values.text);
      toast.success("Fact saved");
      reset({ text: "" });
      onOpenChange(false);
    } catch (err) {
      toast.error("Couldn't save fact", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

  const { ref: rhfRef, ...rhfRest } = register("text");

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
          data-testid="add-fact-dialog"
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            Teach {workerName} a new fact
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
            Plain English. She&apos;ll apply it next time the situation comes up.
          </DialogPrimitive.Description>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
            noValidate
          >
            <textarea
              ref={(el) => {
                rhfRef(el);
                textareaRef.current = el;
              }}
              rows={5}
              data-testid="add-fact-textarea"
              aria-label="New fact text"
              aria-invalid={Boolean(formState.errors.text)}
              placeholder="e.g. Refunds under $50 auto-approve. Above $50 ask Marta."
              className={cn(
                "mt-4 w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
                "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
                "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
              )}
              {...rhfRest}
            />
            {formState.errors.text ? (
              <p
                role="alert"
                data-testid="add-fact-error"
                className="mt-2 text-xs text-[var(--destructive)]"
              >
                {formState.errors.text.message}
              </p>
            ) : null}

            <div className="mt-6 flex justify-end gap-2">
              <Button
                type="button"
                variant="ghost"
                data-testid="add-fact-cancel"
                onClick={() => onOpenChange(false)}
                disabled={formState.isSubmitting}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="primary"
                data-testid="add-fact-save"
                disabled={saveDisabled}
                loading={formState.isSubmitting}
              >
                Save
              </Button>
            </div>
          </form>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
