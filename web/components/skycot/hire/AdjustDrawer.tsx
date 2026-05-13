"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { EXAMPLE_ROLES, type ExampleRole } from "@/lib/hire/examples";
import { cn } from "@/lib/utils";

/**
 * Right-side drawer for free-text re-drafting in Step 4.
 *
 * Mounted via `@radix-ui/react-dialog` with side="right". Width is
 * `max-w-md` on desktop; full-screen on mobile. The user types a
 * free-text adjustment ("make her more formal", "shorter replies")
 * and Save fires a re-draft via `useGenesisWorkerDraft` (wired in the
 * parent). Cancel closes the drawer with no change.
 *
 * After the 3rd Adjust the drawer surfaces a `"Want to start with a
 * specific template?"` fallback below the Save button (per wireframe).
 * Picking a template short-circuits the redraft loop and re-enters
 * Step 2 from the chosen seed problem.
 */
export interface AdjustDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Number of Adjusts already saved this session (drives the fallback). */
  adjustCount: number;
  /** Disable Save while a redraft mutation is in flight. */
  saving?: boolean;
  /** Fired with the trimmed free-text on Save. */
  onSave: (adjustment: string) => void;
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
  const [text, setText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Reset the textarea each time the drawer opens so prior text
  // doesn't leak between Adjust attempts.
  useEffect(() => {
    if (open) {
      setText("");
      const t = setTimeout(() => textareaRef.current?.focus(), 50);
      return () => clearTimeout(t);
    }
    return undefined;
  }, [open]);

  const showTemplateFallback = adjustCount >= ADJUST_FALLBACK_THRESHOLD;

  const handleSave = () => {
    const trimmed = text.trim();
    if (!trimmed || saving) return;
    onSave(trimmed);
  };

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

          <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
            <textarea
              ref={textareaRef}
              data-testid="hire-step4-adjust-textarea"
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder="e.g. 'Make her more formal' or 'Shorter replies, no exclamation marks'"
              rows={6}
              className={cn(
                "block w-full resize-none rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-3 text-sm",
                "outline-none focus:ring-2 focus:ring-[var(--ring)]",
              )}
              aria-label="Adjustment text"
            />

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
              type="button"
              variant="primary"
              data-testid="hire-step4-adjust-save"
              onClick={handleSave}
              loading={saving}
              disabled={text.trim().length === 0 || saving}
            >
              Save
            </Button>
          </footer>
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
