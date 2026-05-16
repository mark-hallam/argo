"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { AnimatePresence, motion as fm } from "motion/react";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Button, type ButtonProps } from "@/components/ui/Button";
import { useMotion } from "@/lib/motion/provider";
import { cn } from "@/lib/utils";

/**
 * Minimal alert-style confirm dialog built on `@radix-ui/react-dialog`.
 *
 * The hire-flow brief calls for shadcn's `AlertDialog`, which is
 * normally built on `@radix-ui/react-alert-dialog`. That package is
 * not installed in this workspace and adding it would touch
 * `package.json` / `pnpm-lock.yaml` outside this brief's may-touch
 * list, so the equivalent UX (modal, two buttons, focus trap) is
 * composed here from the regular dialog primitive (already installed).
 *
 * Public API mirrors shadcn's AlertDialog so a future swap is a single
 * import change.
 */

export interface AlertDialogProps {
  /** Controlled open state. */
  open: boolean;
  /** Called when Radix asks to close (escape, click outside, button). */
  onOpenChange: (open: boolean) => void;
  /** Bold question / headline. */
  title: ReactNode;
  /** Supporting copy beneath the title. */
  description?: ReactNode;
  /** Primary destructive/affirmative button label. */
  confirmLabel: string;
  /** Secondary "keep working" button label. */
  cancelLabel: string;
  /** Variant of the confirm button. Defaults to `destructive`. */
  confirmVariant?: ButtonProps["variant"];
  /** Fires when the confirm button is activated. */
  onConfirm: () => void;
  /**
   * Fires when the cancel button is activated. If omitted, cancel
   * just closes the dialog via `onOpenChange(false)`.
   */
  onCancel?: () => void;
}

export function AlertDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  cancelLabel,
  confirmVariant = "destructive",
  onConfirm,
  onCancel,
}: AlertDialogProps) {
  const m = useMotion();
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <AnimatePresence>
        {open ? (
          <DialogPrimitive.Portal forceMount>
            <DialogPrimitive.Overlay asChild>
              <fm.div
                className={cn("fixed inset-0 z-40 bg-black/40 backdrop-blur-sm")}
                {...m.fadeIn}
              />
            </DialogPrimitive.Overlay>
            <DialogPrimitive.Content asChild>
              <fm.div
                role="alertdialog"
                aria-modal="true"
                className={cn(
                  "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2",
                  "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
                  "focus:outline-none",
                )}
                data-testid="alert-dialog"
                initial={m.springModal.initial}
                animate={m.springModal.animate}
                exit={m.springModal.exit}
                transition={m.springModal.transition}
              >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            {title}
          </DialogPrimitive.Title>
          {description ? (
            <DialogPrimitive.Description className="mt-2 text-sm text-[var(--muted-foreground)]">
              {description}
            </DialogPrimitive.Description>
          ) : null}
          <div className="mt-6 flex justify-end gap-2">
            <Button
              variant="ghost"
              data-testid="alert-dialog-cancel"
              onClick={() => {
                onCancel?.();
                onOpenChange(false);
              }}
            >
              {cancelLabel}
            </Button>
            <Button
              variant={confirmVariant}
              data-testid="alert-dialog-confirm"
              onClick={() => {
                onConfirm();
                onOpenChange(false);
              }}
            >
              {confirmLabel}
            </Button>
          </div>
              </fm.div>
            </DialogPrimitive.Content>
          </DialogPrimitive.Portal>
        ) : null}
      </AnimatePresence>
    </DialogPrimitive.Root>
  );
}

// Re-exported types for callers that wrap the primitive.
export type AlertDialogContentProps = ComponentPropsWithoutRef<typeof DialogPrimitive.Content>;
