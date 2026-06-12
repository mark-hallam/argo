"use client";

import * as RAvatar from "@radix-ui/react-avatar";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef } from "react";
import { cn } from "@/lib/utils";

const avatarVariants = cva(
  "relative inline-flex shrink-0 overflow-hidden rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]",
  {
    variants: {
      size: {
        xs: "size-6 text-[10px]",
        sm: "size-8 text-xs",
        md: "size-10 text-sm",
        lg: "size-14 text-base",
      },
    },
    defaultVariants: { size: "md" },
  },
);

const statusRing = cva(
  "absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-[var(--card)]",
  {
    variants: {
      status: {
        idle: "bg-[var(--muted-foreground)]/40",
        working: "bg-[var(--info)]",
        waiting: "bg-[var(--warning)]",
        blocked: "bg-[var(--destructive)]",
        none: "hidden",
      },
    },
    defaultVariants: { status: "none" },
  },
);

export type WorkerStatus = "idle" | "working" | "waiting" | "blocked";

/**
 * Derive 1-2 character initials for the generated-avatar fallback.
 *
 * Multi-word names take the first letter of the first two words
 * ("Ada Lovelace" → "AL"). Single-word names take the first TWO
 * letters ("Ivy" → "IV") rather than a lone "I" — a bare single
 * letter reads as a placeholder, not a designed monogram (F13,
 * 2026-06-13 fresh-hire cosmetics shakedown). Empty/whitespace
 * input degrades to "?".
 */
export function deriveInitials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0]!.slice(0, 2).toUpperCase();
  return (words[0]![0]! + words[1]![0]!).toUpperCase();
}

export interface AvatarProps
  extends Omit<React.ComponentPropsWithoutRef<typeof RAvatar.Root>, "children">,
    VariantProps<typeof avatarVariants> {
  src?: string;
  alt: string;
  fallback?: string;
  status?: WorkerStatus;
}

export const Avatar = forwardRef<HTMLSpanElement, AvatarProps>(
  ({ className, size, src, alt, fallback, status, ...props }, ref) => {
    const initials = fallback ?? deriveInitials(alt);

    return (
      <span className={cn("relative inline-block", className)}>
        <RAvatar.Root className={cn(avatarVariants({ size }))} ref={ref} {...props}>
          {src ? (
            <RAvatar.Image
              src={src}
              alt={alt}
              className="aspect-square h-full w-full object-cover"
            />
          ) : null}
          <RAvatar.Fallback
            delayMs={src ? 200 : 0}
            className="flex h-full w-full items-center justify-center font-medium"
          >
            {initials}
          </RAvatar.Fallback>
        </RAvatar.Root>
        {status ? <span aria-hidden className={statusRing({ status })} /> : null}
      </span>
    );
  },
);
Avatar.displayName = "Avatar";
