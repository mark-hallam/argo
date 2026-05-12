"use client";

import { useEffect, useRef } from "react";
import { ExamplesPopover } from "@/components/skycot/hire/ExamplesPopover";
import { type UseStep1Result } from "@/lib/hire/use-step1";

/**
 * Hire flow — Step 1 (Land).
 *
 * The view layer for the first hire-flow step: headline, textarea
 * with auto-grow, examples popover trigger, and the deferred
 * validation hint. The owning page hosts `useStep1` and passes the
 * result in so the HireShell's footer Continue can reflect the same
 * `canSubmit` state without lifting it across a ref boundary.
 *
 * Auto-grow uses the textarea's `scrollHeight` (with a 400px clamp
 * matching the wireframe's `max-h-[400px]`). The textarea's own scroll
 * surfaces once content exceeds the clamp.
 *
 * Accessibility:
 *  - The `<h1>` provides the page's accessible name.
 *  - A visually-hidden `<label>` programmatically labels the textarea
 *    (screen readers announce it; the visible heading is decorative
 *    for the input).
 *  - The validation hint is wrapped in a `role="status"` live region
 *    so it's announced when it appears.
 */
export interface Step1LandProps {
  step1: UseStep1Result;
}

const MAX_HEIGHT_PX = 400;

export function Step1Land({ step1 }: Step1LandProps) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Auto-grow: reset to auto so shrink works, then size to scrollHeight
  // clamped at MAX_HEIGHT_PX. Runs on every value change so paste +
  // delete both resize cleanly.
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    const next = Math.min(el.scrollHeight, MAX_HEIGHT_PX);
    el.style.height = `${next}px`;
  }, [step1.value]);

  return (
    <section className="mx-auto max-w-2xl space-y-6" data-testid="hire-step1-land">
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight text-[var(--foreground)]">
          What do you need help with?
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Tell me in your own words. Anything you&apos;d love to stop doing yourself.
        </p>
      </header>

      <div className="space-y-3">
        <label htmlFor="hire-step1-textarea" className="sr-only">
          What do you need help with?
        </label>
        <textarea
          id="hire-step1-textarea"
          ref={textareaRef}
          data-testid="hire-step1-textarea"
          value={step1.value}
          onChange={(event) => step1.setValue(event.target.value)}
          onKeyDown={step1.handleKeyDown}
          placeholder={step1.placeholder}
          rows={5}
          className={[
            "w-full resize-none overflow-y-auto",
            "min-h-[160px] max-h-[400px]",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)]",
            "px-4 py-3 text-base text-[var(--foreground)] placeholder:text-[var(--muted-foreground)]",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
            "transition-shadow duration-[var(--duration-fast)] ease-out",
          ].join(" ")}
        />

        <div className="flex items-start justify-between gap-4">
          <ExamplesPopover onPick={step1.fillFromExample} />
          {step1.showValidationHint ? (
            <p
              role="status"
              data-testid="hire-step1-validation-hint"
              className="text-xs text-[var(--muted-foreground)]"
            >
              Tell me a bit more — even one sentence is enough.
            </p>
          ) : null}
        </div>
      </div>
    </section>
  );
}
