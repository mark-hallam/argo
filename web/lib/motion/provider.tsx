"use client";

import { MotionConfig, useReducedMotion } from "motion/react";
import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  motion as baseMotion,
  reducedMotionFallbacks,
  type MotionVocabulary,
} from "@/lib/motion/variants";

const MotionVocabContext = createContext<MotionVocabulary>(baseMotion);

/**
 * Read the currently-active vocabulary. Components consume motion via
 * `const m = useMotion()` so the reduced-motion swap happens
 * transparently — no per-callsite media-query checks.
 */
export function useMotion(): MotionVocabulary {
  return useContext(MotionVocabContext);
}

export interface MotionRootProps {
  children: ReactNode;
  /** Test-only override. When set, bypasses the `useReducedMotion` hook. */
  forceReducedMotion?: boolean;
}

/**
 * Mount once near the top of the tree (inside `(app)/layout.tsx`).
 * Detects `prefers-reduced-motion: reduce` and swaps the vocabulary
 * to zero-duration fallbacks for every descendant that calls
 * `useMotion()`. Also configures `MotionConfig` so that primitive
 * `<motion.div>` callers honor the same setting even without
 * touching the vocab hook.
 */
export function MotionRoot({ children, forceReducedMotion }: MotionRootProps) {
  const prefersReduced = useReducedMotion();
  const reduced = forceReducedMotion ?? prefersReduced ?? false;
  const vocab = useMemo(
    () => (reduced ? reducedMotionFallbacks : baseMotion),
    [reduced],
  );
  return (
    <MotionConfig reducedMotion={reduced ? "always" : "never"}>
      <MotionVocabContext.Provider value={vocab}>{children}</MotionVocabContext.Provider>
    </MotionConfig>
  );
}
