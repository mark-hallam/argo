"use client";

import { useEffect, useRef, type RefObject } from "react";

/**
 * Selector for natively-focusable elements inside a dialog. Mirrors the
 * WAI-ARIA Authoring Practices focusable set; `[hidden]` and
 * `disabled` elements are excluded so the trap never lands focus on an
 * inert control.
 */
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "area[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
  "audio[controls]",
  "video[controls]",
  '[contenteditable]:not([contenteditable="false"])',
].join(",");

function isVisible(el: HTMLElement): boolean {
  // happy-dom / jsdom don't lay out, so offset checks are unreliable in
  // tests; treat an element as visible unless it is explicitly hidden.
  if (el.hasAttribute("hidden")) return false;
  if (el.getAttribute("aria-hidden") === "true") return false;
  return true;
}

function focusableWithin(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter(isVisible);
}

export interface UseFocusTrapOptions {
  /** Whether the trap is active. Defaults to `true`. */
  active?: boolean;
}

/**
 * Accessible focus management for a hand-rolled modal dialog.
 *
 * Given a ref to the dialog element plus an `onClose` callback, it:
 * 1. captures `document.activeElement` when the trap activates, then
 *    moves focus to the first focusable child (or the dialog itself),
 * 2. traps Tab / Shift+Tab so focus cycles within the dialog,
 * 3. invokes `onClose` on Escape,
 * 4. restores focus to the originally-focused element when the dialog
 *    unmounts or the trap deactivates.
 *
 * Pure and SSR-safe: every `document` access is inside the effect, which
 * never runs on the server. No external dependencies.
 *
 * The dialog element should carry `role="dialog"`, `aria-modal="true"`,
 * and an accessible name (`aria-label` / `aria-labelledby`). For the
 * dialog itself to receive focus when it has no focusable children, give
 * it `tabIndex={-1}`.
 *
 * @example
 * ```tsx
 * function Modal({ onClose }: { onClose: () => void }) {
 *   const ref = useRef<HTMLDivElement>(null);
 *   useFocusTrap(ref, onClose);
 *   return (
 *     <div ref={ref} role="dialog" aria-modal="true" aria-label="Settings" tabIndex={-1}>
 *       <button onClick={onClose}>Close</button>
 *     </div>
 *   );
 * }
 * ```
 */
export function useFocusTrap(
  dialogRef: RefObject<HTMLElement | null>,
  onClose: () => void,
  options: UseFocusTrapOptions = {},
): void {
  const active = options.active ?? true;

  // Keep the latest `onClose` in a ref so the trap effect can stay pinned
  // to `[active, dialogRef]` (re-running it would re-capture focus and
  // fire the restore cleanup on every render) while always calling the
  // current handler — no stale-closure risk even with inline arrows.
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!active) return;
    const dialog = dialogRef.current;
    if (!dialog || typeof document === "undefined") return;

    const previouslyFocused =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;

    // Move focus inward: first focusable child, else the dialog itself.
    const initial = focusableWithin(dialog);
    if (initial.length > 0) {
      initial[0]?.focus();
    } else {
      dialog.focus();
    }

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.stopPropagation();
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab") return;

      const current = dialogRef.current;
      if (!current) return;
      const focusable = focusableWithin(current);
      if (focusable.length === 0) {
        // Nothing focusable but the dialog — keep focus on it.
        event.preventDefault();
        current.focus();
        return;
      }

      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      const activeEl = document.activeElement;

      if (event.shiftKey) {
        // Shift+Tab off the first element (or from outside) wraps to last.
        if (activeEl === first || !current.contains(activeEl)) {
          event.preventDefault();
          last.focus();
        }
      } else if (activeEl === last || !current.contains(activeEl)) {
        // Tab off the last element (or from outside) wraps to first.
        event.preventDefault();
        first.focus();
      }
    }

    // Capture phase so the trap sees Tab before inner handlers and the
    // event can't escape the dialog to background controls.
    document.addEventListener("keydown", onKeyDown, true);

    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      // Restore focus to whatever was focused before the dialog opened.
      if (previouslyFocused && document.contains(previouslyFocused)) {
        try {
          previouslyFocused.focus();
        } catch {
          // element may have become unfocusable — ignore
        }
      }
    };
    // `onClose` is read through `onCloseRef` (kept fresh above), so it is
    // intentionally absent from the deps: the trap must NOT re-run on every
    // render or it would re-capture focus and fire the restore cleanup
    // mid-interaction.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, dialogRef]);
}
