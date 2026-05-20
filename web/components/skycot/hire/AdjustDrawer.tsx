"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import { EXAMPLE_ROLES, type ExampleRole } from "@/lib/hire/examples";
import { adjustDrawerFormSchema } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Right-side drawer for free-text re-drafting in Step 4.
 *
 * Mounted via `@radix-ui/react-dialog` with side="right". Width is
 * `max-w-md` on desktop; full-screen on mobile. RHF + Zod validates
 * the free-text payload (non-empty, ≤ 2000 chars) before firing
 * `onSave`. The Save button stays loading + disabled while the
 * `saving` prop is true (parent's redraft mutation in flight) so
 * double-submit is impossible.
 *
 * After the 3rd Adjust the drawer surfaces a `"Want to start with a
 * specific template?"` fallback below the Save button (per wireframe).
 */
export interface AdjustDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Number of Adjusts already saved this session (drives the fallback). */
  adjustCount: number;
  /** Disable Save while a redraft mutation is in flight. */
  saving?: boolean;
  /** Fired with the trimmed free-text on Save. */
  onSave: (adjustment: string) => void | Promise<void>;
  /** Fired when the user picks a template tile in the fallback panel. */
  onPickTemplate: (example: ExampleRole) => void;
}

const ADJUST_FALLBACK_THRESHOLD = 3;

export function AdjustDrawer({
  open,
  onOpenChange,
  adjustCount,
  saving = false,
  onSave,
  onPickTemplate,
}: AdjustDrawerProps) {
  const toast = useToast();
  const form = useZodForm(adjustDrawerFormSchema, {
    defaultValues: { text: "" },
  });
  const { register, handleSubmit, reset, formState } = form;
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Reset each time the drawer opens so prior text doesn't leak
  // between Adjust attempts.
  useEffect(() => {
    if (open) {
      reset({ text: "" });
      const t = setTimeout(() => textareaRef.current?.focus(), 50);
      return () => clearTimeout(t);
    }
    return undefined;
  }, [open, reset]);

  const showTemplateFallback = adjustCount >= ADJUST_FALLBACK_THRESHOLD;
  const submitting = formState.isSubmitting || saving;

  const submit = handleSubmit(async (values) => {
    try {
      await onSave(values.text);
    } catch (err) {
      toast.error("Couldn't apply adjustment", {
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
            "fixed inset-0 z-40 bg-black/40",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          data-testid="hire-step4-adjust-drawer"
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex h-full w-full max-w-md flex-col",
            "border-l border-[var(--border)] bg-[var(--background)] shadow-xl",
            "focus:outline-none",
          )}
        >
          <header className="flex items-start justify-between gap-3 border-b border-[var(--border)] px-5 py-4">
            <div className="space-y-1">
              <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
                Adjust your teammate
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="text-xs text-[var(--muted-foreground)]">
                What would you change about how they work?
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close
              data-testid="hire-step4-adjust-close"
              className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]"
              aria-label="Close adjust drawer"
            >
              <X className="size-4" />
            </DialogPrimitive.Close>
          </header>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
            noValidate
            className="flex flex-1 flex-col"
          >
            <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
              <textarea
                ref={(el) => {
                  rhfRef(el);
                  textareaRef.current = el;
                }}
                data-testid="hire-step4-adjust-textarea"
                placeholder="e.g. 'Make her more formal' or 'Shorter replies, no exclamation marks'"
                rows={6}
                aria-label="Adjustment text"
                aria-invalid={Boolean(formState.errors.text)}
                className={cn(
                  "block w-full resize-none rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-3 text-sm",
                  "outline-none focus:ring-2 focus:ring-[var(--ring)]",
                )}
                {...rhfRest}
              />
              {formState.errors.text ? (
                <p
                  role="alert"
                  data-testid="hire-step4-adjust-error"
                  className="text-xs text-[var(--destructive)]"
                >
                  {formState.errors.text.message}
                </p>
              ) : null}

              {showTemplateFallback ? (
                <TemplateFallback onPick={onPickTemplate} />
              ) : null}
            </div>

            <footer className="flex justify-end gap-2 border-t border-[var(--border)] px-5 py-4">
              <Button
                type="button"
                variant="ghost"
                data-testid="hire-step4-adjust-cancel"
                onClick={() => onOpenChange(false)}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="primary"
                data-testid="hire-step4-adjust-save"
                loading={submitting}
                disabled={submitting}
              >
                Save
              </Button>
            </footer>
          </form>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

interface TemplateFallbackProps {
  onPick: (example: ExampleRole) => void;
}

function TemplateFallback({ onPick }: TemplateFallbackProps) {
  return (
    <section
      data-testid="hire-step4-adjust-template-fallback"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] p-3 space-y-2"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">
        Want to start with a specific template?
      </p>
      <p className="text-xs text-[var(--muted-foreground)]">
        After a few rounds of adjusting, picking a closer template often
        works better than re-drafting from scratch.
      </p>
      <div className="grid gap-2">
        {EXAMPLE_ROLES.map((example) => (
          <Button
            key={example.id}
            type="button"
            variant="secondary"
            size="sm"
            className="h-auto justify-start whitespace-normal px-3 py-2 text-left"
            data-testid={`hire-step4-adjust-template-${example.id}`}
            onClick={() => onPick(example)}
          >
            <span className="block w-full">
              <span className="block text-sm font-medium">{example.title}</span>
              <span className="mt-1 block text-xs text-[var(--muted-foreground)]">
                {example.description}
              </span>
            </span>
          </Button>
        ))}
      </div>
    </section>
  );
}
