"use client";

import * as PopoverPrimitive from "@radix-ui/react-popover";
import { HelpCircle } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { EXAMPLE_ROLES, type ExampleRole } from "@/lib/hire/examples";
import { cn } from "@/lib/utils";

/**
 * "Need some examples?" popover. Anchored to a ghost button beneath the
 * Step 1 textarea. Renders the five shipped example role descriptions
 * from `lib/hire/examples.ts`; selecting one fires `onPick(description)`
 * and dismisses the popover.
 *
 * Built on `@radix-ui/react-popover` (already a workspace dep). The
 * popover surface and trigger are exposed together so the parent can
 * stay laid-out in the Step1Land flow without having to wire its own
 * Radix `Root`.
 */
export interface ExamplesPopoverProps {
  /** Fires with the example's `description` when a row is selected. */
  onPick: (description: string) => void;
  /** Examples to render. Defaults to the v1 shipped set. */
  examples?: readonly ExampleRole[];
}

export function ExamplesPopover({
  onPick,
  examples = EXAMPLE_ROLES,
}: ExamplesPopoverProps) {
  return (
    <PopoverPrimitive.Root>
      <PopoverPrimitive.Trigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          data-testid="hire-step1-examples-trigger"
          className="text-[var(--muted-foreground)]"
        >
          <HelpCircle aria-hidden />
          Need some examples?
        </Button>
      </PopoverPrimitive.Trigger>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          align="start"
          sideOffset={6}
          className={cn(
            "z-50 w-[min(92vw,440px)]",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-2 shadow-lg",
            "focus:outline-none",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
          data-testid="hire-step1-examples-popover"
        >
          <p className="px-2 pt-1 pb-2 text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
            Pick one to start with
          </p>
          <ul className="flex flex-col gap-1">
            {examples.map((example) => (
              <li key={example.title}>
                <PopoverPrimitive.Close asChild>
                  <button
                    type="button"
                    data-testid={`hire-step1-example-${slugify(example.title)}`}
                    onClick={() => onPick(example.description)}
                    className={cn(
                      "w-full text-left rounded-[var(--radius-md)] px-3 py-2",
                      "hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]",
                      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                      "transition-colors duration-[var(--duration-fast)] ease-out",
                    )}
                  >
                    <span className="block text-sm font-medium text-[var(--foreground)]">
                      {example.title}
                    </span>
                    <span className="mt-0.5 block text-xs text-[var(--muted-foreground)] line-clamp-2">
                      {example.description}
                    </span>
                  </button>
                </PopoverPrimitive.Close>
              </li>
            ))}
          </ul>
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}

function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}
