"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useId, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * Reusable type-the-name-to-confirm dialog. Generic over the
 * expected name so it can back any "irreversible-and-audited"
 * action (Delete is the v1 caller; future callers may include
 * Reset learned memory, Hard-purge tenant). The confirm CTA stays
 * disabled until the user types `expectedName` exactly (case-
 * sensitive — matches GitHub's repo-delete pattern).
 *
 * Reset on close: re-opening the dialog after a cancel re-clears
 * the input so a half-typed name doesn't survive.
 */

export interface TypeNameToConfirmProps {
  /** Controlled open state. */
  open: boolean;
  /** Called when Radix asks to close. */
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  /** Each entry renders on its own line in the description. */
  bodyLines?: ReactNode[];
  /** Exact string the user must type to enable the CTA. */
  expectedName: string;
  /** Confirm-button copy. */
  cta: string;
  /** Fires when the user activates the confirm CTA. */
  onConfirm: () => void;
}

export function TypeNameToConfirm({
  open,
  onOpenChange,
  title,
  bodyLines,
  expectedName,
  cta,
  onConfirm,
}: TypeNameToConfirmProps) {
  const inputId = useId();
  const [typed, setTyped] = useState("");

  useEffect(() => {
    if (!open) setTyped("");
  }, [open]);

  const matches = typed === expectedName;

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
          role="alertdialog"
          aria-modal="true"
          data-testid="type-name-dialog"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            {title}
          </DialogPrimitive.Title>
          {bodyLines && bodyLines.length > 0 ? (
            <DialogPrimitive.Description asChild>
              <div className="mt-2 space-y-1 text-sm text-[var(--muted-foreground)]">
                {bodyLines.map((line, i) => (
                  <p key={i}>{line}</p>
                ))}
              </div>
            </DialogPrimitive.Description>
          ) : null}
          <label htmlFor={inputId} className="mt-4 block text-xs font-medium text-[var(--muted-foreground)]">
            Type <span className="font-mono text-[var(--foreground)]">{expectedName}</span> to confirm:
          </label>
          <input
            id={inputId}
            type="text"
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            data-testid="type-name-input"
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            className={cn(
              "mt-1 w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm",
              "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
            )}
          />
          <div className="mt-6 flex justify-end gap-2">
            <Button
              variant="ghost"
              data-testid="type-name-cancel"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              data-testid="type-name-confirm"
              disabled={!matches}
              onClick={() => {
                if (!matches) return;
                onConfirm();
                onOpenChange(false);
              }}
            >
              {cta}
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
