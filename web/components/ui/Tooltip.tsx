"use client";

import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * Shadcn-style wrapper around `@radix-ui/react-tooltip`. Vendored once so
 * the SPA has a single Tooltip surface: keyboard-accessible, theme-aware,
 * `prefers-reduced-motion` respecting, replaces the scattered native
 * `title="..."` attributes that fail keyboard a11y.
 */

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;
export const TooltipPortal = TooltipPrimitive.Portal;

export interface TooltipContentProps
  extends React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content> {
  /** Tonal variant. `default` is brand-restrained popover; `inverted`
   *  is for tooltips on dark surfaces (icon-button strips, etc.). */
  variant?: "default" | "inverted";
}

export const TooltipContent = React.forwardRef<
  React.ElementRef<typeof TooltipPrimitive.Content>,
  TooltipContentProps
>(({ className, sideOffset = 6, variant = "default", ...props }, ref) => (
  <TooltipPrimitive.Content
    ref={ref}
    sideOffset={sideOffset}
    className={cn(
      "z-50 max-w-xs overflow-hidden rounded-md px-3 py-1.5",
      "text-xs font-medium leading-relaxed shadow-md",
      "transition-opacity duration-150 ease-[var(--motion-ease)]",
      "data-[state=closed]:opacity-0 data-[state=delayed-open]:opacity-100",
      "data-[state=instant-open]:opacity-100",
      "motion-reduce:transition-none",
      variant === "default"
        && "border border-[var(--border)] bg-[var(--popover)] text-[var(--popover-foreground)]",
      variant === "inverted"
        && "border border-transparent bg-[var(--foreground)] text-[var(--background)]",
      className,
    )}
    {...props}
  />
));
TooltipContent.displayName = TooltipPrimitive.Content.displayName;

export interface HoverHintProps {
  /** Tooltip body — string or rich React node. */
  label: React.ReactNode;
  /** The trigger element. Use a single React element so `asChild`
   *  delegates focus/keyboard semantics to it. */
  children: React.ReactElement;
  /** Preferred side. Radix flips automatically on collision. */
  side?: "top" | "right" | "bottom" | "left";
  /** Per-instance open delay in ms. Overrides the provider default. */
  delayMs?: number;
  /** Tonal variant for the content. */
  variant?: "default" | "inverted";
  /** Extra classes for the content surface. */
  className?: string;
}

/**
 * Sugar for the common case: wrap a trigger with `<HoverHint label="...">`
 * and get keyboard + hover tooltips for free.
 */
export function HoverHint({
  label,
  children,
  side = "top",
  delayMs,
  variant,
  className,
}: HoverHintProps) {
  return (
    <Tooltip delayDuration={delayMs}>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side} variant={variant} className={className}>
        {label}
      </TooltipContent>
    </Tooltip>
  );
}
