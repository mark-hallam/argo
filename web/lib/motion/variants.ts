import type { Transition, Variants } from "motion/react";

/**
 * Loose target shape. Framer's `Target` type isn't exported from the
 * top-level package and its CSS-property union is too strict to typecheck
 * statically-defined animations through. Consumers pass these to
 * `motion.div` `initial` / `animate` / `exit` props; framer accepts
 * arbitrary style-keyed records at runtime.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type MotionTarget = Record<string, any>;
/** `MotionTarget` extended with an optional per-target transition. */
export type MotionTargetWithTransition = MotionTarget & { transition?: Transition };

/**
 * Brand-locked motion vocabulary.
 *
 * Single source of truth for every animation in the SPA. Component
 * primitives compose these; bespoke per-component transitions are
 * not allowed.
 *
 * Timings match the `--motion-*` tokens in `web/app/globals.css`:
 *   --motion-fast: 150ms
 *   --motion-base: 250ms
 *   --motion-slow: 400ms (ceiling — no animation exceeds this)
 *   --motion-ease: cubic-bezier(0.4, 0, 0.2, 1)
 *
 * Discipline (enforced by the test suite):
 *   - No animation longer than 400ms.
 *   - No spring scale exceeds 1.05.
 *   - `prefers-reduced-motion: reduce` → all durations 0.
 */

const EASE_STANDARD: [number, number, number, number] = [0.4, 0, 0.2, 1];

export const DURATION_FAST = 0.15;
export const DURATION_BASE = 0.18;
export const DURATION_SCALE = 0.2;
export const DURATION_SLOW = 0.4;

export const MAX_DURATION_S = 0.4;
export const MAX_SPRING_SCALE = 1.05;

export interface MotionVariant {
  initial?: MotionTarget;
  animate?: MotionTargetWithTransition;
  exit?: MotionTarget;
  transition?: Transition;
}

export interface MotionVocabulary {
  fadeUp: MotionVariant;
  fadeIn: MotionVariant;
  scaleIn: MotionVariant;
  staggerChildren: MotionVariant;
  successPulse: MotionVariant;
  springModal: MotionVariant;
  crossfade: MotionVariant;
}

export const motion: MotionVocabulary = {
  fadeUp: {
    initial: { opacity: 0, y: 8 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: 8 },
    transition: { duration: DURATION_BASE, ease: EASE_STANDARD },
  },
  fadeIn: {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: DURATION_FAST, ease: "easeOut" },
  },
  scaleIn: {
    initial: { opacity: 0, scale: 0.96 },
    animate: { opacity: 1, scale: 1 },
    exit: { opacity: 0, scale: 0.96 },
    transition: { duration: DURATION_SCALE, ease: EASE_STANDARD },
  },
  staggerChildren: {
    animate: { transition: { staggerChildren: 0.04 } },
  },
  successPulse: {
    animate: {
      scale: [1, 1.03, 1],
      transition: { duration: DURATION_SLOW, ease: "easeOut" },
    },
  },
  springModal: {
    initial: { opacity: 0, scale: 0.98 },
    animate: { opacity: 1, scale: 1 },
    exit: { opacity: 0, scale: 0.98 },
    transition: { type: "spring", stiffness: 340, damping: 32 } as Transition,
  },
  crossfade: {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: DURATION_BASE, ease: EASE_STANDARD },
  },
} as const;

const zero: Transition = { duration: 0 };

export const reducedMotionFallbacks: MotionVocabulary = {
  fadeUp: { initial: { opacity: 1, y: 0 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 1, y: 0 }, transition: zero },
  fadeIn: { initial: { opacity: 1 }, animate: { opacity: 1 }, exit: { opacity: 1 }, transition: zero },
  scaleIn: { initial: { opacity: 1, scale: 1 }, animate: { opacity: 1, scale: 1 }, exit: { opacity: 1, scale: 1 }, transition: zero },
  staggerChildren: { animate: { transition: { staggerChildren: 0 } } },
  successPulse: { animate: { scale: 1, transition: zero } },
  springModal: { initial: { opacity: 1, scale: 1 }, animate: { opacity: 1, scale: 1 }, exit: { opacity: 1, scale: 1 }, transition: zero },
  crossfade: { initial: { opacity: 1 }, animate: { opacity: 1 }, exit: { opacity: 1 }, transition: zero },
};

/**
 * Convert a motion variant into Framer-Motion `Variants` records keyed
 * by `initial` / `animate` / `exit` for use with `variants={...}` plus
 * `animate="animate"`. Most consumers can pass the variant object
 * directly via `{...motion.fadeUp}` instead.
 */
export function toVariants(v: MotionVariant): Variants {
  const out: Record<string, MotionTarget> = {};
  if (v.initial) out.initial = v.initial;
  if (v.animate) out.animate = v.animate;
  if (v.exit) out.exit = v.exit;
  return out as Variants;
}
