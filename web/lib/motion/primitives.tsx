"use client";

import { AnimatePresence, motion as fm, type Variants } from "motion/react";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { useMotion } from "@/lib/motion/provider";

/**
 * Wrap an App-Router segment to fade-up between routes. Keyed on
 * pathname so each route swap remounts the motion tree, triggering
 * the enter animation. Use directly inside route-group layouts.
 */
export function RouteTransition({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const m = useMotion();
  return (
    <AnimatePresence mode="wait" initial={false}>
      <fm.div key={pathname} {...m.fadeUp}>
        {children}
      </fm.div>
    </AnimatePresence>
  );
}

/**
 * Stagger-animate a list of children. Pair with `<MotionListItem>`
 * (or any motion-aware child) — the parent's `staggerChildren`
 * transition cascades into each item's `animate` variant.
 */
export function MotionList({
  children,
  className,
  as = "div",
}: {
  children: ReactNode;
  className?: string;
  as?: "div" | "ul" | "ol";
}) {
  const m = useMotion();
  const Tag = as === "ul" ? fm.ul : as === "ol" ? fm.ol : fm.div;
  const variants: Variants = {
    initial: {},
    animate: { transition: m.staggerChildren.animate?.transition },
  };
  return (
    <Tag className={className} initial="initial" animate="animate" variants={variants}>
      {children}
    </Tag>
  );
}

export function MotionListItem({
  children,
  className,
  as = "div",
}: {
  children: ReactNode;
  className?: string;
  as?: "div" | "li";
}) {
  const m = useMotion();
  const Tag = as === "li" ? fm.li : fm.div;
  const variants: Variants = {
    initial: m.fadeUp.initial ?? {},
    animate: m.fadeUp.animate ?? {},
  };
  return (
    <Tag className={className} variants={variants} transition={m.fadeUp.transition}>
      {children}
    </Tag>
  );
}

/**
 * Crossfade between two children keyed by `swapKey` — typically a
 * skeleton vs the resolved content. Honors reduced-motion (zero
 * duration via the vocabulary).
 */
export function Crossfade({
  swapKey,
  children,
  className,
}: {
  swapKey: string;
  children: ReactNode;
  className?: string;
}) {
  const m = useMotion();
  return (
    <AnimatePresence mode="wait" initial={false}>
      <fm.div key={swapKey} className={className} {...m.crossfade}>
        {children}
      </fm.div>
    </AnimatePresence>
  );
}
