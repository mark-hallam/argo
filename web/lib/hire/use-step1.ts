"use client";

import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { PLACEHOLDER_PROMPTS, STEP1_MIN_CHARS } from "./examples";

/**
 * Step 1 (Land) form state — owns the textarea value, rotating
 * placeholder pick, premature-submit validation flag, and the
 * Cmd/Ctrl+Enter shortcut. Routing + draft persistence live with the
 * page so the hook stays presentation-agnostic. The validation hint
 * auto-dismisses 5s after it surfaces, or immediately when the user
 * types — whichever fires first.
 */
export interface UseStep1Options {
  /** Initial textarea value (from draft store rehydration). */
  initialValue?: string;
  /** Fires when the user submits with ≥ 15 chars. */
  onSubmit: (value: string) => void;
}

export interface UseStep1Result {
  value: string;
  setValue: (next: string) => void;
  /** True once the textarea has ≥ STEP1_MIN_CHARS characters. */
  canSubmit: boolean;
  /** Show the "Tell me a bit more" hint after a premature submit attempt. */
  showValidationHint: boolean;
  /**
   * The single placeholder picked on mount. Stable for the session so
   * the textarea doesn't re-roll on each render.
   */
  placeholder: string;
  /** Pre-bound `onKeyDown` for the textarea — handles Cmd/Ctrl+Enter. */
  handleKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  /** Try to advance; surfaces the validation hint if too short. */
  attemptSubmit: () => void;
  /** Programmatically fill the textarea (used by examples popover). */
  fillFromExample: (text: string) => void;
}

const HINT_DISMISS_MS = 5000;

function pickPlaceholder(): string {
  const idx = Math.floor(Math.random() * PLACEHOLDER_PROMPTS.length);
  return PLACEHOLDER_PROMPTS[idx] ?? PLACEHOLDER_PROMPTS[0]!;
}

export function useStep1({ initialValue = "", onSubmit }: UseStep1Options): UseStep1Result {
  const [value, setValueState] = useState(initialValue);
  const [showValidationHint, setShowValidationHint] = useState(false);
  // Lazy initializer so the random pick is stable across renders without
  // re-running on every commit. `useState(() => …)` is the textbook idiom.
  const [placeholder] = useState<string>(() => pickPlaceholder());
  const dismissTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearTimer = useCallback(() => {
    if (dismissTimer.current !== null) {
      clearTimeout(dismissTimer.current);
      dismissTimer.current = null;
    }
  }, []);

  useEffect(() => clearTimer, [clearTimer]);

  const setValue = useCallback(
    (next: string) => {
      setValueState(next);
      if (showValidationHint) {
        setShowValidationHint(false);
        clearTimer();
      }
    },
    [showValidationHint, clearTimer],
  );

  const fillFromExample = useCallback(
    (text: string) => {
      setValueState(text);
      setShowValidationHint(false);
      clearTimer();
    },
    [clearTimer],
  );

  const canSubmit = value.trim().length >= STEP1_MIN_CHARS;

  const attemptSubmit = useCallback(() => {
    if (canSubmit) {
      onSubmit(value.trim());
      return;
    }
    setShowValidationHint(true);
    clearTimer();
    dismissTimer.current = setTimeout(() => {
      setShowValidationHint(false);
      dismissTimer.current = null;
    }, HINT_DISMISS_MS);
  }, [canSubmit, value, onSubmit, clearTimer]);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key !== "Enter") return;
      if (!(event.metaKey || event.ctrlKey)) return;
      event.preventDefault();
      attemptSubmit();
    },
    [attemptSubmit],
  );

  return {
    value,
    setValue,
    canSubmit,
    showValidationHint,
    placeholder,
    handleKeyDown,
    attemptSubmit,
    fillFromExample,
  };
}
